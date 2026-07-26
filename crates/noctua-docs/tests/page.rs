//! Structural checks on the rendered page.
//!
//! A docs site is the one artifact here with no compiler to catch its
//! mistakes: a mistyped `aria-controls` or a dead asset reference compiles
//! perfectly and ships broken. These stand in for that compiler.

use std::collections::HashSet;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn page() -> String {
    rendered("index.html")
}

/// Every rendered page, so a structural check covers the translations too.
fn all_pages() -> Vec<(String, String)> {
    let dist = root().join("dist");
    noctua_docs::render(&dist)
        .expect("dist/ must exist — run `cargo xtask build`")
        .into_iter()
        .map(|o| (o.path, o.contents))
        .collect()
}

fn rendered(path: &str) -> String {
    let dist = root().join("dist");
    noctua_docs::render(&dist)
        .expect("dist/ must exist — run `cargo xtask build`")
        .into_iter()
        .find(|o| o.path == path)
        .unwrap_or_else(|| panic!("the generator does not render {path}"))
        .contents
}

#[test]
fn the_page_renders_from_dist() {
    let html = page();
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("noctua-colors"));
    assert!(html.trim_end().ends_with("</html>"));
}

/// Every element opened must be closed, or the browser's error recovery
/// decides the layout instead of the stylesheet.
#[test]
fn tags_are_balanced() {
    for (path, html) in all_pages() {
        assert_balanced(&path, &html);
    }
}

fn assert_balanced(path: &str, html: &str) {
    const VOID: [&str; 8] = ["meta", "link", "br", "hr", "img", "input", "source", "col"];

    let mut stack: Vec<String> = Vec::new();
    let mut rest = html;

    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find('>') else { break };
        let tag = &rest[..end];
        rest = &rest[end + 1..];

        if tag.starts_with('!') || tag.starts_with('?') || tag.ends_with('/') {
            continue;
        }

        if let Some(name) = tag.strip_prefix('/') {
            let name = name.trim().to_lowercase();
            let popped = stack.pop();
            assert_eq!(
                popped.as_deref(),
                Some(name.as_str()),
                "{path}: closing </{name}> does not match the open tag"
            );
        } else {
            let name = tag.split_whitespace().next().unwrap_or("").to_lowercase();
            if !VOID.contains(&name.as_str()) && !name.is_empty() {
                stack.push(name);
            }
        }
    }

    assert!(stack.is_empty(), "{path}: unclosed elements: {stack:?}");
}

/// Every asset the page asks for must be something the generator ships.
#[test]
fn no_reference_points_at_a_missing_file() {
    let html = page();
    // Derived from what the generator actually produces, so adding a page or
    // an asset never needs this list edited to match.
    let palette = noctua_docs::load(&root().join("dist")).expect("dist/ must exist");
    let shipped: HashSet<String> = noctua_docs::assets()
        .iter()
        .map(|(_, to)| (*to).to_owned())
        .chain(
            noctua_docs::token_files(&palette)
                .iter()
                .map(|f| format!("tokens/{f}")),
        )
        .chain(rendered_pages())
        .collect();

    // Integration samples contain `href="dist/css/balanced.css"` as text. That
    // is documentation, not a reference the site has to resolve.
    let outside_code = strip_code_blocks(&html);

    let mut referenced = Vec::new();
    for attribute in ["href=\"", "src=\""] {
        let mut rest = outside_code.as_str();
        while let Some(at) = rest.find(attribute) {
            rest = &rest[at + attribute.len()..];
            let Some(end) = rest.find('"') else { break };
            let value = &rest[..end];
            if !value.starts_with('#') && !value.starts_with("http") {
                referenced.push(value.to_owned());
            }
        }
    }

    assert!(!referenced.is_empty(), "the page references nothing at all");
    for reference in referenced {
        assert!(
            shipped.contains(&reference),
            "the page references `{reference}`, which the generator does not ship"
        );
    }
}

/// Every in-page anchor must land somewhere.
#[test]
fn every_anchor_has_a_target() {
    let html = page();
    let ids = collect(&html, "id=\"");

    let mut rest = html.as_str();
    while let Some(at) = rest.find("href=\"#") {
        rest = &rest[at + 7..];
        let Some(end) = rest.find('"') else { break };
        let target = &rest[..end];
        assert!(
            ids.contains(target),
            "href=\"#{target}\" points at an element that does not exist"
        );
    }
}

/// The ARIA tab pattern breaks silently when the wiring is wrong.
#[test]
fn the_tabs_are_wired_correctly() {
    let html = page();
    let ids = collect(&html, "id=\"");

    for controls in collect(&html, "aria-controls=\"") {
        assert!(
            ids.contains(&controls),
            "aria-controls=\"{controls}\" has no panel"
        );
    }
    for labelled in collect(&html, "aria-labelledby=\"") {
        assert!(
            ids.contains(&labelled),
            "aria-labelledby=\"{labelled}\" has no label"
        );
    }
}

/// The accessibility floor. Each of these is invisible until somebody
/// navigating by keyboard or screen reader hits it.
#[test]
fn the_page_meets_its_accessibility_floor() {
    let html = page();

    assert!(
        html.contains("<html lang=\"en\""),
        "the document needs a language"
    );
    assert!(
        html.contains("name=\"viewport\""),
        "mobile needs a viewport meta"
    );
    assert!(
        html.contains("class=\"skip-link\""),
        "keyboard users need a skip link"
    );
    assert!(
        html.contains("<main id=\"main\">"),
        "there must be a main landmark"
    );
    assert!(
        html.contains("aria-label=\"Primary\""),
        "the nav needs a name"
    );

    assert_eq!(
        html.matches("<h1").count(),
        1,
        "there must be exactly one h1"
    );

    // The palette selects lost their visible labels to icons, so their
    // accessible names come from `aria-label` alone — if that goes, they
    // become unnamed comboboxes.
    for id in ["id=\"accent-select\"", "id=\"saturation-select\""] {
        let at = html.find(id).unwrap_or_else(|| panic!("{id} is missing"));
        let window = &html[at.saturating_sub(240)..(at + 240).min(html.len())];
        assert!(window.contains("aria-label"), "{id} has no accessible name");
    }

    // Every icon-only button needs text a screen reader can reach.
    assert_eq!(
        html.matches("class=\"mode-option\"").count(),
        3,
        "the appearance control must offer light, dark and system"
    );
}

/// Every swatch must carry the data the detail panel reads. A missing
/// attribute renders as `undefined` rather than failing.
#[test]
fn every_swatch_carries_its_own_data() {
    let html = page();
    let swatches = html.matches("class=\"swatch\"").count();
    assert!(swatches > 100, "only {swatches} swatches rendered");

    // Per element rather than by counting occurrences: `data-ink` is also
    // written by the ordered-scale stops, so a total would not tell a swatch
    // that lost an attribute from a scale that gained one.
    for (index, tag) in html.split("class=\"swatch\"").skip(1).enumerate() {
        let tag = tag.split('>').next().expect("an open tag");
        for attribute in [
            "data-stem=",
            "data-hex=",
            "data-css=",
            "data-l=",
            "data-c=",
            "data-h=",
            "data-cr=",
            "data-headroom=",
            "data-ink=",
        ] {
            assert!(
                tag.contains(attribute),
                "swatch {index} has no {attribute}: {tag}"
            );
        }
    }
}

/// The site paints with tokens, never with colors of its own.
#[test]
fn the_site_sources_contain_no_color_literals() {
    // The rendered page is full of colors — that is the palette being shown.
    // The *sources* must have none, which is what makes the page a consumer
    // of the system rather than a second opinion about it.
    for relative in [
        "docs-site/css/site.css",
        "docs-site/css/motion.css",
        "docs-site/js/site.js",
        "docs-site/js/playground.js",
    ] {
        let text = std::fs::read_to_string(root().join(relative)).expect(relative);
        for line in text.lines() {
            // The same marker the source gate honours. Two checks over the
            // same files with different rules is the drift this project
            // exists to prevent, and a comment explaining why `rgb(` must not
            // be parsed is not a color literal.
            if line.contains(noctua_check_marker()) {
                continue;
            }
            assert!(
                !contains_color_literal(line),
                "{relative} contains a color literal: {line}"
            );
        }
    }
}

/// Motion must be opt-out, and only compositable properties animated.
#[test]
fn motion_is_safe_by_default() {
    let motion =
        std::fs::read_to_string(root().join("docs-site/css/motion.css")).expect("motion.css");

    assert!(
        motion.contains("@media (prefers-reduced-motion: reduce)"),
        "every animation must be opt-out"
    );

    // Animating a layout property makes a long page of swatches stutter.
    for property in [
        "transition: height",
        "transition: width",
        "transition: top",
        "transition: margin",
    ] {
        assert!(!motion.contains(property), "{property} triggers layout");
    }
}

/// Touch targets and a reflowing layout, checked in the stylesheet.
///
/// Not a substitute for looking at it on a phone, but it catches the
/// regressions that are easy to introduce and hard to notice on a desktop.
#[test]
fn the_layout_is_built_for_a_narrow_screen() {
    let css = std::fs::read_to_string(root().join("docs-site/css/site.css")).expect("site.css");

    // Interactive controls need 44px of target; 2.75rem is that at the
    // default root size.
    assert!(
        css.contains("min-height: 2.75rem"),
        "touch targets are undersized"
    );

    // No container may be pinned wider than a narrow phone. Small fixed
    // widths are fine and necessary — `width: 1px` is the standard
    // screen-reader-only pattern — so the threshold is what would actually
    // overflow a 360px viewport.
    for line in css.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("width:")
            && let Some(pixels) = value.trim().trim_end_matches(';').strip_suffix("px")
            && let Ok(pixels) = pixels.trim().parse::<f64>()
        {
            assert!(
                pixels < 320.0,
                "a {pixels}px fixed width will overflow a narrow screen: {trimmed}"
            );
        }
    }

    // Wide content scrolls inside its own container, never the body.
    assert!(
        css.contains("overflow-x: auto"),
        "wide tables need their own scroller"
    );
}

/// Both languages, rendered in full, each a complete document.
#[test]
fn every_locale_renders_every_page() {
    let dist = root().join("dist");
    let outputs = noctua_docs::render(&dist).expect("dist/ must exist");
    let paths: Vec<&str> = outputs.iter().map(|o| o.path.as_str()).collect();

    for expected in [
        "index.html",
        "index.pt.html",
        "playground.html",
        "playground.pt.html",
    ] {
        assert!(
            paths.contains(&expected),
            "{expected} is not rendered: {paths:?}"
        );
    }

    for output in &outputs {
        assert!(
            output.contents.starts_with("<!DOCTYPE html>"),
            "{}",
            output.path
        );
        assert!(
            output.contents.trim_end().ends_with("</html>"),
            "{}",
            output.path
        );
    }
}

/// A page whose `lang` disagrees with its content is worse than an untranslated
/// one: a screen reader will pronounce Portuguese with English phonetics.
#[test]
fn each_page_declares_the_language_it_is_written_in() {
    assert!(rendered("index.html").contains("<html lang=\"en\""));
    assert!(rendered("index.pt.html").contains("<html lang=\"pt-BR\""));
    assert!(rendered("playground.pt.html").contains("<html lang=\"pt-BR\""));
}

/// The translation has to reach the content, not just the chrome.
#[test]
fn the_portuguese_page_is_actually_in_portuguese() {
    let pt = rendered("index.pt.html");
    let en = rendered("index.html");

    for phrase in ["A paleta", "Contraste", "Em contexto", "Como funciona"] {
        assert!(
            pt.contains(phrase),
            "the Portuguese page is missing {phrase:?}"
        );
    }
    // And the English headings must not have leaked through.
    for phrase in [">The palette<", ">In context<", ">How it works<"] {
        assert!(
            !pt.contains(phrase),
            "untranslated English left in: {phrase:?}"
        );
        assert!(en.contains(phrase), "the English page lost {phrase:?}");
    }
}

/// Each page must offer the other language, and point at the right file.
#[test]
fn every_page_links_to_its_translation() {
    for (page, expected) in [
        ("index.html", "index.pt.html"),
        ("index.pt.html", "index.html"),
        ("playground.html", "playground.pt.html"),
        ("playground.pt.html", "playground.html"),
    ] {
        let html = rendered(page);
        assert!(
            html.contains(&format!("class=\"lang-switch\" href=\"{expected}\"")),
            "{page} does not offer {expected}"
        );
        // The alternates the bootstrap resolves the redirect against.
        assert!(
            html.contains("rel=\"alternate\""),
            "{page} has no alternates"
        );
    }
}

/// Both locales share a directory, because every asset path is relative and a
/// subdirectory would break one of the two.
#[test]
fn the_translation_does_not_move_the_asset_paths() {
    let en = rendered("index.html");
    let pt = rendered("index.pt.html");
    for asset in ["css/site.css", "tokens/css/ramp.css", "js/site.js"] {
        assert!(en.contains(asset), "English lost {asset}");
        assert!(pt.contains(asset), "Portuguese lost {asset}");
    }
    assert!(
        !pt.contains("../css/"),
        "the Portuguese page climbed a directory"
    );
}

/// A link that names its language must open in that language, whatever the
/// recipient happens to have stored. Only the default page may redirect.
#[test]
fn only_the_default_page_may_redirect_to_a_stored_preference() {
    // The opening tag only — the bootstrap script mentions the attribute by
    // name on every page, which a whole-document search would match.
    fn html_tag(page: &str) -> String {
        let html = rendered(page);
        let at = html.find("<html").expect("an html element");
        html[at..at + html[at..].find('>').expect("a closing bracket")].to_owned()
    }

    for page in ["index.html", "playground.html"] {
        assert!(
            html_tag(page).contains("data-default-locale"),
            "{page} should be redirectable"
        );
    }
    for page in ["index.pt.html", "playground.pt.html"] {
        assert!(
            !html_tag(page).contains("data-default-locale"),
            "{page} names its language, so a shared link to it must not redirect"
        );
    }
}

/// The playground is a second page, and every structural mistake the index
/// page can make it can make too.
#[test]
fn the_playground_page_is_structurally_sound() {
    let html = rendered("playground.html");

    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<html lang=\"en\""), "it needs a language");
    assert!(
        html.contains("name=\"viewport\""),
        "mobile needs a viewport"
    );
    assert!(
        html.contains("class=\"skip-link\""),
        "keyboard users need a skip link"
    );
    assert_eq!(html.matches("<h1").count(), 1, "exactly one h1");

    // Its tabs use the same ARIA pattern, and break the same silent way.
    let ids = collect(&html, "id=\"");
    for controls in collect(&html, "aria-controls=\"") {
        assert!(
            ids.contains(&controls),
            "aria-controls=\"{controls}\" has no panel"
        );
    }
    for labelled in collect(&html, "aria-labelledby=\"") {
        assert!(
            ids.contains(&labelled),
            "aria-labelledby=\"{labelled}\" has no label"
        );
    }
}

/// Every element the playground script reaches for must exist in the markup.
///
/// The two are written in different languages and nothing but this test
/// connects them: a renamed id produces a page that loads, shows its loading
/// message, and then silently does nothing.
#[test]
fn the_playground_script_and_its_markup_agree() {
    let html = rendered("playground.html");
    let script =
        std::fs::read_to_string(root().join("docs-site/js/playground.js")).expect("playground.js");

    let ids = collect(&html, "id=\"");
    let mut looked_up = 0usize;
    let mut rest = script.as_str();

    while let Some(at) = rest.find("getElementById(\"") {
        rest = &rest[at + 16..];
        let Some(end) = rest.find('"') else { break };
        let id = &rest[..end];
        // Panel bodies are built by concatenation and checked below.
        if id.starts_with("pg-body-") {
            continue;
        }
        looked_up += 1;
        assert!(
            ids.contains(id),
            "the script reads #{id}, which the page has no element for"
        );
    }

    assert!(
        looked_up > 3,
        "only {looked_up} lookups checked; the scan found nothing"
    );

    // The concatenated ones, spelled out because the scan cannot see them.
    for view in ["ramps", "gates", "emit"] {
        let id = format!("pg-body-{view}");
        assert!(
            ids.contains(&id),
            "the script fills #{id}, which does not exist"
        );
    }
}

/// Every palette's stylesheet must reach the site, or its picker entry paints
/// nothing. The list used to be hardcoded, so a new theme's CSS was emitted to
/// `dist/` and never copied.
#[test]
fn every_theme_ships_a_stylesheet_and_a_palette_file() {
    let palette = noctua_docs::load(&root().join("dist")).expect("dist/ must exist");
    let files = noctua_docs::token_files(&palette);

    for (index, theme) in palette.theme_names().iter().enumerate() {
        let stylesheet = if index == 0 {
            format!("css/{theme}.css")
        } else {
            format!("css/theme-{theme}.css")
        };
        assert!(
            files.contains(&stylesheet),
            "{theme} has no stylesheet copied"
        );
        assert!(
            files.contains(&format!("json/themes/{theme}.json")),
            "{theme} has no palette file copied"
        );

        // And they must actually exist where the site expects them.
        for file in [&stylesheet, &format!("json/themes/{theme}.json")] {
            let path = root().join("docs-site/public/tokens").join(file);
            assert!(path.exists(), "{} was never written", path.display());
        }
    }
}

/// The inline bootstrap and `site.js` both build the href of a palette
/// stylesheet, and both must spell it the same way.
///
/// The bootstrap injects the stored palette's sheet before the first paint —
/// which is the whole fix for the default theme flashing in on every reload —
/// and `site.js` appends it when the picker changes. A divergence would either
/// 404 before paint or append a second copy of a sheet already present.
#[test]
fn the_bootstrap_and_the_script_build_the_same_stylesheet_path() {
    let html = page();
    let script = std::fs::read_to_string(root().join("docs-site/js/site.js")).expect("site.js");

    // Both sides write `tokens/css/theme-<name>.css` by concatenation, so the
    // literal prefix is what has to match.
    let prefix = "\"tokens/css/theme-\" + ";
    assert!(
        script.contains(prefix),
        "site.js no longer builds the theme href by concatenation: {prefix}"
    );
    assert!(
        html.contains("'tokens/css/theme-' + palette + '.css'"),
        "the bootstrap no longer injects the palette stylesheet"
    );

    // And the id the bootstrap stamps has to be the one the script looks for,
    // or every load appends a duplicate sheet.
    let id = noctua_docs::bootstrap_sheet_id();
    assert!(html.contains(id), "the bootstrap does not set its own id");
    assert!(
        script.contains(id),
        "site.js does not know about the injected sheet, so it would duplicate it"
    );

    // The whole point of injecting it, and invisible if it goes missing. A
    // stylesheet inserted by script is *not* render-blocking by default, so
    // without this the page paints in the default palette and swaps — measured
    // in Chrome, `renderBlockingStatus` reads "non-blocking" without the
    // attribute and "blocking" with it.
    assert!(
        html.contains("setAttribute('blocking', 'render')"),
        "the injected stylesheet is not render-blocking, so the flash returns"
    );
}

/// The page renders one palette; the rest are built by `site.js`. Two
/// renderers for one thing only stays honest if they write the same
/// attributes, so a rename in the Rust must fail here rather than render
/// `undefined` in the browser.
#[test]
fn the_two_ramp_renderers_agree() {
    let html = page();
    let script = std::fs::read_to_string(root().join("docs-site/js/site.js")).expect("site.js");

    let mut checked = 0usize;
    for attribute in [
        "data-stem",
        "data-hex",
        "data-css",
        "data-l",
        "data-c",
        "data-h",
        "data-cr",
        "data-headroom",
        "data-ink",
    ] {
        assert!(
            html.contains(&format!("{attribute}=")),
            "the page lost {attribute}"
        );
        // `dataset.foo` is how the script writes them.
        let property = attribute.trim_start_matches("data-");
        assert!(
            script.contains(&format!("dataset.{property}")),
            "site.js never writes {attribute}, so a client-rendered swatch would omit it"
        );
        checked += 1;
    }
    assert_eq!(checked, 9);

    for class in [
        "swatch-role",
        "swatch-hex",
        "ramp-steps",
        "ramp-head",
        "ramp-note",
    ] {
        assert!(html.contains(class), "the page lost .{class}");
        assert!(script.contains(class), "site.js never builds .{class}");
    }
}

/// One palette in the markup, whatever the spec offers.
#[test]
fn only_the_default_palette_is_rendered() {
    let palette = noctua_docs::load(&root().join("dist")).expect("dist/ must exist");
    let html = page();

    let groups = html.matches("class=\"ramp-group").count();
    assert_eq!(groups, 2, "one palette, two modes — not {groups}");

    let default = palette.default_theme();
    assert!(html.contains(&format!("data-theme-name=\"{default}\"")));
    for theme in palette.theme_names() {
        if theme != default {
            assert!(
                !html.contains(&format!("data-theme-name=\"{theme}\"")),
                "{theme} was rendered into the page"
            );
        }
    }
}

/// Every page the generator renders.
fn rendered_pages() -> Vec<String> {
    let dist = root().join("dist");
    noctua_docs::render(&dist)
        .expect("dist/ must exist — run `cargo xtask build`")
        .into_iter()
        .map(|output| output.path)
        .collect()
}

/// Removes `<pre>` blocks, whose contents are shown rather than executed.
fn strip_code_blocks(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("<pre") {
        out.push_str(&rest[..start]);
        match rest[start..].find("</pre>") {
            Some(end) => rest = &rest[start + end + 6..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn collect(html: &str, attribute: &str) -> HashSet<String> {
    let mut found = HashSet::new();
    let mut rest = html;
    while let Some(at) = rest.find(attribute) {
        rest = &rest[at + attribute.len()..];
        let Some(end) = rest.find('"') else { break };
        found.insert(rest[..end].to_owned());
    }
    found
}

/// The escape marker, spelled once.
///
/// `noctua-check` owns it, but this crate deliberately does not depend on the
/// gates, so the string is repeated here and kept honest by
/// `the_marker_matches_the_source_gate`.
fn noctua_check_marker() -> &'static str {
    "allow-literal:"
}

/// Black and white have no free parameters: nobody *chose* them, so they are
/// not a second opinion about the palette. The source gate exempts them for
/// the same reason, and the two rules have to agree.
fn is_parameterless(literal: &str) -> bool {
    matches!(
        literal.to_ascii_lowercase().as_str(),
        "000" | "fff" | "000000" | "ffffff"
    )
}

fn contains_color_literal(line: &str) -> bool {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '#' {
            let digits: String = chars[i + 1..]
                .iter()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            if matches!(digits.len(), 3 | 6 | 8) && !is_parameterless(&digits) {
                return true;
            }
            i += 1 + digits.len().max(1);
        } else {
            i += 1;
        }
    }
    // `color-mix` over a token is fine; a raw triple is a color this file
    // invented.
    line.contains("rgb(") || line.contains("rgba(") || line.contains("hsl(")
}
