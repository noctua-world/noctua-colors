//! CSS custom properties.
//!
//! # Light and dark without any configuration
//!
//! The brief asks that a consumer pick a light/dark strategy without editing
//! generated files. The answer is to stop asking: all three strategies are
//! emitted, composed so they cannot conflict.
//!
//! ```css
//! :root { /* light */ }
//! @media (prefers-color-scheme: dark) {
//!   :root:not([data-theme="light"]):not(.light) { /* dark */ }
//! }
//! [data-theme="dark"], .dark { /* dark */ }
//! [data-theme="light"], .light { /* light */ }
//! ```
//!
//! The system preference works with no setup. A class or a data attribute
//! overrides it, on the root or on any subtree. The `:not()` guards stop a
//! forced light theme from being overruled by a dark system preference — the
//! one combination that would otherwise break.
//!
//! **The dark block is written twice, on purpose.** Once inside the `@media`
//! query and once for the forced selector, with identical declarations, because
//! the two cannot be combined: a selector list is dropped entirely if any
//! selector in it is invalid, and `@media` cannot be factored into one. Anyone
//! reading this as duplication to remove should know it compresses to nothing —
//! the two blocks are byte-identical, which is the best case for gzip — and that
//! merging them silently loses one of the two mechanisms.
//!
//! # Three value layers
//!
//! Every palette token is emitted three times: hex, then `oklch()` behind
//! `@supports`, then a wider `oklch()` behind `@media (color-gamut: p3)`.
//!
//! The third layer is the point of relative chroma made visible. The same
//! token genuinely has a *different number* there, because its chroma was
//! resolved against a wider boundary. It is not the same color declared twice.
//!
//! # Why the semantic layer is prefixed too
//!
//! `--nc-color-surface`, not `--color-surface`. Tailwind v4 fixes its theme
//! namespace at `--color-*` and nothing can rename it, so the *Tailwind* target
//! emits those names — and since `tailwind/theme.css` imports this file, an
//! unprefixed layer here would occupy that namespace for every consumer,
//! including one who has never installed Tailwind and whose own
//! `--color-surface` would then be silently overridden.
//!
//! Prefixing costs three characters and buys the guarantee that nothing this
//! project emits can collide with anything a consumer writes.

use std::fmt::Write as _;

use noctua_engine::{Palette, ResolvedMode, ResolvedStep};

use crate::tokens;
use crate::value;
use crate::{CommentStyle, EmittedFile, Emitter, header};

/// The CSS custom-properties target.
#[derive(Debug, Clone, Copy)]
pub struct Css;

impl Emitter for Css {
    fn id(&self) -> &'static str {
        "css"
    }

    fn describe(&self) -> &'static str {
        "CSS custom properties with a hex fallback and a Display P3 upgrade layer"
    }

    fn emit(&self, palette: &Palette) -> Vec<EmittedFile> {
        let semantic = tokens::semantic_layer(palette);
        let mut files = vec![ramp_file(palette), contexts_file(palette, &semantic)];

        for (index, theme) in palette.themes.iter().enumerate() {
            files.push(theme_file(palette, theme, index == 0, &semantic));
            files.push(palette_file(palette, theme, &semantic));
        }

        files.push(index_file(palette));
        files
    }
}

/// How a token's value is spelled in a given layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    /// `#rrggbb`, understood everywhere.
    Hex,
    /// `oklch()` resolved against the primary gamut.
    Oklch,
    /// `oklch()` resolved against a wider gamut.
    Wide(usize),
}

impl Layer {
    fn render(self, step: &ResolvedStep) -> String {
        match self {
            Self::Hex => value::hex(step.primary()),
            Self::Oklch => value::oklch(step.primary()),
            Self::Wide(slot) => value::oklch(&step.renditions[slot]),
        }
    }
}

fn spec_path() -> &'static str {
    "specs/noctua.toml"
}

/// The theme's own scope, and the selectors for each mode override.
struct Selectors {
    light: String,
    system_dark: String,
    forced_dark: String,
    forced_light: String,
}

/// The stylesheet a theme is written to.
///
/// Every theme's file is named after the theme. The default one drops the
/// prefix — `balanced.css` rather than `theme-balanced.css` — because it is
/// the one a consumer links by default and the others are opt-in extras asked
/// for by name.
///
/// **Renaming a theme renames this file.** That is deliberate: the file holds
/// exactly one theme and saying so is worth more than a fixed name. The cost
/// is that a rename is a breaking change for anyone linking it directly, and
/// the references inside this repository all have to move together — the
/// README, `examples/`, the site's integration snippet, and
/// `noctua_docs::token_files`. `system/css/index.css` is the name that never
/// moves, for consumers who would rather not track theme names at all.
fn theme_file_name(theme: &str, is_default: bool) -> String {
    if is_default {
        format!("{theme}.css")
    } else {
        format!("theme-{theme}.css")
    }
}

fn selectors(theme: &str, is_default: bool) -> Selectors {
    if is_default {
        Selectors {
            light: ":root".to_owned(),
            // The guards matter: without them a system dark preference would
            // override a page that explicitly asked for light.
            system_dark: r#":root:not([data-theme="light"]):not(.light)"#.to_owned(),
            forced_dark: r#"[data-theme="dark"], .dark"#.to_owned(),
            forced_light: r#"[data-theme="light"], .light"#.to_owned(),
        }
    } else {
        let scope = format!(r#"[data-palette="{theme}"]"#);
        Selectors {
            light: scope.clone(),
            system_dark: format!(r#"{scope}:not([data-theme="light"]):not(.light)"#),
            forced_dark: format!(r#"{scope}[data-theme="dark"], {scope}.dark"#),
            forced_light: format!(r#"{scope}[data-theme="light"], {scope}.light"#),
        }
    }
}

/// Writes one declaration block of palette tokens.
fn block(
    out: &mut String,
    indent: &str,
    selector: &str,
    prefix: &str,
    mode: &ResolvedMode,
    layer: Layer,
) {
    writeln!(out, "{indent}{selector} {{").expect("string write");
    if layer == Layer::Hex {
        // `color-scheme` makes form controls, scrollbars and the canvas match
        // the theme. Without it a dark page still gets light scrollbars.
        writeln!(out, "{indent}  color-scheme: {};", mode.mode.id()).expect("string write");
    }
    for token in tokens::palette_tokens(mode) {
        writeln!(
            out,
            "{indent}  --{prefix}-{}: {};",
            token.stem(),
            layer.render(token.step)
        )
        .expect("string write");
    }
    for (scale, resolved) in &mode.scales {
        for step in &resolved.steps {
            writeln!(
                out,
                "{indent}  --{prefix}-{scale}-{}: {};",
                step.role,
                layer.render(step)
            )
            .expect("string write");
        }
    }
    writeln!(out, "{indent}}}").expect("string write");
}

/// Writes all four mode blocks for one value layer.
fn mode_layer(
    out: &mut String,
    indent: &str,
    selectors: &Selectors,
    prefix: &str,
    light: &ResolvedMode,
    dark: &ResolvedMode,
    layer: Layer,
) {
    block(out, indent, &selectors.light, prefix, light, layer);
    writeln!(out, "\n{indent}@media (prefers-color-scheme: dark) {{").expect("string write");
    block(
        out,
        &format!("{indent}  "),
        &selectors.system_dark,
        prefix,
        dark,
        layer,
    );
    writeln!(out, "{indent}}}\n").expect("string write");
    block(out, indent, &selectors.forced_dark, prefix, dark, layer);
    writeln!(out).expect("string write");
    block(out, indent, &selectors.forced_light, prefix, light, layer);
}

fn theme_file(
    palette: &Palette,
    theme: &noctua_engine::ResolvedTheme,
    is_default: bool,
    semantic: &tokens::ThemeSplit,
) -> EmittedFile {
    let selectors = selectors(&theme.name, is_default);

    let mut out = header(spec_path(), CommentStyle::Block);
    writeln!(out, "\n/* Theme: {}", theme.name).expect("string write");
    if is_default {
        writeln!(
            out,
            "   Bound to :root, so importing this file is all it takes."
        )
        .expect("string write");
    } else {
        writeln!(
            out,
            "   Apply with data-palette=\"{}\" on any element.",
            theme.name
        )
        .expect("string write");
    }
    writeln!(
        out,
        "\n   Light and dark are all three of: the system preference, a\n\
        \x20  [data-theme] attribute, and a .light / .dark class. Nothing to\n\
        \x20  configure — use whichever suits, on the root or on a subtree. */\n"
    )
    .expect("string write");

    theme_body(&mut out, palette, theme, &selectors, semantic);

    EmittedFile::new(
        format!("css/{}", theme_file_name(&theme.name, is_default)),
        out,
    )
}

/// Every declaration one theme contributes, written under `selectors`.
///
/// Split out of [`theme_file`] so that [`palette_file`] can emit the same
/// declarations under a *different* scope. That is the whole trick behind the
/// self-contained per-palette files, and it is the one thing about them that
/// is easy to get silently wrong — see [`palette_file`].
fn theme_body(
    out: &mut String,
    palette: &Palette,
    theme: &noctua_engine::ResolvedTheme,
    selectors: &Selectors,
    semantic: &tokens::ThemeSplit,
) {
    let prefix = &palette.prefix;
    let light = &theme.modes[0];
    let dark = &theme.modes[1];

    writeln!(out, "/* --- Values: hex, understood everywhere --- */\n").expect("string write");
    mode_layer(out, "", selectors, prefix, light, dark, Layer::Hex);

    writeln!(out, "\n/* --- Values: OKLCH, where supported --- */\n").expect("string write");
    writeln!(out, "@supports (color: oklch(0 0 0)) {{").expect("string write");
    mode_layer(out, "  ", selectors, prefix, light, dark, Layer::Oklch);
    writeln!(out, "}}").expect("string write");

    for (slot, gamut) in palette.gamuts.iter().enumerate().skip(1) {
        writeln!(
            out,
            "\n/* --- Values: {} on displays that can show it ---\n\
            \x20  These are different numbers, not the same color repeated: each\n\
            \x20  token's relative chroma resolved against a wider boundary. */\n",
            gamut.id()
        )
        .expect("string write");
        writeln!(out, "@media (color-gamut: p3) {{").expect("string write");
        writeln!(out, "  @supports (color: oklch(0 0 0)) {{").expect("string write");
        mode_layer(
            out,
            "    ",
            selectors,
            prefix,
            light,
            dark,
            Layer::Wide(slot),
        );
        writeln!(out, "  }}").expect("string write");
        writeln!(out, "}}").expect("string write");
    }

    writeln!(
        out,
        "\n/* --- Aliases and the translucency ladder ---\n\
        \x20  Indirections, so they are written once and follow whichever mode\n\
        \x20  and gamut layer is active.\n\
        \x20\n\
        \x20  The semantic contract is NOT here: every theme resolves it\n\
        \x20  identically, so it is written once in contexts.css and only what\n\
        \x20  this theme overrides appears below. */\n"
    )
    .expect("string write");
    writeln!(out, "{} {{", selectors.light).expect("string write");
    for alias in tokens::numeric_aliases(light) {
        writeln!(
            out,
            "  --{prefix}-{}: var(--{prefix}-{});",
            alias.name, alias.target
        )
        .expect("string write");
    }

    if let Some(overrides) = semantic.per_theme.get(&theme.name) {
        writeln!(out).expect("string write");
        for alias in overrides {
            writeln!(
                out,
                "  --{prefix}-color-{}: var(--{prefix}-{});",
                alias.name, alias.target
            )
            .expect("string write");
        }
    }

    // The translucency ladder. One definition covers both modes and every gamut
    // layer, because `color-mix` resolves the token it references rather than a
    // value frozen here.
    writeln!(out).expect("string write");
    for alpha in tokens::alpha_tokens(palette, light) {
        writeln!(
            out,
            "  --{prefix}-{}: {};",
            alpha.stem(),
            value::color_mix(
                &format!("--{prefix}-{}", alpha.source(&palette.alpha.role)),
                alpha.percentage
            )
        )
        .expect("string write");
    }
    writeln!(out, "}}").expect("string write");
}

/// The semantic contract, which does not vary by theme.
///
/// `--nc-color-rejected: var(--nc-danger-solid)` is the same sentence in every
/// palette — the colour behind it changes, the sentence does not. Written into
/// each theme's stylesheet it was 97 KB of every 225 KB file, thirty-nine times
/// over; here it is written once.
///
/// # Two things make the layering work
///
/// **`:where(:root)` rather than `:root`.** `:where()` contributes zero
/// specificity, so a theme that *does* override a slot wins from its
/// `[data-palette="…"]` block whatever order the two files are linked in.
/// Written as a plain `:root` the two would tie at (0,1,0) and source order
/// would decide, which makes the correctness of an override depend on how a
/// consumer wrote their `<link>` tags.
///
/// **Everything it references is a token, not a value.** A consumer therefore
/// links this once and switches palettes without touching it. The cost is one
/// more file that has to be linked: asking for `--nc-color-rejected` with only
/// a theme stylesheet linked fails *silently*, because CSS drops an undefined
/// custom property. `noctua_check::references` catches that inside this
/// repository; outside it, `index.css` imports everything and is the answer.
fn contexts_file(palette: &Palette, semantic: &tokens::ThemeSplit) -> EmittedFile {
    let mut out = header(spec_path(), CommentStyle::Block);
    writeln!(
        out,
        "\n/* The semantic contract: the names an application codes against.\n\
        \x20\n\
        \x20  Every one of these is an indirection onto a palette token, so this\n\
        \x20  file is the same for every theme and is emitted once. Link it\n\
        \x20  alongside ramp.css and a theme, or import index.css and get all\n\
        \x20  three.\n\
        \x20\n\
        \x20  :where() so that a theme overriding a slot always wins, whatever\n\
        \x20  order the stylesheets are linked in. */\n"
    )
    .expect("string write");

    contexts_body(&mut out, palette, semantic);

    EmittedFile::new("css/contexts.css", out)
}

/// The shared semantic layer, without a file around it.
fn contexts_body(out: &mut String, palette: &Palette, semantic: &tokens::ThemeSplit) {
    let prefix = &palette.prefix;
    writeln!(out, ":where(:root) {{").expect("string write");
    for alias in &semantic.shared {
        writeln!(
            out,
            "  --{prefix}-color-{}: var(--{prefix}-{});",
            alias.name, alias.target
        )
        .expect("string write");
    }
    writeln!(out, "}}").expect("string write");
}

/// The dense neutral ramp, which does not vary by theme or mode.
fn ramp_file(palette: &Palette) -> EmittedFile {
    let prefix = &palette.prefix;
    let mut out = header(spec_path(), CommentStyle::Block);
    writeln!(
        out,
        "\n/* The dense neutral ramp: one resource both modes draw from, so\n\
        \x20  --{prefix}-gray-4 is one color rather than two. */\n"
    )
    .expect("string write");

    ramp_body(&mut out, palette);

    EmittedFile::new("css/ramp.css", out)
}

/// The neutral ramp's declarations, without a file around them.
fn ramp_body(out: &mut String, palette: &Palette) {
    let prefix = &palette.prefix;
    let layers: Vec<(String, Layer)> = std::iter::once((String::new(), Layer::Hex))
        .chain(std::iter::once((
            "@supports (color: oklch(0 0 0))".to_owned(),
            Layer::Oklch,
        )))
        .chain(palette.gamuts.iter().enumerate().skip(1).map(|(slot, _)| {
            (
                "@media (color-gamut: p3) { @supports (color: oklch(0 0 0)) }".to_owned(),
                Layer::Wide(slot),
            )
        }))
        .collect();

    for (wrapper, layer) in layers {
        let indent = if wrapper.is_empty() { "" } else { "  " };
        if wrapper.contains("@media") {
            writeln!(out, "@media (color-gamut: p3) {{").expect("string write");
            writeln!(out, "  @supports (color: oklch(0 0 0)) {{").expect("string write");
        } else if !wrapper.is_empty() {
            writeln!(out, "{wrapper} {{").expect("string write");
        }
        let indent = if wrapper.contains("@media") {
            "    "
        } else {
            indent
        };

        writeln!(out, "{indent}:root {{").expect("string write");
        for (ramp, steps) in &palette.neutral_ramps {
            for step in steps {
                writeln!(
                    out,
                    "{indent}  --{prefix}-{ramp}-{}: {};",
                    step.index,
                    layer.render(step)
                )
                .expect("string write");
            }
        }
        writeln!(out, "{indent}}}").expect("string write");

        if wrapper.contains("@media") {
            writeln!(out, "  }}").expect("string write");
            writeln!(out, "}}").expect("string write");
        } else if !wrapper.is_empty() {
            writeln!(out, "}}").expect("string write");
        }
        writeln!(out).expect("string write");
    }
}

/// One palette, complete and alone: `css/palette/<theme>.css`.
///
/// This is the file a person who has picked a palette should link, and nothing
/// else. `index.css` carries all thirty-nine — 5.12 MB, 674 KB over the wire —
/// because it has to serve someone who wants to switch at runtime. Someone who
/// has decided on `blue-vivid` was paying for the other thirty-eight.
///
/// It is the three shared pieces concatenated: the neutral ramp, the semantic
/// contract, and this one theme. That is safe because they declare disjoint
/// names, and where they *could* collide `:where(:root)`'s zero specificity
/// settles it by specificity rather than by source order.
///
/// # The scope, which is the whole trap
///
/// A theme that is not the default is normally written under
/// `[data-palette="blue-vivid"]`, because in `index.css` it is one of
/// thirty-nine and has to be asked for. Concatenated naively into a file of its
/// own that selector matches nothing, so the file defines **no colour at all**
/// — and CSS drops an undefined custom property without a word, so the page
/// renders unstyled and the console stays empty.
///
/// So every per-palette file emits at `:root`, exactly as the default theme's
/// does. `a_palette_file_defines_the_contract_at_root` is the guard, and it
/// exists because this failure is silent rather than loud.
fn palette_file(
    palette: &Palette,
    theme: &noctua_engine::ResolvedTheme,
    semantic: &tokens::ThemeSplit,
) -> EmittedFile {
    // `true` — the default theme's selectors — for *every* palette. See above.
    let selectors = selectors(&theme.name, true);

    let mut out = header(spec_path(), CommentStyle::Block);
    writeln!(
        out,
        "\n/* The {} palette, complete and self-contained.\n\
        \x20\n\
        \x20  One <link> and you are done: this carries the neutral ramp, the\n\
        \x20  semantic contract and this palette's values. Nothing else to\n\
        \x20  import, and none of the other {} palettes to download.\n\
        \x20\n\
        \x20  Bound to :root, so it applies to the whole page. Light and dark\n\
        \x20  both work from this file alone — the system preference, a\n\
        \x20  [data-theme] attribute, or a .light / .dark class.\n\
        \x20\n\
        \x20  Want to switch palettes at runtime instead? Import index.css. */\n",
        theme.name,
        palette.themes.len() - 1
    )
    .expect("string write");

    writeln!(out, "/* === The neutral ramp === */\n").expect("string write");
    ramp_body(&mut out, palette);

    writeln!(out, "\n/* === The semantic contract === */\n").expect("string write");
    contexts_body(&mut out, palette, semantic);

    writeln!(out, "\n\n/* === The {} palette === */\n", theme.name).expect("string write");
    theme_body(&mut out, palette, theme, &selectors, semantic);

    EmittedFile::new(format!("css/palette/{}.css", theme.name), out)
}

fn index_file(palette: &Palette) -> EmittedFile {
    let mut out = header(spec_path(), CommentStyle::Block);
    writeln!(
        out,
        "\n/* Everything: the neutral ramp, the semantic contract, and every\n\
        \x20  theme.\n\
        \x20\n\
        \x20  For one theme, import ramp.css, contexts.css and {}.css —\n\
        \x20  all three. A theme file alone defines no --{}-gray-* and no\n\
        \x20  --{}-color-*, and CSS drops an undefined custom property\n\
        \x20  without saying so. */\n",
        palette.themes[0].name, palette.prefix, palette.prefix
    )
    .expect("string write");

    writeln!(out, "@import \"./ramp.css\";").expect("string write");
    writeln!(out, "@import \"./contexts.css\";").expect("string write");
    for (index, theme) in palette.themes.iter().enumerate() {
        let file = theme_file_name(&theme.name, index == 0);
        writeln!(out, "@import \"./{file}\";").expect("string write");
    }
    EmittedFile::new("css/index.css", out)
}

#[cfg(test)]
mod tests {
    use noctua_engine::build;

    use super::*;

    fn shipped() -> Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        build(&noctua_spec::load(path).expect("valid spec")).expect("builds")
    }

    fn emitted() -> Vec<EmittedFile> {
        Css.emit(&shipped())
    }

    fn file(name: &str) -> String {
        emitted()
            .into_iter()
            .find(|f| f.path == name)
            .unwrap_or_else(|| panic!("{name} should be emitted"))
            .contents
    }

    #[test]
    fn one_file_per_theme_plus_the_two_shared_ones_and_an_index() {
        let palette = shipped();
        let files = Css.emit(&palette);
        // The ramp, the semantic contract, an index, and *two* files per theme:
        // the layered one index.css imports, and the self-contained one.
        assert_eq!(files.len(), palette.themes.len() * 2 + 3);
        for shared in ["css/ramp.css", "css/contexts.css", "css/index.css"] {
            assert!(files.iter().any(|f| f.path == shared), "{shared} missing");
        }
        assert!(files.iter().any(|f| f.path == "css/ochre-balanced.css"));
        assert!(
            files
                .iter()
                .any(|f| f.path == "css/palette/ochre-balanced.css")
        );
    }

    /// Every palette gets a self-contained file, named for the palette.
    #[test]
    fn every_theme_has_a_self_contained_file() {
        let palette = shipped();
        let files = Css.emit(&palette);
        for theme in &palette.themes {
            let path = format!("css/palette/{}.css", theme.name);
            assert!(
                files.iter().any(|f| f.path == path),
                "{path} missing — a palette with no small file to link"
            );
        }
    }

    /// **The guard this feature exists for.**
    ///
    /// A non-default theme's declarations are normally scoped to
    /// `[data-palette="…"]`. Concatenated into a standalone file that selector
    /// matches nothing, and the file silently defines no colour: CSS drops an
    /// undefined custom property without an error, so the page renders
    /// unstyled and the console stays clean. There is no louder symptom to
    /// catch this by, so it is caught here.
    ///
    /// Checked on a theme that is **not** the default, because the default
    /// would pass this test even if the bug were present.
    #[test]
    fn a_palette_file_defines_the_contract_at_root() {
        let palette = shipped();
        let not_default = &palette.themes[1].name;
        assert_ne!(not_default, &palette.themes[0].name);
        let css = file(&format!("css/palette/{not_default}.css"));

        assert!(
            !css.contains("[data-palette="),
            "{not_default} is scoped to an attribute nothing sets — it would define nothing"
        );

        // The three pieces, each proven by a token only it declares.
        assert!(css.contains("--nc-gray-1:"), "the neutral ramp is missing");
        assert!(
            css.contains("--nc-color-surface:"),
            "the semantic contract is missing"
        );
        assert!(
            css.contains("--nc-neutral-bg-app:"),
            "the palette's own values are missing"
        );
    }

    /// Light *and* dark must both work from the one file.
    ///
    /// All three switches, because a consumer picks one and the other two
    /// failing is invisible until someone else picks differently.
    #[test]
    fn a_palette_file_carries_both_modes_and_every_switch() {
        let css = file("css/palette/azure-vivid.css");
        assert!(css.contains("@media (prefers-color-scheme: dark)"));
        assert!(css.contains(r#"[data-theme="dark"]"#));
        assert!(css.contains(".dark {") || css.contains(", .dark {"));
        assert!(css.contains(r#"[data-theme="light"]"#));
        assert!(css.contains("color-scheme: dark;"));
        assert!(css.contains("color-scheme: light;"));
    }

    /// The wide-gamut upgrade survives the concatenation.
    ///
    /// It is the layer most likely to be lost, because it is the only one
    /// nested two deep — and losing it costs nothing visible on an sRGB
    /// display, which is what most development happens on.
    #[test]
    fn a_palette_file_keeps_the_wide_gamut_layer() {
        let css = file("css/palette/jade-sober.css");
        assert!(css.contains("@media (color-gamut: p3)"));
        assert!(css.contains("@supports (color: oklch(0 0 0))"));
        assert!(
            css.contains("color-mix("),
            "the translucency ladder is missing"
        );
    }

    /// A self-contained file must declare every token the contract points at.
    ///
    /// This is the same class of bug as the scope trap and just as silent: a
    /// `var()` naming a token nothing declares resolves to nothing. Rather
    /// than spot-check names, take every `var(--nc-…)` the file references and
    /// require the file itself to declare it.
    #[test]
    fn a_palette_file_references_nothing_it_does_not_declare() {
        let css = file("css/palette/rose-balanced.css");

        let declared: std::collections::HashSet<&str> = css
            .match_indices("  --nc-")
            .filter_map(|(at, _)| css[at + 2..].split(':').next())
            .map(str::trim)
            .collect();

        let mut dangling: Vec<&str> = css
            .match_indices("var(--nc-")
            .filter_map(|(at, _)| css[at + 4..].split(')').next())
            .map(str::trim)
            .filter(|name| !declared.contains(name))
            .collect();
        dangling.sort_unstable();
        dangling.dedup();

        assert!(
            dangling.is_empty(),
            "referenced but never declared, so they resolve to nothing: {dangling:?}"
        );
    }

    #[test]
    fn every_file_carries_a_generated_header() {
        for file in emitted() {
            assert!(
                file.contents.starts_with("/*!"),
                "{} does not start with a header",
                file.path
            );
            assert!(file.contents.contains("do not edit"), "{}", file.path);
            assert!(file.contents.contains(crate::REGENERATE), "{}", file.path);
        }
    }

    /// The default theme drops the prefix; every other theme keeps it. A
    /// rename therefore moves the file, and every reference in the repository
    /// has to move with it — `index.css` is the name that does not.
    #[test]
    fn a_theme_file_is_named_after_its_theme() {
        assert_eq!(theme_file_name("balanced", true), "balanced.css");
        assert_eq!(theme_file_name("anything-else", true), "anything-else.css");
        assert_eq!(theme_file_name("vivid", false), "theme-vivid.css");
    }

    #[test]
    fn the_default_theme_binds_root_so_importing_is_enough() {
        let css = file("css/ochre-balanced.css");
        assert!(
            css.contains(":root {"),
            "the default theme must apply without an attribute"
        );
        assert!(!css.contains("data-palette=\"noctua\""));
    }

    #[test]
    fn other_themes_are_scoped_to_an_attribute() {
        let css = file("css/theme-ochre-vivid.css");
        assert!(css.contains(r#"[data-palette="ochre-vivid"]"#));
    }

    #[test]
    fn all_three_light_dark_strategies_are_present() {
        let css = file("css/ochre-balanced.css");
        assert!(
            css.contains("@media (prefers-color-scheme: dark)"),
            "system preference"
        );
        assert!(css.contains(r#"[data-theme="dark"]"#), "data attribute");
        assert!(css.contains(".dark {") || css.contains(".dark,"), "class");
        assert!(css.contains(r#"[data-theme="light"]"#), "forced light");
    }

    /// The combination that breaks if the guards are missing.
    #[test]
    fn a_forced_light_theme_survives_a_dark_system_preference() {
        let css = file("css/ochre-balanced.css");
        assert!(
            css.contains(r#":not([data-theme="light"]):not(.light)"#),
            "the system-dark block must exclude explicitly-light pages"
        );
    }

    #[test]
    fn colour_scheme_is_declared_so_native_controls_follow() {
        let css = file("css/ochre-balanced.css");
        assert!(css.contains("color-scheme: light;"));
        assert!(css.contains("color-scheme: dark;"));
    }

    #[test]
    fn hex_comes_first_and_oklch_upgrades_it() {
        let css = file("css/ochre-balanced.css");
        let hex_at = css.find("--nc-accent-solid: #").expect("a hex value");
        let oklch_at = css
            .find("--nc-accent-solid: oklch(")
            .expect("an oklch value");
        assert!(
            hex_at < oklch_at,
            "the fallback must come before the upgrade"
        );
        assert!(css.contains("@supports (color: oklch(0 0 0))"));
    }

    /// The payoff of relative chroma, checked in the output.
    #[test]
    fn the_wide_gamut_layer_carries_different_numbers() {
        let css = file("css/ochre-balanced.css");
        assert!(css.contains("@media (color-gamut: p3)"));

        let values: Vec<&str> = css
            .match_indices("--nc-accent-solid: oklch(")
            .map(|(at, _)| {
                let rest = &css[at..];
                &rest[..rest.find(';').expect("terminated")]
            })
            .collect();
        assert!(values.len() >= 2, "expected a base and a wide value");
        assert_ne!(
            values[0],
            values[values.len() - 1],
            "the P3 layer repeated the sRGB value, which would make it pointless"
        );
    }

    #[test]
    fn aliases_and_semantics_are_indirections_written_once() {
        let theme = file("css/ochre-balanced.css");
        let contexts = file("css/contexts.css");

        // Numeric aliases stay with the theme: they name that theme's own
        // ramp, and there are a hundred and forty-four of them however many
        // contexts the spec grows.
        assert!(theme.contains("--nc-accent-9: var(--nc-accent-solid);"));

        // The semantic contract does not. Every theme resolved it identically
        // and every theme therefore carried the same 97 KB.
        assert!(contexts.contains("--nc-color-accent: var(--nc-accent-solid);"));
        assert!(contexts.contains("--nc-color-surface: var(--nc-neutral-bg-app);"));
        assert_eq!(contexts.matches("--nc-color-accent: var(").count(), 1);
        assert!(
            !theme.contains("--nc-color-accent: var("),
            "the theme file still carries a slot no theme overrides"
        );

        // Zero specificity, so a theme that *does* override a slot wins from
        // its own block whatever order the two files are linked in.
        assert!(
            contexts.contains(":where(:root) {"),
            "the shared contract is not at zero specificity, so an override \
             would depend on link order"
        );

        // The unprefixed namespace belongs to Tailwind, and neither of these
        // is the Tailwind target.
        for css in [&theme, &contexts] {
            assert!(
                !css.contains("\n  --color-"),
                "the plain layer must not define Tailwind's theme namespace"
            );
        }
    }

    /// Every theme in the shipped spec resolves the contract identically, so
    /// nothing should be written per theme. A theme that *did* override a slot
    /// must still get its own line — that is the whole reason the split is
    /// derived from the resolved palette rather than assumed.
    #[test]
    fn the_shared_contract_covers_every_shipped_theme() {
        let palette = shipped();
        let split = tokens::semantic_layer(&palette);
        assert!(
            split.per_theme.is_empty(),
            "no shipped theme overrides a slot, so nothing should be per-theme: {:?}",
            split.per_theme.keys().collect::<Vec<_>>()
        );
        assert!(
            split.shared.len() > 1500,
            "only {} shared tokens",
            split.shared.len()
        );
    }

    /// The ladder is one definition for both modes and every gamut layer,
    /// because `color-mix` resolves the token it references rather than a value
    /// frozen at emit time.
    #[test]
    fn the_translucency_ladder_follows_the_token_it_washes() {
        let css = file("css/ochre-balanced.css");
        assert!(css.contains(
            "--nc-neutral-a1: color-mix(in oklab, var(--nc-neutral-text-strong) 2%, transparent);"
        ));
        assert_eq!(
            css.matches("--nc-neutral-a1:").count(),
            1,
            "one definition, not one per mode"
        );
        // Mixing with `transparent` in `oklab` is premultiplied, which is what
        // makes this the token at that alpha rather than a blend toward grey.
        for line in css.lines().filter(|l| l.contains("-a1: color-mix")) {
            assert!(line.contains("in oklab"), "{line}");
            assert!(line.contains(", transparent)"), "{line}");
        }
    }

    #[test]
    fn the_ramp_is_emitted_once_for_both_modes() {
        let css = file("css/ramp.css");
        assert!(css.contains("--nc-gray-1:"));
        assert!(css.contains("--nc-gray-24:"));
        assert!(
            !css.contains("prefers-color-scheme"),
            "the ramp does not vary by mode"
        );
    }

    #[test]
    fn the_index_imports_everything() {
        let css = file("css/index.css");
        assert!(css.contains(r#"@import "./ramp.css";"#));
        assert!(css.contains(r#"@import "./ochre-balanced.css";"#));
        assert!(css.contains(r#"@import "./theme-ochre-vivid.css";"#));
    }

    #[test]
    fn braces_balance_in_every_file() {
        for file in emitted() {
            let opens = file.contents.matches('{').count();
            let closes = file.contents.matches('}').count();
            assert_eq!(opens, closes, "{} has unbalanced braces", file.path);
        }
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(Css.emit(&shipped()), Css.emit(&shipped()));
    }
}
