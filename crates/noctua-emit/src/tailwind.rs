//! Tailwind CSS v4.
//!
//! Tailwind v4 is CSS-first and OKLCH-native, so this is a thin bridge rather
//! than a translation: one `@import` and the utilities exist.
//!
//! The bridge uses `@theme inline`, which is the important detail. A plain
//! `@theme` block would bake the *current* value of each token into the
//! generated utilities, freezing the palette at build time — `bg-accent` would
//! keep its light-mode color when the page switched to dark. `inline` makes
//! Tailwind emit `var()` references instead, so utilities follow whichever
//! mode and gamut layer the runtime cascade selected.

use std::fmt::Write as _;

use noctua_engine::Palette;

use crate::tokens;
use crate::{CommentStyle, EmittedFile, Emitter, header};

/// The Tailwind v4 target.
#[derive(Debug, Clone, Copy)]
pub struct Tailwind;

impl Emitter for Tailwind {
    fn id(&self) -> &'static str {
        "tailwind"
    }

    fn describe(&self) -> &'static str {
        "a Tailwind v4 @theme bridge, drop-in via a single @import"
    }

    fn emit(&self, palette: &Palette) -> Vec<EmittedFile> {
        let mut files = vec![bridge_file(palette), entry_file(palette)];
        for theme in &palette.themes {
            files.push(palette_entry_file(&theme.name));
        }
        files
    }
}

/// The `@theme inline` mapping — every `--color-*` utility name.
///
/// **Palette-independent, and that is why it is its own file.** Every entry
/// points at a `--nc-*` token by name, and those names are identical in all
/// thirty-nine palettes; only the values behind them differ. Inlining this
/// mapping into each per-palette entry would repeat ~110 KB thirty-nine times
/// for ~4.3 MB of byte-identical text. As one shared import it costs ~110 KB
/// once, and each entry file is two lines.
fn bridge_file(palette: &Palette) -> EmittedFile {
    let prefix = &palette.prefix;
    let light = &palette.themes[0].modes[0];

    let mut out = header("specs/noctua.toml", CommentStyle::Block);
    writeln!(
        out,
        "\n/* The Tailwind v4 mapping: every --color-* utility name, pointing at\n\
        \x20  the token behind it.\n\
        \x20\n\
        \x20  This file declares no colour. It is the same for every palette,\n\
        \x20  because the token *names* do not vary — only their values do. Both\n\
        \x20  theme.css and every palette/<name>.css import it, so it is written\n\
        \x20  once rather than {} times.\n\
        \x20\n\
        \x20  Import an entry point instead of this; on its own it maps utilities\n\
        \x20  onto tokens nothing has defined. */\n",
        palette.themes.len() + 1
    )
    .unwrap();

    writeln!(
        out,
        "/* Makes `dark:` follow the same signals the tokens do. */\n\
         @custom-variant dark (&:where(.dark, .dark *, [data-theme=\"dark\"], [data-theme=\"dark\"] *));\n"
    )
    .unwrap();

    writeln!(
        out,
        "/* `inline` is load-bearing: it emits var() references rather than\n\
        \x20  frozen values, so utilities follow the active mode instead of\n\
        \x20  being stamped with whichever mode was active at build time. */\n\
         @theme inline {{"
    )
    .unwrap();

    writeln!(out, "  /* The semantic contract. */").unwrap();
    for alias in tokens::semantic_tokens(light) {
        // Points at the prefixed layer rather than repeating its target, so
        // the contract is defined in exactly one place and this file is the
        // Tailwind-shaped view of it.
        writeln!(
            out,
            "  --color-{}: var(--{prefix}-color-{});",
            alias.name, alias.name
        )
        .unwrap();
    }

    writeln!(out, "\n  /* Scales. */").unwrap();
    for name in tokens::scale_names(light) {
        writeln!(out, "  --color-{name}: var(--{prefix}-{name});").unwrap();
    }

    writeln!(
        out,
        "\n  /* Every palette step, for when the semantic layer is not enough. */"
    )
    .unwrap();
    for token in tokens::palette_tokens(light) {
        let stem = token.stem();
        writeln!(out, "  --color-{stem}: var(--{prefix}-{stem});").unwrap();
    }

    writeln!(out, "\n  /* The dense neutral ramps. */").unwrap();
    for name in tokens::ramp_names(palette) {
        writeln!(out, "  --color-{name}: var(--{prefix}-{name});").unwrap();
    }

    writeln!(out, "\n  /* The translucency ladder. */").unwrap();
    for alpha in tokens::alpha_tokens(palette, light) {
        let stem = alpha.stem();
        writeln!(out, "  --color-{stem}: var(--{prefix}-{stem});").unwrap();
    }

    writeln!(out, "}}").unwrap();
    EmittedFile::new("tailwind/bridge.css", out)
}

/// `tailwind/theme.css` — all thirty-nine palettes, switchable at runtime.
///
/// The long-standing public path, and its behaviour is unchanged. It used to
/// carry the mapping inline; now it imports it. Same tokens, same utilities,
/// same `dark:` variant — one more HTTP request in a dev server, and none at
/// all once a bundler has flattened it, which is the only way Tailwind ever
/// reads this file.
fn entry_file(palette: &Palette) -> EmittedFile {
    let mut out = header("specs/noctua.toml", CommentStyle::Block);
    writeln!(
        out,
        "\n/* Tailwind v4, every palette. Two lines in your entry CSS:\n\
        \x20\n\
        \x20      @import \"tailwindcss\";\n\
        \x20      @import \"<path to the system>/tailwind/theme.css\";\n\
        \x20\n\
        \x20  That is the whole integration.\n\
        \x20\n\
        \x20  This carries all {} palettes, so you can switch at runtime with a\n\
        \x20  data-palette attribute. If you have settled on one, import\n\
        \x20  tailwind/palette/<name>.css instead — same utilities, a small\n\
        \x20  fraction of the bytes. */\n",
        palette.themes.len()
    )
    .unwrap();

    writeln!(out, "@import \"../css/index.css\";").unwrap();
    writeln!(out, "@import \"./bridge.css\";").unwrap();
    EmittedFile::new("tailwind/theme.css", out)
}

/// `tailwind/palette/<theme>.css` — one palette's utilities, and nothing else.
///
/// Two `@import`s and no rules of its own, which is not incidental: `@import`
/// must precede every other rule in a stylesheet, so a file that is nothing but
/// imports cannot violate that. `packaging::relative_imports_resolve` checks
/// both targets are inside the published package.
fn palette_entry_file(theme: &str) -> EmittedFile {
    let mut out = header("specs/noctua.toml", CommentStyle::Block);
    writeln!(
        out,
        "\n/* Tailwind v4, the {theme} palette alone. Two lines in your entry CSS:\n\
        \x20\n\
        \x20      @import \"tailwindcss\";\n\
        \x20      @import \"<path to the system>/tailwind/palette/{theme}.css\";\n\
        \x20\n\
        \x20  Identical utilities to theme.css — bg-surface, text-fg, dark: and\n\
        \x20  the rest — carrying one palette instead of all of them. */\n"
    )
    .unwrap();

    writeln!(out, "@import \"../../css/palette/{theme}.css\";").unwrap();
    writeln!(out, "@import \"../bridge.css\";").unwrap();
    EmittedFile::new(format!("tailwind/palette/{theme}.css"), out)
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
        Tailwind
            .emit(&shipped())
            .into_iter()
            .find(|f| f.path == name)
            .unwrap_or_else(|| panic!("{name} should be emitted"))
            .contents
    }

    /// The mapping — where every assertion about `--color-*` names belongs.
    fn emitted() -> String {
        file("tailwind/bridge.css")
    }

    #[test]
    fn the_bridge_the_entry_and_one_file_per_palette() {
        let palette = shipped();
        let files = Tailwind.emit(&palette);
        assert_eq!(files.len(), palette.themes.len() + 2);
        assert!(files.iter().any(|f| f.path == "tailwind/theme.css"));
        assert!(files.iter().any(|f| f.path == "tailwind/bridge.css"));
        for theme in &palette.themes {
            let path = format!("tailwind/palette/{}.css", theme.name);
            assert!(files.iter().any(|f| f.path == path), "{path} missing");
        }
    }

    /// Both entry points must be **imports only**.
    ///
    /// `@import` is only honoured before every other rule, so a stray
    /// declaration here would silently disable the imports that follow it —
    /// and the symptom is an unstyled page, not an error.
    #[test]
    fn the_entry_points_carry_nothing_but_imports() {
        let palette = shipped();
        let mut entries = vec!["tailwind/theme.css".to_owned()];
        entries.extend(
            palette
                .themes
                .iter()
                .map(|t| format!("tailwind/palette/{}.css", t.name)),
        );

        for path in entries {
            // Strip block comments first. The generated headers are multi-line
            // and their middle lines are ordinary prose, so a line-by-line
            // "does it look like a comment" test reads them as CSS rules.
            let contents = file(&path);
            let mut code = String::new();
            let mut rest = contents.as_str();
            while let Some(open) = rest.find("/*") {
                code.push_str(&rest[..open]);
                // An unterminated comment swallows the remainder, which is what
                // a CSS parser does too.
                let Some(close) = rest[open..].find("*/") else {
                    rest = "";
                    break;
                };
                rest = &rest[open + close + 2..];
            }
            code.push_str(rest);

            for line in code.lines().map(str::trim).filter(|l| !l.is_empty()) {
                assert!(
                    line.starts_with("@import"),
                    "{path} has a rule before its imports: {line}"
                );
            }
        }
    }

    /// A per-palette entry must reach the palette *and* the mapping.
    ///
    /// Either one alone renders nothing: the palette without the bridge
    /// defines tokens no utility names, and the bridge without the palette
    /// names utilities no token defines.
    #[test]
    fn a_palette_entry_imports_its_palette_and_the_bridge() {
        let css = file("tailwind/palette/blue-vivid.css");
        assert!(css.contains(r#"@import "../../css/palette/blue-vivid.css";"#));
        assert!(css.contains(r#"@import "../bridge.css";"#));
    }

    /// The mapping is written once, not once per palette.
    ///
    /// If this ever regresses, the tree grows by ~4.3 MB of byte-identical
    /// text and nothing else changes — easy to miss, and the whole reason the
    /// bridge is a separate file.
    #[test]
    fn the_mapping_is_not_repeated_in_every_palette_entry() {
        let palette = shipped();
        for theme in &palette.themes {
            let css = file(&format!("tailwind/palette/{}.css", theme.name));
            assert!(
                !css.contains("@theme inline"),
                "{} inlines the mapping instead of importing it",
                theme.name
            );
            assert!(css.len() < 1_000, "{} is not two imports", theme.name);
        }
    }

    /// Without `inline`, every utility freezes at its build-time value and
    /// dark mode silently stops working.
    #[test]
    fn the_theme_block_is_inline() {
        let css = emitted();
        assert!(css.contains("@theme inline {"), "must be `@theme inline`");
        assert!(!css.contains("@theme {"));
    }

    #[test]
    fn every_theme_entry_is_a_var_reference() {
        let css = emitted();
        let block = css.split("@theme inline {").nth(1).expect("a theme block");
        for line in block
            .lines()
            .filter(|l| l.trim_start().starts_with("--color-"))
        {
            assert!(
                line.contains("var(--nc-"),
                "a frozen value would break mode switching: {line}"
            );
        }
    }

    #[test]
    fn the_dark_variant_matches_the_token_selectors() {
        let css = emitted();
        assert!(css.contains("@custom-variant dark"));
        assert!(css.contains(".dark"));
        assert!(css.contains(r#"[data-theme="dark"]"#));
    }

    /// The all-palettes entry must still reach every palette.
    ///
    /// This is the public path from 0.1.x and its behaviour must not move: it
    /// carried the mapping inline and now imports it, but what a consumer gets
    /// is the same.
    #[test]
    fn it_imports_the_tokens_it_bridges() {
        let css = file("tailwind/theme.css");
        assert!(css.contains(r#"@import "../css/index.css";"#));
        assert!(css.contains(r#"@import "./bridge.css";"#));
    }

    #[test]
    fn the_semantic_contract_is_exposed_as_utilities() {
        let css = emitted();
        for name in [
            "--color-surface",
            "--color-fg",
            "--color-accent",
            "--color-danger",
        ] {
            assert!(css.contains(&format!("{name}: var(")), "missing {name}");
        }
    }
}
