//! SCSS variables and a flat lookup map.
//!
//! Cheap to emit and it unblocks consumers whose build predates custom
//! properties. Values are hex, because a Sass compiler resolves these at build
//! time and cannot follow a `var()` — which also means SCSS output cannot
//! follow a runtime mode switch. Both modes are emitted under distinct names
//! and the consumer picks; that limitation is inherent to compile-time
//! variables, not a shortcut taken here.

use std::fmt::Write as _;

use noctua_engine::Palette;

use crate::value;
use crate::{CommentStyle, EmittedFile, Emitter, header, tokens};

/// The SCSS target.
#[derive(Debug, Clone, Copy)]
pub struct Scss;

impl Emitter for Scss {
    fn id(&self) -> &'static str {
        "scss"
    }

    fn describe(&self) -> &'static str {
        "SCSS variables and a flat map, for build-time consumers"
    }

    fn emit(&self, palette: &Palette) -> Vec<EmittedFile> {
        let prefix = &palette.prefix;
        let mut out = header("specs/noctua.toml", CommentStyle::Line("//"));
        writeln!(
            out,
            "//\n\
             // Sass resolves these at build time, so they cannot follow a runtime\n\
             // mode switch. Both modes are emitted; pick one, or use the CSS\n\
             // custom properties instead if the page needs to switch live.\n"
        )
        .unwrap();

        let mut entries: Vec<(String, String)> = Vec::new();
        for (ramp, steps) in &palette.neutral_ramps {
            for step in steps {
                let key = format!("{ramp}-{}", step.index);
                let hex = value::hex(step.primary());
                writeln!(out, "${prefix}-{key}: {hex};").unwrap();
                entries.push((key, hex));
            }
        }
        writeln!(out).unwrap();

        for theme in &palette.themes {
            for mode in &theme.modes {
                writeln!(out, "// {} / {}", theme.name, mode.mode.id()).unwrap();
                for token in tokens::palette_tokens(mode) {
                    let key = format!("{}-{}-{}", theme.name, mode.mode.id(), token.stem());
                    let hex = value::hex(token.step.primary());
                    writeln!(out, "${prefix}-{key}: {hex};").unwrap();
                    entries.push((key, hex));
                }
                for alpha in tokens::alpha_tokens(palette, mode) {
                    let key = format!("{}-{}-{}", theme.name, mode.mode.id(), alpha.stem());
                    // Sass has no `color-mix`, so the ladder is eight-digit hex
                    // in the web ordering: channels first, alpha last.
                    let hex = value::hex_rgba(alpha.step.primary(), alpha.percentage);
                    writeln!(out, "${prefix}-{key}: {hex};").unwrap();
                    entries.push((key, hex));
                }
                for (scale, resolved) in &mode.scales {
                    for step in &resolved.steps {
                        let key =
                            format!("{}-{}-{scale}-{}", theme.name, mode.mode.id(), step.role);
                        let hex = value::hex(step.primary());
                        writeln!(out, "${prefix}-{key}: {hex};").unwrap();
                        entries.push((key, hex));
                    }
                }
                writeln!(out).unwrap();
            }
        }

        writeln!(
            out,
            "// Flat lookup: map.get($noctua-colors, \"balanced-light-accent-solid\")"
        )
        .unwrap();
        writeln!(out, "$noctua-colors: (").unwrap();
        for (key, hex) in &entries {
            writeln!(out, "  \"{key}\": {hex},").unwrap();
        }
        writeln!(out, ");").unwrap();

        vec![EmittedFile::new("scss/_noctua.scss", out)]
    }
}

#[cfg(test)]
mod tests {
    use noctua_engine::build;

    use super::*;

    fn emitted() -> String {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        let palette = build(&noctua_spec::load(path).expect("valid")).expect("builds");
        Scss.emit(&palette).remove(0).contents
    }

    #[test]
    fn it_emits_variables_and_a_map() {
        let scss = emitted();
        assert!(scss.contains("$nc-ochre-balanced-light-accent-solid: #"));
        assert!(scss.contains("$noctua-colors: ("));
        assert!(scss.contains("\"ochre-balanced-light-accent-solid\": #"));
    }

    #[test]
    fn both_modes_are_available_since_sass_cannot_switch_at_runtime() {
        let scss = emitted();
        assert!(scss.contains("$nc-ochre-balanced-light-accent-solid:"));
        assert!(scss.contains("$nc-ochre-balanced-dark-accent-solid:"));
    }

    #[test]
    fn values_are_hex_because_sass_cannot_follow_a_var() {
        let scss = emitted();
        assert!(
            !scss.contains("var(--"),
            "a Sass build cannot resolve var()"
        );
        assert!(
            !scss.contains("oklch("),
            "older Sass consumers are the point of this target"
        );
    }

    #[test]
    fn the_map_is_balanced() {
        let scss = emitted();
        assert_eq!(scss.matches('(').count(), scss.matches(')').count());
    }
}
