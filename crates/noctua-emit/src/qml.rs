//! QML singletons, for Quickshell and other Qt consumers.
//!
//! This target exists because the two projects closest to this one —
//! `noctua-hub` and `noctua-shell` — are Quickshell, and could not consume a
//! single one of the web-facing targets. They each keep a hand-written
//! `Theme.qml`; between them, five of thirteen shared tokens had already
//! drifted apart, and neither had a `success` or `warning` color at all.
//!
//! # Two Qt constraints shape the output
//!
//! Qt's `color` type parses `#RRGGBB` and `#AARRGGBB`. It does **not** parse
//! `oklch()`, so this target is hex-only and there is no wide-gamut layer.
//!
//! Note also that the eight-digit form is **ARGB**, not RGBA — alpha leads. The
//! translucency ladder here is written in that ordering by
//! [`crate::value::hex_argb`], which exists as a separate function from the
//! web's `hex_rgba` for exactly this reason: the two produce the same eight
//! digits in a different order, so getting it wrong yields a plausible colour
//! rather than an error, and it is the kind of mistake that survives review.

use std::fmt::Write as _;

use noctua_engine::Palette;

use crate::{CommentStyle, EmittedFile, Emitter, header, name, tokens, value};

/// The QML singleton target.
#[derive(Debug, Clone, Copy)]
pub struct Qml;

impl Emitter for Qml {
    fn id(&self) -> &'static str {
        "qml"
    }

    fn describe(&self) -> &'static str {
        "QML singletons for Quickshell and other Qt consumers, hex only"
    }

    fn emit(&self, palette: &Palette) -> Vec<EmittedFile> {
        let mut files = Vec::new();
        let mut registered: Vec<String> = Vec::new();

        for theme in &palette.themes {
            for mode in &theme.modes {
                let type_name = format!(
                    "{}{}",
                    name::pascal(&name::identifier(&theme.name)),
                    name::pascal(mode.mode.id())
                );

                let mut out = header("specs/noctua.toml", CommentStyle::Line("//"));
                writeln!(
                    out,
                    "//\n\
                     // Theme `{}`, {} mode. Import and use as a singleton:\n\
                     //\n\
                     //     import \".\"\n\
                     //     Rectangle {{ color: {type_name}.surface }}\n\
                     //\n\
                     // Qt colors are hex: `color` does not parse oklch(), so there is\n\
                     // no wide-gamut layer here. The eight-digit form is ARGB.\n",
                    theme.name,
                    mode.mode.id()
                )
                .unwrap();

                writeln!(out, "pragma Singleton\n").unwrap();
                writeln!(out, "import QtQuick\n").unwrap();
                writeln!(out, "QtObject {{").unwrap();

                semantic(&mut out, mode);
                families(&mut out, mode);
                scales(&mut out, mode);
                translucency(&mut out, palette, mode);
                ramps(&mut out, palette);

                writeln!(out, "}}").unwrap();

                files.push(EmittedFile::new(format!("qml/{type_name}.qml"), out));
                registered.push(type_name);
            }
        }

        let mut dir = header("specs/noctua.toml", CommentStyle::Line("#"));
        writeln!(dir, "\nmodule NoctuaColors\n").unwrap();
        for type_name in &registered {
            writeln!(dir, "singleton {type_name} 1.0 {type_name}.qml").unwrap();
        }
        files.push(EmittedFile::new("qml/qmldir", dir));

        files
    }
}

/// The semantic contract, as flat properties.
fn semantic(out: &mut String, mode: &noctua_engine::ResolvedMode) {
    writeln!(out, "    // --- Semantic contract ---").unwrap();
    let stems: std::collections::BTreeMap<String, String> = tokens::palette_tokens(mode)
        .into_iter()
        .map(|t| (t.stem(), value::hex(t.step.primary())))
        .collect();
    for alias in tokens::semantic_tokens(mode) {
        if let Some(hex) = stems.get(&alias.target) {
            writeln!(
                out,
                "    readonly property color {}: \"{hex}\"",
                name::qml_property(&alias.name)
            )
            .unwrap();
        }
    }
}

/// Every family's ramp.
fn families(out: &mut String, mode: &noctua_engine::ResolvedMode) {
    for (family, resolved) in &mode.families {
        writeln!(out, "\n    // --- {family} ---").unwrap();
        for step in &resolved.steps {
            writeln!(
                out,
                "    readonly property color {}{}: \"{}\"",
                name::qml_property(family),
                name::pascal(&step.role),
                value::hex(step.primary())
            )
            .unwrap();
        }
    }
}

/// Scales, as arrays.
///
/// An array because a scale is ordered and an array is the ordered container.
/// Named stops additionally get a flat property, since `Noctua.magnitudeHigh` is
/// what a Qt author will write and `Noctua.magnitude[3]` is not.
fn scales(out: &mut String, mode: &noctua_engine::ResolvedMode) {
    for (scale, resolved) in &mode.scales {
        writeln!(out, "\n    // --- {scale} ---").unwrap();
        writeln!(
            out,
            "    readonly property var {}: [",
            name::qml_property(scale)
        )
        .unwrap();
        for step in &resolved.steps {
            writeln!(out, "        \"{}\",", value::hex(step.primary())).unwrap();
        }
        writeln!(out, "    ]").unwrap();
        for step in &resolved.steps {
            if step.role.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            writeln!(
                out,
                "    readonly property color {}{}: \"{}\"",
                name::qml_property(scale),
                name::pascal(&step.role),
                value::hex(step.primary())
            )
            .unwrap();
        }
    }
}

/// The translucency ladder, in Qt's ARGB ordering.
///
/// [`value::hex_argb`] exists precisely so this file cannot accidentally emit
/// the web's ordering, which would read as a plausible colour rather than as an
/// error.
fn translucency(out: &mut String, palette: &Palette, mode: &noctua_engine::ResolvedMode) {
    writeln!(out, "\n    // --- Translucency ladder (ARGB) ---").unwrap();
    for alpha in tokens::alpha_tokens(palette, mode) {
        writeln!(
            out,
            "    readonly property color {}: \"{}\"",
            name::qml_property(&alpha.stem()),
            value::hex_argb(alpha.step.primary(), alpha.percentage)
        )
        .unwrap();
    }
}

/// The dense neutral ramps, as arrays.
fn ramps(out: &mut String, palette: &Palette) {
    for (ramp, steps) in &palette.neutral_ramps {
        writeln!(out, "\n    // --- Dense {ramp} ramp ---").unwrap();
        writeln!(
            out,
            "    readonly property var {}: [",
            name::qml_property(ramp)
        )
        .unwrap();
        for step in steps {
            writeln!(out, "        \"{}\",", value::hex(step.primary())).unwrap();
        }
        writeln!(out, "    ]").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use noctua_engine::build;

    use super::*;

    fn shipped() -> Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        build(&noctua_spec::load(path).expect("valid")).expect("builds")
    }

    fn file(name: &str) -> String {
        Qml.emit(&shipped())
            .into_iter()
            .find(|f| f.path == name)
            .unwrap_or_else(|| panic!("{name} should be emitted"))
            .contents
    }

    #[test]
    fn one_singleton_per_theme_and_mode_plus_a_qmldir() {
        let palette = shipped();
        let files = Qml.emit(&palette);
        assert_eq!(files.len(), palette.themes.len() * 2 + 1);
        assert!(files.iter().any(|f| f.path == "qml/qmldir"));
        assert!(files.iter().any(|f| f.path == "qml/OchreBalancedDark.qml"));
        assert!(files.iter().any(|f| f.path == "qml/OchreBalancedLight.qml"));
    }

    #[test]
    fn it_is_a_registered_singleton() {
        let qml = file("qml/OchreBalancedDark.qml");
        assert!(qml.contains("pragma Singleton"));
        assert!(qml.contains("import QtQuick"));
        assert!(qml.contains("QtObject {"));

        let dir = file("qml/qmldir");
        assert!(dir.contains("singleton OchreBalancedDark 1.0 OchreBalancedDark.qml"));
    }

    /// Qt's `color` type cannot parse anything else.
    #[test]
    fn every_value_is_hex() {
        let qml = file("qml/OchreBalancedDark.qml");

        // Checked on declarations rather than the whole file: the header
        // comment mentions `oklch()` precisely to explain why it is absent.
        for line in qml.lines().filter(|l| !l.trim_start().starts_with("//")) {
            assert!(!line.contains("oklch("), "Qt cannot parse oklch(): {line}");
        }

        for line in qml
            .lines()
            .filter(|l| l.contains("readonly property color"))
        {
            let value = line.split(": ").nth(1).expect("a value");
            // A quote, a hash, six digits, a quote — or eight digits for a
            // translucency stop, in Qt's ARGB ordering.
            let translucent = line.contains("A1:")
                || line
                    .split(':')
                    .next()
                    .is_some_and(|name| name.rsplit(' ').next().is_some_and(is_alpha_stop));
            let expected = if translucent { 11 } else { 9 };
            assert!(
                value.starts_with("\"#") && value.len() == expected,
                "not a {}-digit hex: {line}",
                expected - 3
            );
        }
    }

    /// `neutralA1` through `accentA12`: a camel-case stem, `A`, and a number.
    fn is_alpha_stop(name: &str) -> bool {
        let Some((stem, index)) = name.rsplit_once('A') else {
            return false;
        };
        !stem.is_empty() && !index.is_empty() && index.chars().all(|c| c.is_ascii_digit())
    }

    #[test]
    fn property_names_are_camel_case_as_qml_expects() {
        let qml = file("qml/OchreBalancedDark.qml");
        assert!(qml.contains("readonly property color accentSolid:"));
        assert!(qml.contains("readonly property color neutralBgApp:"));
        assert!(
            !qml.contains("accent-solid"),
            "kebab-case is not a QML identifier"
        );
    }

    /// The gap that made the two Quickshell projects invent their own greens.
    #[test]
    fn the_translucency_ladder_is_written_in_qts_ordering() {
        // Qt reads the eight-digit form as ARGB. The first stop is 2% opaque,
        // so its leading byte is 0x05 — if the ordering were the web's, the
        // *trailing* byte would be, and the colour would come out wrong in a
        // way no reviewer notices.
        let qml = file("qml/OchreBalancedLight.qml");
        assert!(
            qml.contains("readonly property color neutralA1: \"#05"),
            "the alpha byte must lead"
        );
        let opaque: Vec<&str> = qml
            .lines()
            .filter(|line| line.contains("A1: \"") || line.contains("A12: \""))
            .collect();
        assert!(!opaque.is_empty(), "no ladder was emitted");
        for line in opaque {
            let hex = line.split('"').nth(1).expect("a hex value");
            assert_eq!(hex.len(), 9, "eight digits and a hash: {line}");
        }
    }

    #[test]
    fn it_provides_the_semantic_tokens_the_fleet_was_missing() {
        let qml = file("qml/OchreBalancedDark.qml");
        for property in ["success", "warning", "danger", "info", "surface", "fg"] {
            assert!(
                qml.contains(&format!("readonly property color {property}:")),
                "missing {property}"
            );
        }
    }

    #[test]
    fn braces_and_brackets_balance() {
        let qml = file("qml/OchreBalancedDark.qml");
        assert_eq!(qml.matches('{').count(), qml.matches('}').count());
        assert_eq!(qml.matches('[').count(), qml.matches(']').count());
    }
}
