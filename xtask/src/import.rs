//! `cargo xtask import` — fitting an existing palette to spec parameters.
//!
//! Reads colors out of whatever the source happens to be, groups them into
//! ramps, and fits each ramp to a hue and chroma curve. Prints the residuals
//! and a spec fragment.
//!
//! The residuals are the point. A fit that cannot reach a just-noticeable
//! difference is reported as a failure to express, not smoothed over — that is
//! how this stays a test of the model rather than a demonstration of it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::Path;

use noctua_core::map::{from_hex, oklab_to_rgb_unmapped, rgb_to_oklch, to_hex};
use noctua_core::{Gamut, Oklch, Rgb};
use noctua_engine::fit_family;

use crate::ui;

/// Runs the importer against a source.
pub(crate) fn run(
    root: &Path,
    source: &str,
    name: Option<&str>,
    gamut: Gamut,
) -> Result<(), String> {
    let text = read_source(root, source)?;
    // Qt writes #AARRGGBB; CSS writes #RRGGBBAA. Nothing in the digits tells
    // them apart, so the source decides.
    let order = if Path::new(source)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("qml"))
    {
        HexOrder::Argb
    } else {
        HexOrder::Rgba
    };
    let found = extract(&text, gamut, order);

    if found.is_empty() {
        return Err(format!(
            "no colors found in `{source}`\n  \
             the importer reads #rgb, #rrggbb, #rrggbbaa, rgb(), and oklch() literals"
        ));
    }

    ui::heading("import");
    ui::detail(&format!("{} colors from {source}", found.len()));

    let ramps = group(&found);
    if ramps.is_empty() {
        return Err(format!(
            "found {} colors, but no ramp of 3 or more related colors\n  \
             a curve cannot be fitted to isolated values; name them so they share a stem,\n  \
             such as `accent-1`, `accent-2`, `accent-3`",
            found.len()
        ));
    }

    let mut fragment = String::new();
    let mut expressed = 0usize;

    for (stem, colors) in &ramps {
        let oklch: Vec<Oklch> = colors.iter().map(|c| c.color).collect();
        let Some(fit) = fit_family(&oklch, gamut) else {
            continue;
        };

        let label = name.unwrap_or(stem);
        let verdict = if fit.is_imperceptible() {
            expressed += 1;
            ui::ok(&format!(
                "{label}: {} colors, worst {:.4}, mean {:.4} — expressible",
                colors.len(),
                fit.worst(),
                fit.mean()
            ));
            if fit.is_well_constrained() {
                "expressible"
            } else {
                "expressible, weakly constrained"
            }
        } else {
            ui::warn(&format!(
                "{label}: {} colors, worst {:.4}, mean {:.4} — beyond a just-noticeable difference ({:.2})",
                colors.len(),
                fit.worst(),
                fit.mean(),
                noctua_core::JND
            ));
            "not expressible within a just-noticeable difference"
        };

        if !fit.is_well_constrained() {
            ui::detail(&format!(
                "only {} colors against {} parameters — weak evidence either way",
                colors.len(),
                noctua_engine::fit::PARAMETERS
            ));
        }

        // The worst color, named, so the residual points somewhere.
        if let Some((index, residual)) = fit
            .residuals
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            && *residual >= noctua_core::JND
        {
            ui::detail(&format!(
                "worst color: {} ({})",
                colors[index].name.as_deref().unwrap_or("unnamed"),
                to_hex(oklab_to_rgb_unmapped(colors[index].color.to_oklab(), gamut))
            ));
        }

        let _ = write!(
            fragment,
            "# {stem}: {verdict}\n{}\n",
            fit.to_spec_fragment(&sanitize(label))
        );
    }

    println!();
    ui::heading("spec fragment");
    println!("{fragment}");

    ui::detail(&format!(
        "{expressed} of {} ramps expressible within a just-noticeable difference",
        ramps.len()
    ));
    ui::detail("paste the fragment into specs/noctua.toml, then run `cargo xtask build`");

    Ok(())
}

/// A color as it was found, with whatever name and scope the source gave it.
#[derive(Debug, Clone)]
struct Found {
    /// The enclosing selector, for sources that have blocks.
    ///
    /// A stylesheet states the same token once per theme — light, dark,
    /// `.dark`, `[data-theme='moss']`. Those are the same role under
    /// different conditions, not a ramp, and fitting a curve across them
    /// would produce a residual that measures nothing.
    scope: String,
    name: Option<String>,
    color: Oklch,
}

fn read_source(root: &Path, source: &str) -> Result<String, String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        return Err(format!(
            "cannot fetch `{source}`\n  \
             remote import is not built in: this tool has no HTTP client, by design.\n  \
             fetch the stylesheet yourself and import the file:\n    \
             curl -sL {source} > palette.css && cargo xtask import palette.css"
        ));
    }

    // A bare list of colors, rather than a path.
    if source.contains('#') && !Path::new(source).exists() {
        return Ok(source.to_owned());
    }

    let path = if Path::new(source).is_absolute() {
        Path::new(source).to_path_buf()
    } else {
        root.join(source)
    };

    std::fs::read_to_string(&path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))
}

/// Which byte order eight hex digits are in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HexOrder {
    /// `#RRGGBBAA` — CSS Color 4.
    Rgba,
    /// `#AARRGGBB` — Qt, and therefore every QML theme.
    Argb,
}

/// Pulls every color literal out of arbitrary text, with its name when the
/// line looks like an assignment.
fn extract(text: &str, gamut: Gamut, order: HexOrder) -> Vec<Found> {
    let mut found = Vec::new();
    let mut scopes: Vec<String> = Vec::new();

    for line in text.lines() {
        // Skip a line that is only a comment, in any of the three syntaxes
        // this is likely to meet.
        let trimmed = line.trim_start();
        if trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
            || (trimmed.starts_with('#') && !starts_with_hex(trimmed))
        {
            continue;
        }

        let name = name_on(line);
        let scope = scopes.last().cloned().unwrap_or_default();
        for color in colors_in(line, gamut, order) {
            found.push(Found {
                scope: scope.clone(),
                name: name.clone(),
                color,
            });
        }

        track_blocks(line, &mut scopes);
    }

    found
}

/// Maintains the stack of enclosing selectors.
///
/// A media query pushes a scope of its own and the rule inside it pushes
/// another, so the innermost is always the one that owns the declarations.
fn track_blocks(line: &str, scopes: &mut Vec<String>) {
    let mut selector_start = 0usize;
    for (at, c) in line.char_indices() {
        match c {
            '{' => {
                let selector = line[selector_start..at].trim();
                scopes.push(if selector.is_empty() {
                    scopes.last().cloned().unwrap_or_default()
                } else {
                    selector.to_owned()
                });
                selector_start = at + 1;
            }
            '}' => {
                scopes.pop();
                selector_start = at + 1;
            }
            _ => {}
        }
    }
}

/// Whether a leading `#` begins a color rather than a comment.
fn starts_with_hex(text: &str) -> bool {
    let digits = text[1..]
        .chars()
        .take_while(char::is_ascii_hexdigit)
        .count();
    matches!(digits, 3 | 6 | 8)
}

/// The identifier being assigned on this line, if there is one.
///
/// Covers `--accent-9: …` (CSS), `readonly property color accentSolid: …`
/// (QML), `"accent-9": …` (JSON) and `$accent-9: …` (SCSS).
fn name_on(line: &str) -> Option<String> {
    let (left, _) = line.split_once(':')?;
    let identifier = left
        .rsplit([' ', '\t', '"', '\''])
        .find(|part| !part.is_empty())?;
    let cleaned = identifier.trim_start_matches(['-', '$', '@']);

    let usable = !cleaned.is_empty()
        && cleaned
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    usable.then(|| cleaned.to_owned())
}

/// Every color literal on a line, in order.
fn colors_in(line: &str, gamut: Gamut, order: HexOrder) -> Vec<Oklch> {
    let mut colors = Vec::new();
    let mut offset = 0usize;

    while offset < line.len() {
        let rest = &line[offset..];
        if !line.is_char_boundary(offset) {
            offset += 1;
            continue;
        }

        if let Some(after) = rest.strip_prefix('#') {
            let digits: String = after.chars().take_while(char::is_ascii_hexdigit).collect();
            if let Some(rgb) = hex(&digits, order) {
                colors.push(rgb_to_oklch(rgb, gamut));
            }
            offset += 1 + digits.len().max(1);
        } else if let Some(color) = function(rest, "rgb(", from_rgb) {
            colors.push(rgb_to_oklch(color, gamut));
            offset += 4;
        } else if let Some(color) = function(rest, "oklch(", from_oklch) {
            colors.push(color);
            offset += 6;
        } else {
            offset += rest.chars().next().map_or(1, char::len_utf8);
        }
    }

    colors
}

/// Reads `name(a b c)` or `name(a, b, c)` into whatever `parse` builds.
fn function<T>(text: &str, prefix: &str, parse: impl Fn(&[f64]) -> Option<T>) -> Option<T> {
    let inner = text.strip_prefix(prefix)?;
    let end = inner.find(')')?;
    let numbers: Vec<f64> = inner[..end]
        .split([',', ' ', '/'])
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            let part = part.trim();
            part.strip_suffix('%').map_or_else(
                || part.parse().ok(),
                |p| p.parse::<f64>().ok().map(|v| v / 100.0),
            )
        })
        .collect();
    parse(&numbers)
}

fn from_rgb(numbers: &[f64]) -> Option<Rgb> {
    let [r, g, b, ..] = numbers else { return None };
    // A percentage was already divided by 100; a plain number is 0-255.
    let scale = |v: f64| if v <= 1.0 { v } else { v / 255.0 };
    Some(Rgb {
        r: scale(*r),
        g: scale(*g),
        b: scale(*b),
    })
}

fn from_oklch(numbers: &[f64]) -> Option<Oklch> {
    let [l, c, h, ..] = numbers else { return None };
    Some(Oklch {
        l: *l,
        c: *c,
        h: *h,
    })
}

/// Hex digits without the `#`, in the given byte order.
///
/// Three and six digits are unambiguous and go straight to
/// [`noctua_core::map::from_hex`]. Eight are reordered first when the source
/// is Qt, because there the alpha channel comes first.
fn hex(digits: &str, order: HexOrder) -> Option<Rgb> {
    match (digits.len(), order) {
        (8, HexOrder::Argb) => from_hex(&format!("#{}", &digits[2..])),
        _ => from_hex(&format!("#{digits}")),
    }
}

/// Whether `prefix` ends at a segment boundary inside `name`.
///
/// `color-surface` is a segment prefix of `color-surface-raised` but not of
/// `color-surfaces`, and `bg` is one of `bgPopup`. Without the boundary check
/// the second case would swallow the first.
fn is_segment_prefix(prefix: &str, name: &str) -> bool {
    if prefix.len() >= name.len() || !name.starts_with(prefix) {
        return false;
    }
    name[prefix.len()..]
        .chars()
        .next()
        .is_some_and(|c| c == '-' || c == '_' || c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// The family a token name belongs to.
///
/// Real palettes name by role rather than by index — `bgPopup`, `textDim`,
/// `color-surface-raised` — so the family is whichever *shorter token that
/// actually exists* the name extends. `color-surface-raised` belongs to
/// `color-surface` because that token is there; `color-ring` belongs to
/// nothing, because `color` is not a token anyone declared.
///
/// That last part is what keeps unrelated singletons from piling into a
/// plausible-looking bucket and producing a residual that measures nothing.
/// A numeric suffix is the one exception: `chart-1` has no `chart` to point
/// at, and the digits say what it is.
fn stem_within(name: &str, all: &BTreeSet<&str>) -> String {
    let longest = all
        .iter()
        .filter(|candidate| is_segment_prefix(candidate, name))
        .max_by_key(|candidate| candidate.len());

    if let Some(head) = longest {
        return (*head).to_owned();
    }

    let without_digits = name.trim_end_matches(|c: char| c.is_ascii_digit());
    let trimmed = without_digits.trim_end_matches(['-', '_']);
    if trimmed.is_empty() || trimmed == name {
        name.to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Groups colors into ramps by family, ordering each ramp light to dark so
/// the fit runs over a consistent direction.
///
/// Unnamed colors form a single ramp in source order — which is right for a
/// pasted hex list, where the order *is* the ramp.
fn group(found: &[Found]) -> Vec<(String, Vec<Found>)> {
    let mut groups: indexmap::IndexMap<String, Vec<Found>> = indexmap::IndexMap::new();
    // A file with one block — a QML singleton, a single `:root` — gains
    // nothing from having that block named in every label.
    let distinct: BTreeSet<&str> = found.iter().map(|c| c.scope.as_str()).collect();
    let scoped = distinct.len() > 1;

    // Stems are resolved against the names in the same scope. A token that
    // exists only in the dark block must not become the family head for the
    // light one.
    let names_by_scope: BTreeMap<&str, BTreeSet<&str>> =
        found.iter().fold(BTreeMap::new(), |mut map, color| {
            if let Some(name) = color.name.as_deref() {
                map.entry(color.scope.as_str()).or_default().insert(name);
            }
            map
        });
    let empty = BTreeSet::new();

    for color in found {
        let names = names_by_scope.get(color.scope.as_str()).unwrap_or(&empty);
        let stem = color
            .name
            .as_deref()
            .map_or_else(|| "imported".to_owned(), |name| stem_within(name, names));
        let key = if scoped && !color.scope.is_empty() {
            format!("{} in {}", stem, color.scope)
        } else {
            stem
        };
        groups.entry(key).or_default().push(color.clone());
    }

    // Two colors cannot describe a curve, and fitting one would report a
    // perfect residual that means nothing.
    groups.retain(|_, colors| colors.len() >= 3);

    for colors in groups.values_mut() {
        // Named ramps arrive in whatever order the file listed them; sorting
        // by lightness makes `t` mean the same thing for every source.
        colors.sort_by(|a, b| b.color.l.total_cmp(&a.color.l));
    }

    groups.into_iter().collect()
}

/// A TOML key that will not need quoting.
fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let trimmed = cleaned.trim_matches('-').to_lowercase();
    if trimmed.is_empty() {
        "imported".to_owned()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Vec<Found> {
        extract(text, Gamut::Srgb, HexOrder::Rgba)
    }

    #[test]
    fn hex_in_every_length_is_read() {
        let short = hex("f00", HexOrder::Rgba).expect("3 digits");
        let long = hex("ff0000", HexOrder::Rgba).expect("6 digits");
        assert_eq!(short, long, "three digits expand to six"); // allow-literal: parser fixture
        assert!(hex("ff0", HexOrder::Rgba).is_some());
        assert!(
            hex("ff00", HexOrder::Rgba).is_none(),
            "4 digits is not a color"
        );
    }

    /// The bug this prevents is silent and total: every QML color read with
    /// its channels rotated, producing a fit that looks plausible and is
    /// entirely wrong.
    #[test]
    fn eight_digits_respect_the_source_byte_order() {
        // Opaque red, spelled the Qt way and the CSS way. // allow-literal: parser fixture
        let qt = hex("ffff0000", HexOrder::Argb).expect("argb");
        let css = hex("ff0000ff", HexOrder::Rgba).expect("rgba");
        let plain = hex("ff0000", HexOrder::Rgba).expect("rrggbb");

        assert_eq!(qt, plain, "Qt puts alpha first");
        assert_eq!(css, plain, "CSS puts alpha last");
        assert_ne!(
            hex("ffff0000", HexOrder::Rgba),
            Some(plain),
            "reading Qt as CSS must not silently produce the same color"
        );
    }

    #[test]
    fn names_are_read_from_every_syntax_this_will_meet() {
        // One binding per line so the marker stays with its literal whatever
        // the formatter decides to do with the assertions.
        let css = "  --accent-9: #b07a4e;"; // allow-literal: parser fixture
        let scss = "$accent-9: #b07a4e;"; // allow-literal: parser fixture
        let json = "  \"accent-9\": \"#b07a4e\","; // allow-literal: parser fixture
        let qml = "  readonly property color accentSolid: \"#b07a4e\""; // allow-literal: parser fixture
        let bare = "#b07a4e #c08a5e"; // allow-literal: parser fixture

        assert_eq!(name_on(css).as_deref(), Some("accent-9"));
        assert_eq!(name_on(scss).as_deref(), Some("accent-9"));
        assert_eq!(name_on(json).as_deref(), Some("accent-9"));
        assert_eq!(name_on(qml).as_deref(), Some("accentSolid"));
        assert_eq!(name_on(bare), None, "a bare list has no names");
    }

    #[test]
    fn a_qml_theme_groups_into_ramps() {
        // The real shape of the file this has to import. Written a line at a
        // time so each literal can carry its marker.
        let qml = [
            "pragma Singleton",
            "QtObject {",
            "  readonly property color accent1: \"#f6ede4\"", // allow-literal: parser fixture
            "  readonly property color accent2: \"#e0c9b0\"", // allow-literal: parser fixture
            "  readonly property color accent3: \"#b07a4e\"", // allow-literal: parser fixture
            "  readonly property color accent4: \"#7a5334\"", // allow-literal: parser fixture
            "  readonly property color text: \"#ece7e0\"",    // allow-literal: parser fixture
            "}",
        ]
        .join("\n");
        let found = extract(&qml, Gamut::Srgb, HexOrder::Argb);
        assert_eq!(found.len(), 5, "every color literal is found");

        let ramps = group(&found);
        assert_eq!(ramps.len(), 1, "only accent has three or more");
        assert_eq!(ramps[0].0, "accent");
        assert_eq!(ramps[0].1.len(), 4);

        // Sorted light to dark.
        let lightness: Vec<f64> = ramps[0].1.iter().map(|c| c.color.l).collect();
        assert!(lightness.windows(2).all(|w| w[0] >= w[1]), "{lightness:?}");
    }

    #[test]
    fn a_bare_hex_list_is_one_ramp_in_source_order() {
        let found = read("#f6ede4, #e0c9b0, #b07a4e, #7a5334"); // allow-literal: parser fixture
        assert_eq!(found.len(), 4);
        let ramps = group(&found);
        assert_eq!(ramps.len(), 1);
        assert_eq!(ramps[0].0, "imported");
    }

    #[test]
    fn comments_do_not_contribute_colors() {
        let css = [
            "/* the old palette */",
            "// --accent-9: #ff0000;", // allow-literal: parser fixture
            "--accent-1: #f6ede4;",    // allow-literal: parser fixture
            "--accent-2: #e0c9b0;",    // allow-literal: parser fixture
            "--accent-3: #b07a4e;",    // allow-literal: parser fixture
        ]
        .join("\n");
        let found = read(&css);
        assert_eq!(found.len(), 3, "the commented-out color is not imported");
    }

    #[test]
    fn css_color_functions_are_read() {
        let found = read("--a: rgb(176, 122, 78);\n--b: oklch(0.6 0.09 60);\n");
        assert_eq!(found.len(), 2);
        assert!((found[1].color.l - 0.6).abs() < 1e-12);
        assert!((found[1].color.h - 60.0).abs() < 1e-12);
    }

    #[test]
    fn isolated_colors_do_not_become_a_ramp() {
        // Two colors cannot describe a curve, and pretending otherwise would
        // report a perfect fit that means nothing.
        let found = read("--one: #ff0000;\n--two: #00ff00;\n"); // allow-literal: parser fixture
        assert!(group(&found).is_empty());
    }

    #[test]
    fn a_family_is_the_shorter_token_a_name_extends() {
        let names: BTreeSet<&str> = ["bg", "bgPopup", "text", "textDim", "accent", "accentBright"]
            .into_iter()
            .collect();

        assert_eq!(stem_within("bgPopup", &names), "bg");
        assert_eq!(stem_within("textDim", &names), "text");
        assert_eq!(stem_within("accentBright", &names), "accent");
        // A family head belongs to its own family.
        assert_eq!(stem_within("bg", &names), "bg");
    }

    #[test]
    fn the_longest_matching_family_wins() {
        // Both `color` and `color-surface` could claim it; the specific one
        // must, or every token collapses into one bucket.
        let names: BTreeSet<&str> = ["color", "color-surface", "color-surface-raised"]
            .into_iter()
            .collect();
        assert_eq!(stem_within("color-surface-raised", &names), "color-surface");
    }

    /// The rule that stops unrelated singletons from forming a fake ramp.
    #[test]
    fn a_name_extending_nothing_that_exists_stands_alone() {
        let names: BTreeSet<&str> = ["color-ring", "color-scrim", "color-fg"]
            .into_iter()
            .collect();
        // There is no `color` token, so these three do not become a `color` ramp.
        assert_eq!(stem_within("color-ring", &names), "color-ring");
        assert_eq!(stem_within("color-scrim", &names), "color-scrim");
        assert_eq!(stem_within("color-fg", &names), "color-fg");
    }

    #[test]
    fn a_numeric_suffix_names_its_own_family() {
        // `chart-1` has no `chart` token to point at, but the digits say what
        // it is.
        let names: BTreeSet<&str> = ["chart-1", "chart-2", "chart-3"].into_iter().collect();
        for name in &names {
            assert_eq!(stem_within(name, &names), "chart");
        }
    }

    #[test]
    fn a_segment_prefix_stops_at_a_boundary() {
        assert!(is_segment_prefix("bg", "bgPopup"));
        assert!(is_segment_prefix("color-surface", "color-surface-raised"));
        assert!(!is_segment_prefix("color-surface", "color-surfaces"));
        assert!(!is_segment_prefix("bg", "bg"));
    }

    #[test]
    fn a_spec_key_is_always_valid_toml() {
        assert_eq!(sanitize("Accent Solid"), "accent-solid");
        assert_eq!(sanitize("--accent--"), "accent");
        assert_eq!(sanitize("!!!"), "imported");
    }
}
