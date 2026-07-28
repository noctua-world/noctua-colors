//! The documentation site.
//!
//! A static generator. It reads `dist/` — the same artifacts any other project
//! consumes — and writes plain HTML, CSS and JavaScript that deploy anywhere a
//! file server runs.
//!
//! # Why it reads `dist/` rather than calling the engine
//!
//! Because a site that recomputed its own colors would be a second
//! implementation, agreeing with the first only by luck. Reading the emitted
//! JSON means the site is wrong exactly when the output is wrong — which is
//! the only useful place for it to be wrong.
//!
//! Every swatch, chart and code sample on the site therefore comes from
//! `dist/json/palette.json`, and every color it *paints with* comes from
//! `dist/css/`. There is no third source.

mod controls;
pub mod data;
pub mod i18n;
pub mod page;
mod playground;
pub mod sections;

use std::path::Path;

pub use data::Palette;

/// The id the inline bootstrap stamps on the stylesheet it injects.
///
/// Exposed so a test can check that `site.js` looks for the same one — without
/// it, every reload with a non-default palette appends a duplicate sheet.
#[must_use]
pub fn bootstrap_sheet_id() -> &'static str {
    page::BOOTSTRAP_SHEET_ID
}

/// The global the inline bootstrap parks its palette-JSON fetch on.
///
/// Exposed for the same reason as [`bootstrap_sheet_id`]: `site.js` reads it,
/// and a rename on one side that nothing compares would silently cost the
/// prefetch — which fails as a slower page rather than as an error.
#[must_use]
pub fn theme_fetch_global() -> &'static str {
    page::THEME_FETCH
}

/// A file the generator produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    /// Path relative to the site root, using forward slashes.
    pub path: String,
    /// Complete file contents.
    pub contents: String,
}

/// Reads the emitted palette.
///
/// # Errors
///
/// A missing or malformed `dist/`, which means `cargo xtask build` has not run.
pub fn load(dist: &Path) -> Result<Palette, String> {
    let json = std::fs::read_to_string(dist.join("json/palette.json")).map_err(|e| {
        format!("could not read dist/json/palette.json ({e}). Run `cargo xtask build` first.")
    })?;
    Palette::parse(&json)
}

/// Renders the whole site from a built `dist/`.
///
/// # Errors
///
/// A missing or malformed `dist/`, which means `cargo xtask build` has not run.
pub fn render(dist: &Path) -> Result<Vec<Output>, String> {
    let palette = load(dist)?;

    // Both locales, rendered in full. The alternative — one page that
    // rewrites itself on load — shows the wrong language first and needs
    // script to show any language at all.
    let mut outputs = Vec::new();
    for locale in i18n::Locale::all() {
        outputs.push(Output {
            path: locale.page("index"),
            contents: page::render(&palette, locale),
        });
        // A route of its own: the playground costs a WebAssembly module, and
        // charging every reader for it would be the wrong trade.
        outputs.push(Output {
            path: locale.page("playground"),
            contents: playground::render(&palette, locale),
        });
    }
    Ok(outputs)
}

/// Static files copied verbatim into the site output.
///
/// Returned as `(source, destination)` pairs relative to `docs-site/` and the
/// site root. Fonts are here rather than generated because they are vendored
/// by hand; see `AGENTS.md`.
#[must_use]
pub fn assets() -> Vec<(&'static str, &'static str)> {
    vec![
        ("css/site.css", "css/site.css"),
        ("css/motion.css", "css/motion.css"),
        ("js/site.js", "js/site.js"),
        ("js/playground.js", "js/playground.js"),
        ("assets/fonts/fonts.css", "assets/fonts/fonts.css"),
        (
            "assets/fonts/NoctuaIosevka-Regular.woff2",
            "assets/fonts/NoctuaIosevka-Regular.woff2",
        ),
        (
            "assets/fonts/NoctuaIosevka-Bold.woff2",
            "assets/fonts/NoctuaIosevka-Bold.woff2",
        ),
        (
            "assets/fonts/NoctuaIosevka-Italic.woff2",
            "assets/fonts/NoctuaIosevka-Italic.woff2",
        ),
        ("assets/fonts/OFL.md", "assets/fonts/OFL.md"),
        ("assets/fonts/ATTRIBUTION.md", "assets/fonts/ATTRIBUTION.md"),
    ]
}

/// Token files copied out of `dist/` into the site.
///
/// The site links these exactly as a consumer would, which is what stops it
/// from drifting.
///
/// Derived from the palette rather than listed. A hardcoded list was correct
/// for exactly as long as nobody added a theme: the new theme's stylesheet was
/// emitted into `dist/`, never copied to the site, and `index.css` imported it
/// anyway — a 404 for every visitor and a theme in the picker with no colors
/// behind it. Nothing failed, because the test that guarded the list built its
/// expectation from the same list.
#[must_use]
pub fn token_files(palette: &Palette) -> Vec<String> {
    let mut files = vec![
        "css/index.css".to_owned(),
        "css/ramp.css".to_owned(),
        "css/contexts.css".to_owned(),
        "json/axes.json".to_owned(),
    ];

    for (index, theme) in palette.theme_names().iter().enumerate() {
        // The first theme is the default and takes the bare name; the rest are
        // prefixed. This mirrors `noctua_emit::css`, which owns the rule.
        files.push(if index == 0 {
            format!("css/{theme}.css")
        } else {
            format!("css/theme-{theme}.css")
        });
        files.push(format!("json/themes/{theme}.json"));
    }

    files
}
