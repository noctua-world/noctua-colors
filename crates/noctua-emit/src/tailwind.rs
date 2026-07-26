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
        let prefix = &palette.prefix;
        let light = &palette.themes[0].modes[0];

        let mut out = header("specs/noctua.toml", CommentStyle::Block);
        writeln!(
            out,
            "\n/* Tailwind v4. One line in your entry CSS:\n\
            \x20\n\
            \x20      @import \"tailwindcss\";\n\
            \x20      @import \"<path to dist>/tailwind/theme.css\";\n\
            \x20\n\
            \x20  That is the whole integration. */\n"
        )
        .unwrap();

        writeln!(out, "@import \"../css/index.css\";\n").unwrap();

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
        vec![EmittedFile::new("tailwind/theme.css", out)]
    }
}

#[cfg(test)]
mod tests {
    use noctua_engine::build;

    use super::*;

    fn emitted() -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        let palette = build(&noctua_spec::load(path).expect("valid")).expect("builds");
        Tailwind.emit(&palette).remove(0).contents
    }

    #[test]
    fn it_is_a_single_drop_in_file() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        let palette = build(&noctua_spec::load(path).expect("valid")).expect("builds");
        let files = Tailwind.emit(&palette);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "tailwind/theme.css");
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

    #[test]
    fn it_imports_the_tokens_it_bridges() {
        assert!(emitted().contains(r#"@import "../css/index.css";"#));
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
