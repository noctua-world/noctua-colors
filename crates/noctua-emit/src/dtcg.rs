//! Design Tokens Community Group JSON, conformant to **Format Module 2025.10**.
//!
//! That version is the first stable one, and it made a colour `$value` an
//! **object** with required `colorSpace` and `components`. A plain hex string —
//! which this target emitted while the spec was a draft, on the argument that it
//! was what tools actually read — is no longer valid, and Style Dictionary v5
//! parses 2025.10 by default. So the shape changed here before the package was
//! ever published: altering token shape afterwards is a breaking change for
//! every downstream pipeline.
//!
//! The change is a straight improvement rather than a compliance tax. The spec
//! lists `oklch` among its colour spaces, so the coordinates a colour was
//! actually *defined* by are now the primary, lossless value, and `hex` rides
//! along as the six-digit sRGB fallback the spec provides for exactly this. What
//! used to be demoted into `$extensions` is now the value itself.
//!
//! One consequence worth knowing: the spec requires `hex` to be **six** digits,
//! with opacity in a separate `alpha` property. So the translucency ladder no
//! longer carries an eight-digit `#rrggbbaa` here — a conformant consumer reads
//! `alpha` and composites. The eight-digit forms still exist where they are the
//! native spelling: `color-mix()` in CSS, and ARGB in QML.

use noctua_engine::{Palette, ResolvedStep};
use serde_json::{Map, Value, json};

use crate::{CommentStyle, EmittedFile, Emitter, header, value};

/// The colour space every emitted `$value` declares.
///
/// The palette is authored in OKLCH and solved in it, so this is the
/// representation that loses nothing. `hex` is the fallback, not the source.
const COLOR_SPACE: &str = "oklch";

/// The DTCG token target.
#[derive(Debug, Clone, Copy)]
pub struct Dtcg;

impl Emitter for Dtcg {
    fn id(&self) -> &'static str {
        "dtcg"
    }

    fn describe(&self) -> &'static str {
        "DTCG 2025.10 JSON tokens, consumable by Style Dictionary unchanged"
    }

    fn emit(&self, palette: &Palette) -> Vec<EmittedFile> {
        let mut files = Vec::new();

        for theme in &palette.themes {
            for mode in &theme.modes {
                let mut root = Map::new();
                root.insert(
                    "$description".into(),
                    description(&theme.name, mode.mode.id()),
                );

                for (family, resolved) in &mode.families {
                    let mut group = Map::new();
                    group.insert("$type".into(), "color".into());
                    for step in &resolved.steps {
                        group.insert(step.role.clone(), token(step));
                    }
                    root.insert(family.clone(), Value::Object(group));
                }

                for (scale, resolved) in &mode.scales {
                    let mut group = Map::new();
                    group.insert("$type".into(), "color".into());
                    for step in &resolved.steps {
                        group.insert(step.role.clone(), token(step));
                    }
                    root.insert(scale.clone(), Value::Object(group));
                }

                let mut alpha = Map::new();
                alpha.insert("$type".into(), "color".into());
                for stop in crate::tokens::alpha_tokens(palette, mode) {
                    alpha.insert(stop.stem(), alpha_token(stop.step, stop.percentage));
                }
                root.insert("alpha".into(), Value::Object(alpha));

                for (ramp, steps) in &palette.neutral_ramps {
                    let mut group = Map::new();
                    group.insert("$type".into(), "color".into());
                    for step in steps {
                        group.insert(step.index.to_string(), token(step));
                    }
                    root.insert(ramp.clone(), Value::Object(group));
                }

                files.push(EmittedFile::new(
                    format!("tokens/{}-{}.json", theme.name, mode.mode.id()),
                    render(&Value::Object(root)),
                ));
            }
        }

        files
    }
}

/// JSON has no comments, so the generated-file warning goes in a field the
/// format already defines for prose.
fn description(theme: &str, mode: &str) -> Value {
    let banner = header("specs/noctua.toml", CommentStyle::Line(""));
    let banner = banner.replace('\n', " ").trim().to_owned();
    json!(format!("Theme {theme}, {mode} mode. {banner}"))
}

/// An opaque colour token.
///
/// `components` is `[L, C, H]`, the order the spec fixes for `oklch`: lightness
/// in 0 to 1, chroma from 0 up, hue in 0 to 360.
fn token(step: &ResolvedStep) -> Value {
    let color = step.primary();
    json!({
        "$type": "color",
        "$value": {
            "colorSpace": COLOR_SPACE,
            "components": components(step),
            "hex": value::hex(color),
        },
        "$extensions": {
            // Relative chroma has no expression in the spec, and it is the one
            // number that explains *why* a step looks the way it does — how much
            // of the gamut's room at that lightness and hue the colour takes.
            "colors.noctua.relativeChroma": round(color.achieved_relative_chroma, 4),
        }
    })
}

/// One stop of the translucency ladder.
///
/// `alpha` is the spec's own opacity property, 0 to 1, and `hex` stays six
/// digits because the spec requires that. Compositing is the consumer's job,
/// which is the honest division: an alpha token has no colour until it lands on
/// a backdrop.
fn alpha_token(step: &ResolvedStep, percentage: f64) -> Value {
    let color = step.primary();
    json!({
        "$type": "color",
        "$value": {
            "colorSpace": COLOR_SPACE,
            "components": components(step),
            "alpha": round(percentage / 100.0, 4),
            "hex": value::hex(color),
        }
    })
}

/// The OKLCH triple, rounded the way every other target rounds it so the same
/// colour reads identically across `system/`.
fn components(step: &ResolvedStep) -> Value {
    let oklch = step.primary().oklch;
    json!([round(oklch.l, 4), round(oklch.c, 4), round(oklch.h, 2)])
}

fn round(value: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (value * scale).round() / scale
}

fn render(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("tokens always serialize");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use noctua_engine::build;

    use super::*;

    fn shipped() -> Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        build(&noctua_spec::load(path).expect("valid")).expect("builds")
    }

    fn parsed(name: &str) -> Value {
        let contents = Dtcg
            .emit(&shipped())
            .into_iter()
            .find(|f| f.path == name)
            .unwrap_or_else(|| panic!("{name} should be emitted"))
            .contents;
        serde_json::from_str(&contents).expect("valid JSON")
    }

    /// Walks every colour token in a file, so a conformance rule is checked
    /// against all of them rather than against whichever one a test picked.
    fn every_value(tokens: &Value, mut visit: impl FnMut(&str, &Value)) {
        fn walk(node: &Value, path: &str, visit: &mut impl FnMut(&str, &Value)) {
            let Some(object) = node.as_object() else {
                return;
            };
            if let Some(value) = object.get("$value") {
                visit(path, value);
                return;
            }
            for (key, child) in object {
                if key.starts_with('$') {
                    continue;
                }
                walk(child, &format!("{path}/{key}"), visit);
            }
        }
        walk(tokens, "", &mut visit);
    }

    #[test]
    fn one_file_per_theme_and_mode() {
        let palette = shipped();
        let files = Dtcg.emit(&palette);
        assert_eq!(files.len(), palette.themes.len() * 2);
        assert!(
            files
                .iter()
                .any(|f| f.path == "tokens/ochre-balanced-light.json")
        );
        assert!(
            files
                .iter()
                .any(|f| f.path == "tokens/ochre-sober-dark.json")
        );
    }

    #[test]
    fn every_file_is_valid_json() {
        for file in Dtcg.emit(&shipped()) {
            serde_json::from_str::<Value>(&file.contents)
                .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", file.path));
        }
    }

    /// The requirement that replaced "a plain hex string is what tools read":
    /// Format Module 2025.10 makes a colour `$value` an object with **required**
    /// `colorSpace` and `components`. A bare string is not a valid value, so a
    /// regression here is a conformance failure rather than a style difference.
    ///
    /// Checked over every token in the file, and the component ranges are
    /// checked too, because a value inside the right shape but outside the
    /// declared range is just as unusable.
    #[test]
    fn every_value_is_a_conformant_oklch_object() {
        let palette = shipped();
        let mode = &palette.themes[0].modes[0];
        // Derived from the palette rather than written as a number, so the
        // walker and the emitter have to agree on how many tokens exist. A
        // literal here would pass just as happily against a file that had
        // silently stopped emitting a whole group.
        let expected: usize = mode.families.values().map(|f| f.steps.len()).sum::<usize>()
            + mode.scales.values().map(|s| s.steps.len()).sum::<usize>()
            + crate::tokens::alpha_tokens(&palette, mode).len()
            + palette
                .neutral_ramps
                .iter()
                .map(|(_, steps)| steps.len())
                .sum::<usize>();

        let tokens = parsed("tokens/ochre-balanced-light.json");
        let mut seen = 0usize;
        every_value(&tokens, |path, value| {
            seen += 1;
            let object = value
                .as_object()
                .unwrap_or_else(|| panic!("{path}: $value must be an object, got {value}"));
            assert_eq!(object["colorSpace"], COLOR_SPACE, "{path}");

            let components = object["components"]
                .as_array()
                .unwrap_or_else(|| panic!("{path}: components must be an array"));
            assert_eq!(components.len(), 3, "{path}: oklch takes [L, C, H]");
            let (l, c, h) = (
                components[0].as_f64().expect("L is a number"),
                components[1].as_f64().expect("C is a number"),
                components[2].as_f64().expect("H is a number"),
            );
            assert!((0.0..=1.0).contains(&l), "{path}: L {l} outside 0..=1");
            assert!(c >= 0.0, "{path}: C {c} is negative");
            assert!((0.0..360.0).contains(&h), "{path}: H {h} outside 0..360");
        });
        assert_eq!(
            seen, expected,
            "the walker found {seen} tokens and the palette says {expected}"
        );
    }

    /// The spec is explicit that the fallback "MUST be formatted in 6 digit CSS
    /// hex color notation", which is why the translucency ladder moved its
    /// opacity into `alpha` instead of an eighth and ninth digit.
    #[test]
    fn the_hex_fallback_is_always_six_digits() {
        let tokens = parsed("tokens/ochre-balanced-dark.json");
        every_value(&tokens, |path, value| {
            let hex = value["hex"]
                .as_str()
                .unwrap_or_else(|| panic!("{path}: hex"));
            assert_eq!(hex.len(), 7, "{path}: {hex} is not #rrggbb");
            assert!(hex.starts_with('#'), "{path}: {hex}");
            assert!(
                hex[1..].chars().all(|c| c.is_ascii_hexdigit()),
                "{path}: {hex}"
            );
        });
    }

    #[test]
    fn translucent_tokens_carry_alpha_and_opaque_ones_do_not() {
        let tokens = parsed("tokens/ochre-balanced-light.json");

        let wash = &tokens["alpha"];
        let stop = wash
            .as_object()
            .expect("the alpha group")
            .iter()
            .find(|(key, _)| !key.starts_with('$'))
            .map(|(_, value)| value)
            .expect("at least one stop");
        let alpha = stop["$value"]["alpha"].as_f64().expect("an alpha number");
        assert!(
            (0.0..=1.0).contains(&alpha),
            "alpha {alpha} outside 0..=1 — the spec's range, not a percentage"
        );

        assert!(
            tokens["accent"]["solid"]["$value"].get("alpha").is_none(),
            "an opaque token must not declare alpha"
        );
    }

    /// Relative chroma is the one thing the spec cannot express, so it is the
    /// one thing left in `$extensions`. The OKLCH coordinates used to live here
    /// and are now the value itself; keeping a copy would be two sources for one
    /// number.
    #[test]
    fn only_the_unexpressible_rides_in_extensions() {
        let tokens = parsed("tokens/ochre-balanced-light.json");
        let extensions = &tokens["accent"]["solid"]["$extensions"];
        assert!(extensions["colors.noctua.relativeChroma"].is_number());
        assert!(
            extensions.get("colors.noctua.oklch").is_none(),
            "oklch is the $value now; a second copy can only disagree with it"
        );
    }

    #[test]
    fn the_generated_warning_survives_a_format_without_comments() {
        let tokens = parsed("tokens/ochre-balanced-light.json");
        let description = tokens["$description"].as_str().expect("a description");
        assert!(description.contains("do not edit"), "{description}");
        assert!(description.contains(crate::REGENERATE), "{description}");
    }

    #[test]
    fn groups_cover_families_the_chart_and_the_ramp() {
        let tokens = parsed("tokens/ochre-balanced-dark.json");
        assert!(tokens["accent"]["solid"].is_object());
        assert!(tokens["neutral"]["bg-app"].is_object());
        assert!(tokens["chart"]["1"].is_object());
        assert!(tokens["gray"]["24"].is_object());
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(Dtcg.emit(&shipped()), Dtcg.emit(&shipped()));
    }
}
