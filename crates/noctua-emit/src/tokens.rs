//! What names a palette exposes, and what each one points at.
//!
//! Every target emits the same three layers, so the layering is decided here
//! once rather than seven times:
//!
//! - **Palette tokens** carry actual values. `--nc-accent-solid`.
//! - **Numeric aliases** exist for interop with numbered scales. `--nc-accent-9`
//!   points at `--nc-accent-solid`; the role name stays canonical.
//! - **Semantic tokens** are the contract an application codes against.
//!   `--nc-color-accent` points at whichever family this theme assigned, and
//!   the Tailwind target re-exports it as `--color-accent`.
//!
//! Only palette tokens have values, and only they need to be repeated per
//! mode and per gamut. Aliases and semantics are indirections, identical in
//! every mode, and are emitted once — which is what keeps generated CSS from
//! multiplying out to something nobody can read.

use indexmap::IndexMap;
use noctua_engine::{ResolvedMode, ResolvedStep};

/// A token that carries a real color.
#[derive(Debug, Clone, Copy)]
pub struct PaletteToken<'a> {
    /// Family this step belongs to.
    pub family: &'a str,
    /// Canonical role name.
    pub role: &'a str,
    /// The step, with one rendition per gamut.
    pub step: &'a ResolvedStep,
}

impl PaletteToken<'_> {
    /// `accent-solid`, the stem every target prefixes its own way.
    #[must_use]
    pub fn stem(&self) -> String {
        format!("{}-{}", self.family, self.role)
    }
}

/// A token that points at another token.
#[derive(Debug, Clone)]
pub struct AliasToken {
    /// The alias name, without prefix.
    pub name: String,
    /// The palette-token stem it points at.
    pub target: String,
}

/// Every palette token in a mode, families in order, steps in ramp order.
#[must_use]
pub fn palette_tokens(mode: &ResolvedMode) -> Vec<PaletteToken<'_>> {
    mode.families
        .iter()
        .flat_map(|(family, resolved)| {
            resolved.steps.iter().map(move |step| PaletteToken {
                family,
                role: &step.role,
                step,
            })
        })
        .collect()
}

/// Numeric aliases: `accent-9` for `accent-solid`.
///
/// Numbered scales are how most of the world talks about color ramps, and a
/// system that refuses to answer to a number is a system people work around.
/// The names stay canonical; these are a translation, not a second vocabulary.
#[must_use]
pub fn numeric_aliases(mode: &ResolvedMode) -> Vec<AliasToken> {
    mode.families
        .iter()
        .flat_map(|(family, resolved)| {
            resolved.steps.iter().map(move |step| AliasToken {
                name: format!("{family}-{}", step.index),
                target: format!("{family}-{}", step.role),
            })
        })
        .collect()
}

/// The slot whose tokens are the page itself rather than a context.
const NEUTRAL_SLOT: &str = "neutral";

/// What a neutral slot contributes: `(name template, role)`, with `{}` standing
/// for the slot's variant suffix — empty for `neutral`, `-cool` for
/// `neutral-cool`.
///
/// Qualifiers accumulate rightward, so the cool variant of `fg-muted` is
/// `fg-muted-cool`. Same direction as `accent-hover` and `border-strong`, and
/// it keeps the base names exactly what they have always been.
const SURFACE_SHAPE: [(&str, &str); 7] = [
    ("surface{}", "bg-app"),
    ("surface-subtle{}", "bg-subtle"),
    ("surface-raised{}", "bg-element"),
    ("fg{}", "text-strong"),
    ("fg-muted{}", "text-muted"),
    ("border{}", "border-element"),
    ("border-strong{}", "border-strong"),
];

/// What every other slot contributes: `(name template, role)`, with `{}`
/// standing for the slot name.
///
/// `on-{}` is the family's own lightest step rather than a computed black or
/// white. In light mode that step is near-white and the solid beneath it is
/// mid-toned; in dark mode it is near-black and the solid is light. It lands
/// the right way round in both, carries a trace of the family's hue, and needs
/// no color math in this crate.
const CONTEXT_SHAPE: [(&str, &str); 5] = [
    ("{}", "solid"),
    ("{}-hover", "solid-hover"),
    ("{}-bg", "bg-subtle"),
    ("{}-border", "border-element"),
    ("on-{}", "bg-app"),
];

/// The variant suffix if this slot is a neutral, otherwise `None`.
///
/// Keyed on the **slot**, not the family it resolves to. An alias like
/// `inactive = "neutral"` must not produce a second `surface` and `fg`: it is
/// a context that happens to be gray, and it gets a context's tokens.
fn surface_variant(slot: &str) -> Option<&str> {
    slot.strip_prefix(NEUTRAL_SLOT)
        .filter(|rest| rest.is_empty() || rest.starts_with('-'))
}

/// The semantic contract: the names an application codes against.
///
/// Driven by [`ResolvedMode::semantic`] and two shape tables rather than by a
/// list of names. That is the difference between six contexts and forty — a
/// hand-written list has to be edited for every alias the spec adds, and stops
/// being readable long before it stops being correct.
///
/// Every slot is one of two kinds, decided by its own name: a neutral, which
/// contributes the page's surfaces, text and borders; or a context, which
/// contributes a fill, its hover, a tinted background, a border, and a
/// foreground to put on the fill.
#[must_use]
pub fn semantic_tokens(mode: &ResolvedMode) -> Vec<AliasToken> {
    let mut tokens = Vec::new();
    let mut push = |name: String, family: &str, role: &str| {
        tokens.push(AliasToken {
            name,
            target: format!("{family}-{role}"),
        });
    };

    for (slot, family) in &mode.semantic {
        if let Some(variant) = surface_variant(slot) {
            for (template, role) in SURFACE_SHAPE {
                push(template.replace("{}", variant), family, role);
            }
            continue;
        }

        for (template, role) in CONTEXT_SHAPE {
            push(template.replace("{}", slot), family, role);
        }

        // One global focus ring, not one per context: a page has a single
        // focus style, and it follows the brand.
        if slot == "accent" {
            push("ring".to_owned(), family, "border-strong");
        }
    }

    tokens
}

/// A view of something split into what every theme agrees on and what a theme
/// has to state for itself.
///
/// The semantic layer is an *indirection* — `--nc-color-rejected` points at
/// `--nc-danger-solid`, which is a different colour in every palette but the
/// same sentence in every one of them. Written per theme it was therefore the
/// same three hundred and forty lines repeated thirty-nine times: measured,
/// 97 KB of each 225 KB stylesheet and 58.7 KB of each of the seventy-eight
/// mode blocks in `palette.json`. At two dozen contexts that was a rounding
/// error; at three hundred and fifty it is most of the artifact.
///
/// Derived from the resolved palette rather than from the spec: a slot is
/// shared when every theme resolves it to the same family, whatever the reason.
/// A `[themes.<name>.semantic]` override lands in [`Self::per_theme`]
/// automatically, and so would any future mechanism that moved one.
#[derive(Debug, Clone, Default)]
pub struct ThemeSplit {
    /// Written once. Every theme resolves these identically.
    pub shared: Vec<AliasToken>,
    /// Written per theme, keyed by theme name — only what that theme differs
    /// on. Themes with nothing to add do not appear.
    pub per_theme: IndexMap<String, Vec<AliasToken>>,
}

/// The semantic contract, split by what every theme agrees on.
///
/// See [`ThemeSplit`]. Read from each theme's light mode, because the semantic
/// map does not vary by mode — it is slot to family, and a mode changes what a
/// family's steps *are*, not which family fills a slot.
#[must_use]
pub fn semantic_layer(palette: &noctua_engine::Palette) -> ThemeSplit {
    split_by_theme(palette, |mode| {
        semantic_tokens(mode)
            .into_iter()
            .map(|alias| (alias.name, alias.target))
            .collect()
    })
}

/// Semantic *slot* to family — the contract one level up from the tokens.
///
/// Split the same way and for the same reason. Not derivable from
/// [`semantic_layer`]: a slot contributes five tokens whose names are not the
/// slot's, so a consumer that wants to offer "the contexts" needs this one.
#[must_use]
pub fn slot_layer(palette: &noctua_engine::Palette) -> ThemeSplit {
    split_by_theme(palette, |mode| {
        mode.semantic
            .iter()
            .map(|(slot, family)| (slot.clone(), family.clone()))
            .collect()
    })
}

/// Splits a per-theme name-to-target view into shared and per-theme parts.
///
/// A name is shared when *every* theme has it and every theme points it at the
/// same target. Anything else is written out by each theme that has it, which
/// is what keeps a partial override — a theme that redefines one slot — from
/// silently dropping the other three hundred.
fn split_by_theme(
    palette: &noctua_engine::Palette,
    of: impl Fn(&ResolvedMode) -> Vec<(String, String)>,
) -> ThemeSplit {
    let per_theme: Vec<(&str, IndexMap<String, String>)> = palette
        .themes
        .iter()
        .map(|theme| {
            (
                theme.name.as_str(),
                of(&theme.modes[0]).into_iter().collect(),
            )
        })
        .collect();

    let Some((_, first)) = per_theme.first() else {
        return ThemeSplit::default();
    };

    // Order follows the first theme, then anything a later theme adds — so the
    // output is stable and readable rather than in whatever order a set
    // iterates. Determinism is invariant 4.
    let mut names: Vec<&str> = first.keys().map(String::as_str).collect();
    for (_, tokens) in &per_theme {
        for name in tokens.keys() {
            if !names.contains(&name.as_str()) {
                names.push(name);
            }
        }
    }

    let mut split = ThemeSplit::default();
    for name in names {
        let target = first.get(name);
        let agreed = target.is_some_and(|target| {
            per_theme
                .iter()
                .all(|(_, tokens)| tokens.get(name) == Some(target))
        });

        if agreed {
            split.shared.push(AliasToken {
                name: name.to_owned(),
                target: target.expect("agreed implies present").clone(),
            });
            continue;
        }

        for (theme, tokens) in &per_theme {
            let Some(target) = tokens.get(name) else {
                continue;
            };
            split
                .per_theme
                .entry((*theme).to_owned())
                .or_default()
                .push(AliasToken {
                    name: name.to_owned(),
                    target: target.clone(),
                });
        }
    }

    split
}

/// Every scale entry as `chart-1`, `level-0`, `magnitude-lower`.
///
/// The stem is the step's own label, so a counted scale numbers from zero and a
/// named one uses its names — the scale decides, not this function.
#[must_use]
pub fn scale_names(mode: &ResolvedMode) -> Vec<String> {
    mode.scales
        .iter()
        .flat_map(|(scale, resolved)| {
            resolved
                .steps
                .iter()
                .map(move |step| format!("{scale}-{}", step.role))
        })
        .collect()
}

/// One stop of the translucency ladder.
#[derive(Debug, Clone, Copy)]
pub struct AlphaToken<'a> {
    /// Family this wash comes from.
    pub family: &'a str,
    /// The stem, without prefix: `neutral-a3`.
    pub index: usize,
    /// Opacity, as a percentage.
    pub percentage: f64,
    /// The step being washed, so a hex target can read its channels.
    pub step: &'a ResolvedStep,
}

impl AlphaToken<'_> {
    /// `neutral-a3`, the stem every target prefixes its own way.
    #[must_use]
    pub fn stem(&self) -> String {
        format!("{}-a{}", self.family, self.index)
    }

    /// The palette-token stem this is a wash of: `neutral-text-strong`.
    #[must_use]
    pub fn source(&self, role: &str) -> String {
        format!("{}-{role}", self.family)
    }
}

/// The translucency ladder, for every family that has one.
///
/// Twelve stops of one token each, not a per-mode value: in CSS a stop is a
/// `color-mix` of a token that already follows the mode, and in a hex target it
/// is that mode's channels plus an alpha byte. Either way nothing new is solved
/// here — an alpha token is a *view* of a step that already exists and was
/// already checked.
#[must_use]
pub fn alpha_tokens<'a>(
    palette: &'a noctua_engine::Palette,
    mode: &'a ResolvedMode,
) -> Vec<AlphaToken<'a>> {
    let mut out = Vec::new();
    for family in &palette.alpha.families {
        let Some(resolved) = mode.families.get(family) else {
            continue;
        };
        let Some(step) = resolved
            .steps
            .iter()
            .find(|step| step.role == palette.alpha.role)
        else {
            continue;
        };
        for (i, &percentage) in palette.alpha.percentages.iter().enumerate() {
            out.push(AlphaToken {
                family,
                index: i + 1,
                percentage,
                step,
            });
        }
    }
    out
}

/// Every dense neutral ramp entry as `gray-1`, `gray-cool-1`.
#[must_use]
pub fn ramp_names(palette: &noctua_engine::Palette) -> Vec<String> {
    palette
        .neutral_ramps
        .iter()
        .flat_map(|(ramp, steps)| {
            steps
                .iter()
                .map(move |step| format!("{ramp}-{}", step.index))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use noctua_engine::build;
    use noctua_spec::Spec;

    use super::*;

    fn shipped() -> noctua_engine::Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        let spec: Spec = noctua_spec::load(path).expect("valid spec");
        build(&spec).expect("builds")
    }

    /// Every shipped slot has to survive every target's name mangling and come
    /// out a legal identifier there.
    ///
    /// The audit that found `readonly` and `required` is not the guard; this
    /// is. A slot is a name a person types into the spec, and the failure mode
    /// is not a bad colour — it is a QML singleton that will not parse, taking
    /// every token in it down, or a Rust module that will not compile. Both are
    /// silent until something loads the file.
    #[test]
    fn every_slot_survives_every_target() {
        // Words that are not identifiers in a target this project emits.
        const RUST_KEYWORDS: &[&str] = &[
            "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
            "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait",
            "true", "type", "union", "unsafe", "use", "where", "while", "abstract", "become",
            "box", "do", "final", "macro", "override", "priv", "try", "typeof", "unsized",
            "virtual", "yield", "gen",
        ];

        let palette = shipped();
        let mode = &palette.themes[0].modes[0];
        let names: Vec<String> = semantic_tokens(mode)
            .into_iter()
            .map(|alias| alias.name)
            .collect();
        assert!(names.len() > 1500, "only {} names emitted", names.len());

        for name in &names {
            // QML: a camel-case property name, escaped away from every keyword.
            // Checked on the emitted name rather than on the slot, because
            // `on-{slot}` and `{slot}-bg` are names the singleton declares too.
            let qml = crate::name::qml_property(name);
            assert!(
                !crate::name::is_qml_reserved(&qml),
                "QML property `{qml}` (from `{name}`) is a reserved word, which \
                 does not fail the token — it fails the whole singleton"
            );
            assert!(
                qml.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "QML property `{qml}` is not an identifier"
            );

            // Rust: SCREAMING_SNAKE constants, which no keyword collides with —
            // but only because of the case. The assertion is on that rather
            // than on today's call sites, so moving a name into a module or a
            // function fails here first.
            let konst = crate::name::screaming_snake(name);
            assert!(
                konst
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
                "Rust constant `{konst}` is not screaming snake case, so a keyword \
                 like `{}` could reach it",
                RUST_KEYWORDS[0]
            );
            assert!(
                !RUST_KEYWORDS.contains(&konst.as_str()),
                "Rust constant `{konst}` is a keyword"
            );

            // CSS and SCSS take the name verbatim after a prefix, and both
            // accept anything in this alphabet.
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "`{name}` is not spellable as a custom property"
            );
        }
    }

    #[test]
    fn every_family_contributes_every_role() {
        let palette = shipped();
        let mode = &palette.themes[0].modes[0];
        let tokens = palette_tokens(mode);
        assert_eq!(tokens.len(), mode.families.len() * palette.roles.len());
    }

    #[test]
    fn token_names_are_unique() {
        let palette = shipped();
        for theme in &palette.themes {
            for mode in &theme.modes {
                let mut names: Vec<String> = palette_tokens(mode)
                    .iter()
                    .map(PaletteToken::stem)
                    .collect();
                names.extend(numeric_aliases(mode).into_iter().map(|a| a.name));
                let count = names.len();
                names.sort();
                names.dedup();
                assert_eq!(
                    names.len(),
                    count,
                    "duplicate token names in {}",
                    theme.name
                );
            }
        }
    }

    #[test]
    fn every_alias_points_at_a_token_that_exists() {
        let palette = shipped();
        for theme in &palette.themes {
            for mode in &theme.modes {
                let stems: Vec<String> = palette_tokens(mode)
                    .iter()
                    .map(PaletteToken::stem)
                    .collect();
                for alias in numeric_aliases(mode)
                    .into_iter()
                    .chain(semantic_tokens(mode))
                {
                    assert!(
                        stems.contains(&alias.target),
                        "{} points at {}, which does not exist",
                        alias.name,
                        alias.target
                    );
                }
            }
        }
    }

    #[test]
    fn the_semantic_contract_covers_what_an_application_needs() {
        let palette = shipped();
        let names: Vec<String> = semantic_tokens(&palette.themes[0].modes[0])
            .into_iter()
            .map(|t| t.name)
            .collect();

        for required in [
            "surface",
            "surface-subtle",
            "fg",
            "fg-muted",
            "border",
            "ring",
            "accent",
            "accent-hover",
            "on-accent",
            "danger",
            "danger-bg",
            "success",
            "warning",
            "info",
        ] {
            assert!(
                names.contains(&required.to_owned()),
                "missing --nc-color-{required}"
            );
        }
    }

    /// The contract must be the same in both modes, or an application would
    /// have to know which mode it is in to know which names exist.
    #[test]
    fn the_semantic_contract_does_not_change_between_modes() {
        let palette = shipped();
        for theme in &palette.themes {
            let names = |mode: &ResolvedMode| -> Vec<String> {
                semantic_tokens(mode).into_iter().map(|t| t.name).collect()
            };
            assert_eq!(
                names(&theme.modes[0]),
                names(&theme.modes[1]),
                "{}",
                theme.name
            );
        }
    }
}
