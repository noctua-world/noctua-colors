//! Turning the accent grid into ordinary themes.
//!
//! `[accents]` and `[saturations]` are two axes; a palette is one point on
//! each. Rather than teach the engine, the eight emitters and the five gates
//! about axes, this expands the grid into the `[themes.*]` they already
//! understand — so everything downstream keeps seeing a plain list of themes
//! and none of it had to change.
//!
//! # Why a grid rather than thirty-six blocks
//!
//! Twelve accents and three saturations is fifteen lines of specification and
//! thirty-six palettes. Written out longhand it would be some two hundred and
//! fifty lines of near-identical TOML, and the two axes — the thing a reader
//! actually wants to see, and the thing the documentation site needs in order
//! to offer two controls — would be implicit in a naming convention.
//!
//! # What a generated theme is
//!
//! Exactly what a hand-written one would have been: a chroma multiplier plus a
//! `FamilyOverride` on `accent`. Nothing here can express a theme that could
//! not have been typed out, which is what keeps this a shorthand rather than a
//! second, parallel model.

use indexmap::IndexMap;

use crate::model::{FamilyOverride, Spec, Theme};

/// The family an accent replaces.
const ACCENT_FAMILY: &str = "accent";

/// Expands `[accents]` × `[saturations]` into themes, in place.
///
/// Generated themes come first, in accent-major order, so the first accent and
/// the first saturation form the default palette. Hand-written `[themes.*]`
/// follow and are never overwritten — if a name collides, the hand-written one
/// wins, and [`crate::validate`] reports it rather than letting the grid
/// silently shadow it.
///
/// Does nothing when either axis is empty, so a spec that names no accents
/// behaves exactly as it did before this existed.
pub(crate) fn accent_grid(spec: &mut Spec) {
    if spec.accents.is_empty() || spec.saturations.is_empty() {
        return;
    }

    let mut generated: IndexMap<String, Theme> = IndexMap::new();

    for (accent_name, accent) in &spec.accents {
        for (saturation_name, multiplier) in &spec.saturations {
            let name = format!("{accent_name}-{saturation_name}");
            if spec.themes.contains_key(&name) {
                continue;
            }

            let mut families = IndexMap::new();
            families.insert(
                ACCENT_FAMILY.to_owned(),
                FamilyOverride {
                    hue: Some(accent.hue.clone()),
                    cr: accent.cr.clone(),
                    chroma: None,
                    // Empty means "no correction", which is a real answer for
                    // most hues — not the same as "inherit whatever the base
                    // family had", which would apply a blue's correction to a
                    // green.
                    hue_correction: Some(accent.hue_correction.clone()),
                },
            );

            generated.insert(
                name,
                Theme {
                    chroma: *multiplier,
                    accent: Some(accent_name.clone()),
                    saturation: Some(saturation_name.clone()),
                    // Left empty: `resolve_semantic` already maps every slot
                    // to the family of the same name, which is what these
                    // themes want.
                    semantic: IndexMap::new(),
                    families,
                },
            );
        }
    }

    // Generated first, so the grid's first cell is the default theme the CSS
    // binds to `:root`.
    generated.extend(std::mem::take(&mut spec.themes));
    spec.themes = generated;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn expanded(toml: &str) -> Spec {
        parse("test.toml", toml).expect("valid spec")
    }

    const GRID: &str = r"
[families.accent]
hue = 59.3

[accents]
ochre = { hue = { base = 59.3, torsion = -7.0 } }
blue  = { hue = { base = 250.0 }, hue_correction = [[0.5, 6.0]] }

[saturations]
balanced = 0.82
vivid = 1.15
";

    #[test]
    fn the_grid_is_the_product_of_both_axes() {
        let spec = expanded(GRID);
        let names: Vec<&str> = spec.themes.keys().map(String::as_str).collect();
        assert_eq!(
            names,
            [
                "ochre-balanced",
                "ochre-vivid",
                "blue-balanced",
                "blue-vivid"
            ],
            "accent-major order, so the first accent's first saturation is default"
        );
    }

    #[test]
    fn each_cell_carries_its_saturation_and_its_accent_hue() {
        let spec = expanded(GRID);

        let vivid = &spec.themes["blue-vivid"];
        assert!((vivid.chroma - 1.15).abs() < f64::EPSILON);
        assert_eq!(vivid.accent.as_deref(), Some("blue"));
        assert_eq!(vivid.saturation.as_deref(), Some("vivid"));

        let hue = vivid.families["accent"].hue.as_ref().expect("a hue");
        assert!((hue.base() - 250.0).abs() < f64::EPSILON);
    }

    /// A correction measured for a blue must not follow a green around.
    #[test]
    fn an_accent_without_a_correction_gets_none_rather_than_inheriting_one() {
        let spec = expanded(GRID);

        let blue = spec.themes["blue-balanced"].families["accent"]
            .hue_correction
            .as_ref()
            .expect("blue names a correction");
        assert_eq!(blue.len(), 1);

        let ochre = spec.themes["ochre-balanced"].families["accent"]
            .hue_correction
            .as_ref()
            .expect("the override is always set, even when empty");
        assert!(ochre.is_empty(), "no correction, not the base family's");
    }

    #[test]
    fn a_spec_with_no_accents_is_untouched() {
        let spec = expanded(
            r"
[families.accent]
hue = 59.3

[themes.plain]
chroma = 0.9
",
        );
        let names: Vec<&str> = spec.themes.keys().map(String::as_str).collect();
        assert_eq!(names, ["plain"]);
        assert!(spec.themes["plain"].accent.is_none());
    }

    /// One axis alone generates nothing, which is almost never what someone
    /// writing it meant — so it is reported rather than silently obeyed.
    #[test]
    fn one_axis_alone_is_reported_rather_than_silently_doing_nothing() {
        let error = parse(
            "test.toml",
            r"
[families.accent]
hue = 59.3

[accents]
ochre = { hue = 59.3 }
",
        )
        .expect_err("accents without saturations generate no palettes");

        let message = error.problems()[0].message();
        assert!(message.contains("saturations"), "{message}");
    }

    #[test]
    fn hand_written_themes_follow_the_grid_and_are_never_overwritten() {
        let spec = expanded(&format!("{GRID}\n[themes.custom]\nchroma = 0.5\n"));
        let names: Vec<&str> = spec.themes.keys().map(String::as_str).collect();
        assert_eq!(names.last(), Some(&"custom"));
        assert!((spec.themes["custom"].chroma - 0.5).abs() < f64::EPSILON);
    }
}
