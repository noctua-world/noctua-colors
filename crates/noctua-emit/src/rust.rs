//! A ready-to-depend-on Rust crate of constants.
//!
//! Emitted as a whole crate rather than a bare `.rs` file, so a consumer adds
//! one line to `Cargo.toml` and is done — no `include!`, no build script, no
//! path juggling:
//!
//! ```toml
//! noctua-colors-tokens = { path = "../noctua-colors/dist/rust" }
//! ```
//!
//! Everything is `const`, so there is no runtime cost and no allocation. The
//! generated crate has no dependencies of its own, including on this project:
//! a consumer pulling in tokens should not inherit a color-math library.

use std::fmt::Write as _;

use noctua_engine::{Palette, ResolvedStep};

use crate::name;
use crate::value;
use crate::{CommentStyle, EmittedFile, Emitter, header};

/// The Rust constants target.
#[derive(Debug, Clone, Copy)]
pub struct Rust;

impl Emitter for Rust {
    fn id(&self) -> &'static str {
        "rust"
    }

    fn describe(&self) -> &'static str {
        "a dependency-free Rust crate of const colors, for Dioxus and native consumers"
    }

    fn emit(&self, palette: &Palette) -> Vec<EmittedFile> {
        vec![
            EmittedFile::new("rust/Cargo.toml", cargo_toml(palette)),
            EmittedFile::new("rust/src/lib.rs", lib_rs(palette)),
        ]
    }
}

/// The Cargo feature that gates one theme's module.
fn theme_feature(theme: &str) -> String {
    name::identifier(theme).to_lowercase()
}

fn cargo_toml(palette: &Palette) -> String {
    let mut out = header("specs/noctua.toml", CommentStyle::Line("#"));
    out.push_str(
        "\n[package]\n\
         name = \"noctua-colors-tokens\"\n\
         description = \"Generated color tokens from noctua-colors.\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         license = \"MIT OR Apache-2.0\"\n\
         \n\
         # Its own workspace root.\n\
         #\n\
         # Without this the crate cannot be built standalone from inside the\n\
         # repository that generated it, and vendoring it into a consumer\n\
         # would quietly absorb it into their workspace — inheriting their\n\
         # lints, their profiles and their resolver version.\n\
         [workspace]\n\
         \n\
         # Deliberately empty. A consumer of tokens should not inherit a\n\
         # color-math library.\n\
         [dependencies]\n",
    );

    // One feature per theme, and only the first on by default.
    //
    // Not about compile time — the whole crate builds in about half a second.
    // It is about `.rmeta`: every consumer that names this crate loads the
    // metadata for every theme in it, which is megabytes for a grid this size,
    // and the example consumer uses five modules out of thirty-seven. A feature
    // per theme lets a consumer pay for the palettes it ships.
    out.push_str(
        "\n# One feature per palette. The default is the first one in the spec,\n\
         # which is the palette the CSS binds to `:root`.\n\
         #\n\
         # A consumer shipping one palette should ask for one:\n\
         #\n\
         #     noctua-colors-tokens = { path = \"...\", default-features = false, \\\n\
         #                              features = [\"blue_vivid\"] }\n\
         #\n\
         # `all` is there for a tool that needs every palette, such as a picker.\n\
         [features]\n",
    );

    let features: Vec<String> = palette
        .themes
        .iter()
        .map(|theme| theme_feature(&theme.name))
        .collect();

    if let Some(first) = features.first() {
        writeln!(out, "default = [\"{first}\"]").expect("write");
    }
    for feature in &features {
        writeln!(out, "{feature} = []").expect("write");
    }
    writeln!(
        out,
        "all = [{}]",
        features
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(", ")
    )
    .expect("write");

    out
}

/// Writes one `Color` constant.
fn constant(out: &mut String, indent: &str, name: &str, step: &ResolvedStep) {
    let color = step.primary();
    let (r, g, b) = value::rgb_bytes(color);
    writeln!(out, "{indent}/// `{}`", value::hex(color)).expect("string write");
    writeln!(
        out,
        "{indent}pub const {name}: Color = Color {{ r: {r}, g: {g}, b: {b}, \
         l: {:.4}, c: {:.4}, h: {:.2}, hex: \"{}\" }};",
        color.oklch.l,
        color.oklch.c,
        color.oklch.h,
        value::hex(color)
    )
    .expect("string write");
}

fn lib_rs(palette: &Palette) -> String {
    let mut out = header("specs/noctua.toml", CommentStyle::Line("//!"));
    out.push_str(
        "//!\n\
         //! Every color is a `const`, so nothing here allocates or costs\n\
         //! anything at runtime.\n\
         \n\
         #![no_std]\n\
         #![allow(clippy::unreadable_literal)]\n\
         \n",
    );

    out.push_str(
        "/// One resolved color.\n\
         ///\n\
         /// Carries both the eight-bit channels a renderer wants and the OKLCH\n\
         /// coordinates the color was defined by, because throwing the latter\n\
         /// away makes it impossible to reason about a token after the fact.\n\
         #[derive(Debug, Clone, Copy, PartialEq)]\n\
         pub struct Color {\n\
        \x20   /// Red channel, 0 to 255.\n\
        \x20   pub r: u8,\n\
        \x20   /// Green channel, 0 to 255.\n\
        \x20   pub g: u8,\n\
        \x20   /// Blue channel, 0 to 255.\n\
        \x20   pub b: u8,\n\
        \x20   /// Oklab lightness, 0 to 1.\n\
        \x20   pub l: f32,\n\
        \x20   /// Chroma.\n\
        \x20   pub c: f32,\n\
        \x20   /// Hue in degrees.\n\
        \x20   pub h: f32,\n\
        \x20   /// The color as `#rrggbb`.\n\
        \x20   pub hex: &'static str,\n\
         }\n\
         \n\
         impl Color {\n\
        \x20   /// Packs the channels into `0xRRGGBB`.\n\
        \x20   #[must_use]\n\
        \x20   pub const fn packed(self) -> u32 {\n\
        \x20       ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)\n\
        \x20   }\n\
         }\n\n",
    );

    out.push_str(
        "/// One stop of the translucency ladder.\n\
         ///\n\
         /// A separate type rather than an `alpha` field on [`Color`], so the\n\
         /// several thousand opaque constants do not each carry a byte that is\n\
         /// always 255 — and so a translucent token cannot be passed where an\n\
         /// opaque one was meant.\n\
         ///\n\
         /// **These composite differently on every backdrop**, which is what\n\
         /// alpha is for and also why no contrast gate can audit them: contrast\n\
         /// is a property of a pair of opaque colours. Measure the composite in\n\
         /// place, or use a solved step instead.\n\
         #[derive(Debug, Clone, Copy, PartialEq)]\n\
         pub struct Alpha {\n\
        \x20   /// The colour being washed, at full opacity.\n\
        \x20   pub color: Color,\n\
        \x20   /// Opacity, 0 to 255.\n\
        \x20   pub alpha: u8,\n\
        \x20   /// The stop as `#rrggbbaa`, the web ordering.\n\
        \x20   pub hex: &'static str,\n\
         }\n\
         \n\
         impl Alpha {\n\
        \x20   /// Packs into `0xAARRGGBB`, which is what Qt and Win32 expect.\n\
        \x20   ///\n\
        \x20   /// Note the ordering: alpha **leads**. The web's `#rrggbbaa` is\n\
        \x20   /// in [`Alpha::hex`], and the two are not interchangeable.\n\
        \x20   #[must_use]\n\
        \x20   pub const fn packed_argb(self) -> u32 {\n\
        \x20       ((self.alpha as u32) << 24) | self.color.packed()\n\
        \x20   }\n\
         }\n\n",
    );

    // The dense neutral ramps, which do not vary by theme or mode.
    for (ramp, steps) in &palette.neutral_ramps {
        let module = name::identifier(ramp).to_lowercase();
        writeln!(
            out,
            "/// The dense `{ramp}` ramp, shared by every theme and mode."
        )
        .expect("write");
        writeln!(out, "pub mod {module} {{").expect("write");
        writeln!(out, "    use super::Color;\n").expect("write");
        for step in steps {
            constant(&mut out, "    ", &format!("STEP_{}", step.index), step);
        }
        writeln!(out, "\n    /// Every step, in ramp order.").expect("write");
        writeln!(out, "    pub const ALL: [Color; {}] = [", steps.len()).expect("write");
        for step in steps {
            writeln!(out, "        STEP_{},", step.index).expect("write");
        }
        writeln!(out, "    ];").expect("write");
        writeln!(out, "}}\n").expect("write");
    }

    for theme in &palette.themes {
        write_theme(&mut out, palette, theme);
    }

    out
}

/// One mode's translucency ladder.
fn write_alpha(out: &mut String, palette: &Palette, mode: &noctua_engine::ResolvedMode) {
    let stops = crate::tokens::alpha_tokens(palette, mode);
    if stops.is_empty() {
        return;
    }

    writeln!(out, "        /// The translucency ladder.").expect("write");
    writeln!(out, "        pub mod alpha {{").expect("write");
    writeln!(out, "            use crate::{{Alpha, Color}};\n").expect("write");
    for stop in stops {
        let color = stop.step.primary();
        let (r, g, b) = crate::value::rgb_bytes(color);
        writeln!(
            out,
            "            /// `{}` at {}% opacity.",
            stop.source(&palette.alpha.role),
            stop.percentage
        )
        .expect("write");
        writeln!(
            out,
            "            pub const {}: Alpha = Alpha {{ color: Color {{ r: {r}, g: {g}, \
             b: {b}, l: {:.4}, c: {:.4}, h: {:.2}, hex: \"{}\" }}, alpha: {}, hex: \"{}\" }};",
            name::screaming_snake(&stop.stem()),
            color.oklch.l,
            color.oklch.c,
            color.oklch.h,
            crate::value::hex(color),
            crate::value::alpha_byte(stop.percentage),
            crate::value::hex_rgba(color, stop.percentage),
        )
        .expect("write");
    }
    writeln!(out, "        }}\n").expect("write");
}

/// One theme's nested modules.
fn write_theme(out: &mut String, palette: &Palette, theme: &noctua_engine::ResolvedTheme) {
    let theme_module = name::identifier(&theme.name).to_lowercase();
    writeln!(out, "/// Theme `{}`.", theme.name).expect("write");
    writeln!(
        out,
        "///\n/// Behind the `{}` feature.",
        theme_feature(&theme.name)
    )
    .expect("write");
    writeln!(out, "#[cfg(feature = \"{}\")]", theme_feature(&theme.name)).expect("write");
    writeln!(out, "pub mod {theme_module} {{").expect("write");

    for mode in &theme.modes {
        writeln!(out, "    /// {} mode.", name::pascal(mode.mode.id())).expect("write");
        writeln!(out, "    pub mod {} {{", mode.mode.id()).expect("write");

        for family in mode.families.values() {
            let family_module = name::identifier(&family.name).to_lowercase();
            writeln!(out, "        /// Family `{}`.", family.name).expect("write");
            writeln!(out, "        pub mod {family_module} {{").expect("write");
            writeln!(out, "            use crate::Color;\n").expect("write");
            for step in &family.steps {
                constant(
                    out,
                    "            ",
                    &name::screaming_snake(&step.role),
                    step,
                );
            }
            writeln!(out, "\n            /// Every step, in ramp order.").expect("write");
            writeln!(
                out,
                "            pub const ALL: [Color; {}] = [",
                family.steps.len()
            )
            .expect("write");
            for step in &family.steps {
                writeln!(
                    out,
                    "                {},",
                    name::screaming_snake(&step.role)
                )
                .expect("write");
            }
            writeln!(out, "            ];").expect("write");
            writeln!(out, "        }}\n").expect("write");
        }

        write_alpha(out, palette, mode);

        for (scale, resolved) in &mode.scales {
            let module = name::identifier(scale).to_lowercase();
            writeln!(out, "        /// Scale `{scale}`.").expect("write");
            writeln!(out, "        pub mod {module} {{").expect("write");
            writeln!(out, "            use crate::Color;\n").expect("write");
            for step in &resolved.steps {
                constant(
                    out,
                    "            ",
                    &name::screaming_snake(&format!("stop-{}", step.role)),
                    step,
                );
            }
            writeln!(
                out,
                "\n            /// Every entry, in order.\n            pub const ALL: [Color; {}] = [",
                resolved.steps.len()
            )
            .expect("write");
            for step in &resolved.steps {
                writeln!(
                    out,
                    "                {},",
                    name::screaming_snake(&format!("stop-{}", step.role))
                )
                .expect("write");
            }
            writeln!(out, "            ];").expect("write");
            writeln!(out, "        }}").expect("write");
        }

        writeln!(out, "    }}\n").expect("write");
    }
    writeln!(out, "}}\n").expect("write");
}

#[cfg(test)]
mod tests {
    use noctua_engine::build;

    use super::*;

    fn shipped() -> Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        build(&noctua_spec::load(path).expect("valid")).expect("builds")
    }

    fn lib() -> String {
        Rust.emit(&shipped())
            .into_iter()
            .find(|f| f.path == "rust/src/lib.rs")
            .expect("lib.rs")
            .contents
    }

    #[test]
    fn it_emits_a_complete_crate() {
        let files = Rust.emit(&shipped());
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(
            paths.contains(&"rust/Cargo.toml"),
            "a consumer must be able to path-depend"
        );
        assert!(paths.contains(&"rust/src/lib.rs"));
    }

    /// A generated crate that cannot be built is not a deliverable.
    ///
    /// Without its own `[workspace]`, `cargo build` inside `dist/rust` fails
    /// outright — the crate sits under this repository's workspace root and
    /// cargo refuses to build a package that "believes it's in a workspace
    /// when it's not". Vendoring it into a consumer would have the same effect
    /// in reverse, silently absorbing it into their workspace.
    ///
    /// `cargo xtask check` compiles it for real; this catches the manifest
    /// mistake without paying for a build.
    #[test]
    fn the_generated_crate_is_its_own_workspace_root() {
        let toml = Rust
            .emit(&shipped())
            .into_iter()
            .find(|f| f.path == "rust/Cargo.toml")
            .expect("Cargo.toml")
            .contents;

        let parsed: toml::Value =
            toml::from_str(&toml).expect("the generated manifest must be valid TOML");
        assert!(
            parsed.get("workspace").is_some(),
            "the manifest must declare its own workspace:\n{toml}"
        );
        assert_eq!(
            parsed["package"]["name"].as_str(),
            Some("noctua-colors-tokens")
        );
    }

    /// A consumer of colors should not inherit a color-math library.
    #[test]
    fn the_generated_crate_has_no_dependencies() {
        let toml = Rust
            .emit(&shipped())
            .into_iter()
            .find(|f| f.path == "rust/Cargo.toml")
            .expect("Cargo.toml")
            .contents;
        // Up to the next section: `[features]` follows `[dependencies]`, and
        // features are not dependencies.
        let deps = toml
            .split("[dependencies]")
            .nth(1)
            .expect("a dependencies section")
            .split("\n[")
            .next()
            .expect("a section body");
        for line in deps.lines() {
            let line = line.trim();
            assert!(
                line.is_empty() || line.starts_with('#'),
                "the generated crate must stay dependency-free, found: {line}"
            );
        }
    }

    /// Every consumer that names the crate loads the metadata for whatever is
    /// compiled in, so a program shipping one palette must be able to compile
    /// one palette.
    #[test]
    fn every_theme_is_behind_a_feature_of_its_own() {
        let palette = shipped();
        let files = Rust.emit(&palette);
        let toml = &files
            .iter()
            .find(|f| f.path == "rust/Cargo.toml")
            .expect("Cargo.toml")
            .contents;
        let lib = &files
            .iter()
            .find(|f| f.path == "rust/src/lib.rs")
            .expect("lib.rs")
            .contents;

        for theme in &palette.themes {
            let feature = name::identifier(&theme.name).to_lowercase();
            assert!(
                toml.contains(&format!("\n{feature} = []")),
                "{feature} is not a feature"
            );
            assert!(
                lib.contains(&format!("#[cfg(feature = \"{feature}\")]")),
                "{feature}'s module is not gated"
            );
        }

        // The default is the palette the CSS binds to `:root`, so the obvious
        // dependency line gives a consumer the obvious palette.
        let first = name::identifier(&palette.themes[0].name).to_lowercase();
        assert!(toml.contains(&format!("default = [\"{first}\"]")), "{toml}");
    }

    #[test]
    fn constants_are_nested_by_theme_mode_and_family() {
        let lib = lib();
        assert!(lib.contains("pub mod ochre_balanced {"));
        assert!(lib.contains("pub mod light {"));
        assert!(lib.contains("pub mod dark {"));
        assert!(lib.contains("pub mod accent {"));
        assert!(lib.contains("pub const SOLID: Color = Color {"));
        assert!(lib.contains("pub const TEXT_STRONG: Color = Color {"));
    }

    #[test]
    fn it_keeps_the_oklch_coordinates_not_just_the_bytes() {
        // Discarding them makes a token impossible to reason about later.
        let lib = lib();
        assert!(lib.contains("pub l: f32,"));
        assert!(lib.contains("pub c: f32,"));
        assert!(lib.contains("pub h: f32,"));
    }

    #[test]
    fn the_dense_ramps_are_emitted_once() {
        let lib = lib();
        assert!(lib.contains("pub mod gray {"));
        assert!(lib.contains("pub const STEP_1: Color"));
        assert!(lib.contains("pub const STEP_24: Color"));

        // The tinted variants are siblings, not a second numbering scheme:
        // `gray_cool::STEP_7` is `gray::STEP_7` at another temperature.
        assert!(lib.contains("pub mod gray_cool {"));
        assert!(lib.contains("pub mod gray_warm {"));
    }

    #[test]
    fn every_family_exposes_an_ordered_array() {
        let lib = lib();
        assert!(lib.contains("pub const ALL: [Color; 12] = ["));
    }

    #[test]
    fn it_is_no_std_so_embedded_and_wasm_consumers_work() {
        assert!(lib().contains("#![no_std]"));
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(Rust.emit(&shipped()), Rust.emit(&shipped()));
    }
}
