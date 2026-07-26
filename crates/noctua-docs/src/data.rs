//! The palette, as the site sees it.
//!
//! Read from `dist/json/palette.json` — the same artifact any other consumer
//! gets, parsed with no privileged access to the engine.
//!
//! That constraint is the point. A docs site that imported `noctua-engine` and
//! recomputed colors would be a second implementation that happens to agree,
//! and the day it stopped agreeing nobody would know. Reading the emitted file
//! means the site is wrong exactly when the output is wrong, which is the only
//! useful place for it to be wrong.

use indexmap::IndexMap;
use serde::Deserialize;

/// The whole system, as emitted.
#[derive(Debug, Clone, Deserialize)]
pub struct Palette {
    /// Custom-property namespace, such as `nc`.
    pub prefix: String,
    /// Gamuts emitted, primary first.
    pub gamuts: Vec<String>,
    /// Scale role names, in ramp order.
    pub roles: Vec<String>,
    /// The dense neutral ramps, keyed by stem — `gray`, `gray-cool`,
    /// `gray-warm` — and shared by both modes.
    #[serde(rename = "grayRamps", default)]
    pub gray_ramps: IndexMap<String, Vec<Step>>,
    /// The two axes palettes are chosen on.
    #[serde(default)]
    pub axes: Axes,

    /// Themes, keyed by name, **in the order the spec declared them**.
    ///
    /// Order is load-bearing: the CSS emitter binds the first theme to
    /// `:root` and scopes the rest under `[data-palette]`, so the first entry
    /// here has to be the same theme the stylesheet is painting. A sorted map
    /// looked correct for as long as the default theme also sorted first, and
    /// would have silently marked the wrong theme as selected the moment one
    /// sorted ahead of it.
    pub themes: IndexMap<String, IndexMap<String, ModePalette>>,
}

impl Palette {
    /// The untinted dense ramp, for the places that mean specifically that one.
    #[must_use]
    pub fn gray_ramp(&self) -> &[Step] {
        self.gray_ramps.get("gray").map_or(&[], Vec::as_slice)
    }
}

/// The accent and saturation axes.
///
/// Empty for a spec that writes its themes out by hand, in which case the site
/// falls back to offering them as one flat list.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Axes {
    /// Accent names, in spec order.
    #[serde(default)]
    pub accents: Vec<String>,
    /// Saturation names, in spec order.
    #[serde(default)]
    pub saturations: Vec<String>,
    /// `"{accent}/{saturation}"` to the theme it resolves to.
    #[serde(default)]
    pub themes: IndexMap<String, String>,
}

impl Axes {
    /// Whether both axes are populated, so two controls make sense.
    #[must_use]
    pub fn is_grid(&self) -> bool {
        !self.accents.is_empty() && !self.saturations.is_empty()
    }

    /// The theme a pair resolves to.
    #[must_use]
    pub fn theme(&self, accent: &str, saturation: &str) -> Option<&str> {
        self.themes
            .get(&format!("{accent}/{saturation}"))
            .map(String::as_str)
    }
}

/// One theme in one mode.
#[derive(Debug, Clone, Deserialize)]
pub struct ModePalette {
    /// Families, keyed by name, in spec order.
    pub families: IndexMap<String, Family>,
    /// Emitted token name to the `family-role` stem it points at.
    pub semantic: IndexMap<String, String>,
    /// Semantic *slot* to the family that fills it, in spec order.
    ///
    /// Not the same map as [`Self::semantic`], and not derivable from it: a slot
    /// contributes five tokens whose names are not the slot's, so the token map
    /// cannot say which contexts exist.
    #[serde(default)]
    pub slots: IndexMap<String, String>,
    /// Scales, keyed by stem, with the categorical `chart` first.
    pub scales: IndexMap<String, Vec<Step>>,
    /// The translucency ladder, keyed by stem — `neutral-a1`.
    #[serde(default)]
    pub alpha: IndexMap<String, AlphaStop>,
    /// Every gated pair, measured.
    #[serde(default)]
    pub contrast: Vec<ContrastRow>,
}

/// One stop of the translucency ladder.
#[derive(Debug, Clone, Deserialize)]
pub struct AlphaStop {
    /// The palette-token stem this is a wash of.
    #[serde(rename = "of")]
    pub of: String,
    /// Opacity, as a percentage.
    pub percentage: f64,
    /// The stop as `#rrggbbaa`.
    pub hex: String,
}

/// One measured contrast pair, as the gate measured it.
///
/// Read from the artifact rather than measured here, for the same reason the
/// rest of this module is: a site that did its own colour math would be a second
/// implementation that happens to agree.
#[derive(Debug, Clone, Deserialize)]
pub struct ContrastRow {
    /// Semantic name of the foreground.
    pub fg: String,
    /// Semantic name of the background.
    pub bg: String,
    /// APCA contrast, as a magnitude.
    pub lc: f64,
    /// WCAG 2.1 ratio, for compliance reporting only.
    pub wcag: f64,
    /// The threshold this pair is held to.
    pub minimum: f64,
    /// `fail` or `warn`.
    pub severity: String,
    /// Whether the pair clears its threshold.
    pub passes: bool,
}

/// One family's ramp.
#[derive(Debug, Clone, Deserialize)]
pub struct Family {
    /// Nominal hue in degrees.
    #[serde(rename = "baseHue")]
    pub base_hue: f64,
    /// Steps, in ramp order.
    pub steps: Vec<Step>,
}

/// One step of a ramp.
#[derive(Debug, Clone, Deserialize)]
pub struct Step {
    /// Canonical role name, or the index for ramps without roles.
    pub role: String,
    /// Position in the ramp, from one.
    pub index: usize,
    /// One entry per gamut, primary first.
    pub renditions: Vec<Color>,
}

impl Step {
    /// The rendition in the primary gamut.
    #[must_use]
    pub fn primary(&self) -> &Color {
        &self.renditions[0]
    }
}

/// One step in one gamut.
#[derive(Debug, Clone, Deserialize)]
pub struct Color {
    /// The gamut this rendition resolved against.
    pub gamut: String,
    /// Channel values as `#rrggbb`.
    pub hex: String,
    /// OKLCH coordinates.
    pub oklch: Oklch,
    /// Ready to drop into CSS.
    pub css: String,
    /// Fraction of the gamut boundary reached.
    #[serde(rename = "relativeChroma")]
    pub relative_chroma: f64,
    /// Fraction the spec asked for.
    #[serde(rename = "requestedRelativeChroma")]
    pub requested_relative_chroma: f64,
    /// Chroma remaining between this color and the boundary.
    #[serde(rename = "chromaHeadroom")]
    pub chroma_headroom: f64,
}

/// OKLCH coordinates.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Oklch {
    /// Perceptual lightness, 0 to 1.
    pub l: f64,
    /// Chroma.
    pub c: f64,
    /// Hue in degrees.
    pub h: f64,
}

impl Palette {
    /// Parses the emitted `palette.json`.
    ///
    /// # Errors
    ///
    /// Malformed or unreadable JSON.
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json)
            .map_err(|error| format!("dist/json/palette.json is not readable: {error}"))
    }

    /// Theme names, in a stable order.
    #[must_use]
    pub fn theme_names(&self) -> Vec<&str> {
        self.themes.keys().map(String::as_str).collect()
    }

    /// The default theme, which is the one bound to `:root`.
    #[must_use]
    pub fn default_theme(&self) -> &str {
        // The emitter binds the first theme in spec order to `:root`, and both
        // the JSON and this map preserve that order.
        self.theme_names().first().copied().unwrap_or("default")
    }

    /// One theme and mode, if it exists.
    #[must_use]
    pub fn mode(&self, theme: &str, mode: &str) -> Option<&ModePalette> {
        self.themes.get(theme)?.get(mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../dist/json/palette.json");
        let json = std::fs::read_to_string(path).expect("dist/json/palette.json — run xtask build");
        Palette::parse(&json).expect("valid palette")
    }

    #[test]
    fn the_emitted_palette_parses() {
        let palette = shipped();
        assert_eq!(palette.prefix, "nc");
        assert!(palette.gamuts.len() >= 2);
        assert_eq!(palette.roles.len(), 12);
        assert_eq!(palette.gray_ramp().len(), 24);

        // Every ramp is the same lightnesses at a different temperature, so a
        // consumer can swap one for another without moving any contrast. That
        // claim is only true if they are the same length.
        for (name, steps) in &palette.gray_ramps {
            assert_eq!(steps.len(), 24, "{name} has a different length");
        }
        assert!(palette.themes.len() >= 3);
    }

    /// The first theme here must be the one the stylesheet binds to `:root`.
    ///
    /// A sorted map passed this for as long as the default theme also sorted
    /// first. Reading the emitted CSS rather than trusting either map is what
    /// makes the check mean something.
    #[test]
    fn the_first_theme_is_the_one_the_stylesheet_paints() {
        let palette = shipped();
        let css = concat!(env!("CARGO_MANIFEST_DIR"), "/../../dist/css/index.css");
        let index = std::fs::read_to_string(css).expect("dist/css/index.css");

        // `index.css` imports the default theme's bare file name first, after
        // the shared ramp.
        let default_import = index
            .lines()
            .filter(|line| line.starts_with("@import"))
            .find(|line| !line.contains("ramp.css"))
            .expect("a theme import");

        assert!(
            default_import.contains(&format!("{}.css", palette.default_theme())),
            "the site calls `{}` the default, but the stylesheet imports {default_import}",
            palette.default_theme()
        );
    }

    /// Spec order, not alphabetical order.
    #[test]
    fn themes_keep_the_order_they_were_declared_in() {
        let palette = shipped();
        let names = palette.theme_names();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        // If these ever coincide the test is vacuous, so say so rather than
        // passing quietly.
        assert!(
            names != sorted || names.len() < 2,
            "every theme name sorts into declaration order, so this proves nothing — \
             add a theme whose name sorts before the default"
        );
    }

    #[test]
    fn every_theme_has_both_modes_with_the_same_families() {
        let palette = shipped();
        for theme in palette.theme_names() {
            let light = palette.mode(theme, "light").expect("light");
            let dark = palette.mode(theme, "dark").expect("dark");
            let names = |m: &ModePalette| m.families.keys().cloned().collect::<Vec<_>>();
            assert_eq!(names(light), names(dark), "{theme}");
        }
    }

    #[test]
    fn every_step_carries_the_metadata_the_site_renders() {
        let palette = shipped();
        let mode = palette
            .mode(palette.default_theme(), "light")
            .expect("light");
        for family in mode.families.values() {
            for step in &family.steps {
                let color = step.primary();
                assert!(color.hex.starts_with('#'));
                assert!(color.css.starts_with("oklch("));
                assert!((0.0..=1.0).contains(&color.oklch.l));
                assert!(color.chroma_headroom >= 0.0);
            }
        }
    }

    #[test]
    fn semantic_names_resolve_to_real_steps() {
        let palette = shipped();
        let mode = palette
            .mode(palette.default_theme(), "light")
            .expect("light");
        for (name, stem) in &mode.semantic {
            let resolved = mode.families.values().any(|family| {
                family.steps.iter().any(|step| {
                    mode.families
                        .iter()
                        .any(|(fname, _)| *stem == format!("{fname}-{}", step.role))
                })
            });
            assert!(resolved, "{name} points at {stem}, which is not a step");
        }
    }
}
