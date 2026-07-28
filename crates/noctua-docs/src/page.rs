//! The page shell: head, navigation, footer.
//!
//! One page. The site is a reference you scroll and search, not a hierarchy
//! you navigate — splitting it across routes would mean a visitor hunting for
//! "what is relative chroma" has to guess which page it is on.

use maud::{DOCTYPE, Markup, html};

use crate::Palette;
use crate::controls;
use crate::i18n::{Locale, t};
use crate::sections;

/// Renders the whole page in one locale.
#[must_use]
pub fn render(palette: &Palette, locale: Locale) -> String {
    let markup = html! {
        (DOCTYPE)
        html lang=(locale.tag()) data-default-locale[locale.is_default()] {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title {
                    (t(
                        locale,
                        "noctua-colors — a color system compiler",
                        "noctua-colors — um compilador de sistemas de cores",
                    ))
                }
                meta name="description" content=(t(
                    locale,
                    "A color system compiler. One declarative spec in; CSS, Tailwind, Rust, \
                     DTCG, TypeScript, SCSS and QML out. The repository versions the curves, \
                     not the colors.",
                    "Um compilador de sistemas de cores. Uma especificação declarativa entra; \
                     CSS, Tailwind, Rust, DTCG, TypeScript, SCSS e QML saem. O repositório \
                     versiona as curvas, não as cores.",
                ));

                // Each language points at the other, so a search engine
                // indexes them as one document in two versions rather than as
                // duplicates competing with each other.
                @for other in Locale::all() {
                    link rel="alternate" hreflang=(other.tag()) href=(other.page("index"));
                }

                (font_preloads())

                link rel="stylesheet" href="assets/fonts/fonts.css";
                // The generated tokens. Everything the page paints with comes
                // from here — there is no other source of color on this site.
                //
                // The shared ramp and the default palette only. `index.css`
                // imports every theme, which at thirty-six palettes is two
                // megabytes of CSS to render one of them; the rest are fetched
                // by `site.js` the first time they are chosen. Source order
                // then puts an injected sheet after this one, which is exactly
                // what the `[data-palette]` scheme relies on.
                link rel="stylesheet" href="tokens/css/ramp.css";
                // The semantic contract, emitted once because every theme
                // resolves it identically. Before the theme file, so a theme
                // that overrides a slot still wins.
                link rel="stylesheet" href="tokens/css/contexts.css";
                link rel="stylesheet" id="palette-stylesheet"
                     href=(format!("tokens/css/{}.css", palette.default_theme()));
                link rel="stylesheet" href="css/site.css";
                link rel="stylesheet" href="css/motion.css";

                // Applied before first paint so a dark-mode visitor never sees
                // a white flash. Inline because an external file would arrive
                // too late to prevent it.
                script {
                    (maud::PreEscaped(theme_bootstrap(Some(palette.default_theme()))))
                }
            }
            body {
                a class="skip-link" href="#main" {
                    (t(locale, "Skip to content", "Pular para o conteúdo"))
                }
                (navigation(palette, locale))
                main id="main" {
                    (sections::hero(palette, locale))
                    (sections::model(palette, locale))
                    (sections::palette_browser(palette, locale))
                    (sections::contexts(palette, locale))
                    (sections::contrast_matrix(palette, locale))
                    (sections::previews(palette, locale))
                    (sections::integration(locale))
                }
                (sections::detail_panel(locale))
                (footer(locale))
                script src="js/site.js" defer {}
            }
        }
    };
    format!("{}\n", markup.into_string())
}

/// The three faces `fonts.css` declares, in the order they are preloaded.
pub(crate) const FONT_FACES: [&str; 3] = ["Regular", "Bold", "Italic"];

/// Preloads every webfont face, ahead of the stylesheet that asks for them.
///
/// All three, not just the one first paint needs. `font-display: optional` uses
/// a face only if it is ready by the first paint and never swaps afterwards, so
/// a face the browser does not learn about until `fonts.css` parses routinely
/// misses that window and renders in the platform monospace *for that load*.
/// Preloading only the regular therefore left bold and italic falling back
/// inconsistently from one reload to the next, which reads as the typeface
/// flickering. The three are subset and total 34 KB, so preloading the lot
/// costs less than the one late request it removes.
///
/// Shared by both page shells: two lists of faces would drift the moment a
/// fourth was added.
pub(crate) fn font_preloads() -> Markup {
    html! {
        @for face in FONT_FACES {
            link rel="preload" href=(format!("assets/fonts/NoctuaIosevka-{face}.woff2"))
                 as="font" type="font/woff2" crossorigin;
        }
    }
}

/// The id given to the stylesheet the bootstrap injects.
///
/// `site.js` looks for it so it does not append the same sheet a second time.
/// Shared through `data-` attributes would be tidier, but this runs before the
/// body exists.
pub(crate) const BOOTSTRAP_SHEET_ID: &str = "palette-stylesheet-restored";

/// Where the bootstrap parks the palette JSON it started fetching.
///
/// `site.js` picks it up from `window`. A global is not elegant; it is the only
/// channel between a script that runs while the head is parsing and one that
/// runs after the body exists.
pub(crate) const THEME_FETCH: &str = "__noctuaThemeFetch";

/// Restores the visitor's choices before the first paint.
///
/// Deliberately tiny and deliberately inline. Anything loaded as a separate
/// file arrives after the first paint, which is exactly the flash this
/// prevents.
///
/// # Why it injects a stylesheet
///
/// Setting `data-palette` alone was not enough, and the gap was visible on every
/// reload. The attribute was restored before the first paint, but the sheet that
/// *defines* `[data-palette="blue-vivid"]` was only appended later by `site.js`
/// — so the page painted in the default theme's `:root` block and snapped to the
/// chosen palette once the fetch landed. Restoring a preference has to mean
/// restoring the thing it selects.
///
/// A `<link rel="stylesheet">` appended while the head is still parsing is
/// render-blocking, so the first paint already carries the right colours. The
/// href convention is duplicated from `site.js` and guarded by a test, because
/// two spellings of the same path is exactly the kind of thing that drifts.
///
/// # Why the language redirect lives here too
///
/// Same reason: sending someone to the other translation after the page has
/// rendered would show them a complete document in the wrong language first. It
/// runs before anything is painted, and it cannot loop — it only fires when the
/// stored language differs from this page's, and the page it navigates to
/// matches.
/// `default_theme` names the palette already linked in the markup, so the
/// bootstrap knows which one it does *not* need to fetch. `None` says every
/// palette is already present — which is the playground, where `index.css`
/// imports all of them — and suppresses the injection entirely.
pub(crate) fn theme_bootstrap(default_theme: Option<&str>) -> String {
    let inject = default_theme.map_or_else(String::new, |default| {
        format!(
            r"
      // The default palette is already linked in the markup; every other one
      // has to arrive before the first paint or its colours flash in late.
      if (palette !== '{default}' && /^[a-z0-9-]+$/.test(palette)) {{
        var sheet = document.createElement('link');
        sheet.rel = 'stylesheet';
        sheet.id = '{BOOTSTRAP_SHEET_ID}';
        // Set before insertion, and load-bearing: a stylesheet inserted by
        // script is *not* render-blocking by default, so without this the page
        // paints in the default palette and swaps — which is the whole flash
        // this exists to remove. Chrome reports the difference through
        // `PerformanceResourceTiming.renderBlockingStatus`.
        sheet.setAttribute('blocking', 'render');
        sheet.href = 'tokens/css/theme-' + palette + '.css';
        document.head.appendChild(sheet);

        // Started here rather than in `site.js`, which cannot ask for it until
        // it has resolved the palette from `axes.json` — three requests in
        // series, all of them after the first paint. This one leaves with the
        // stylesheet, so the numbers in the ramp browser land about as early as
        // they can. `site.js` reads the promise off `{THEME_FETCH}` and only
        // fetches for itself when the name does not match.
        if (typeof fetch === 'function') {{
          window.{THEME_FETCH} = {{
            theme: palette,
            data: fetch('tokens/json/themes/' + palette + '.json')
              .then(function (r) {{ return r.ok ? r.json() : null; }})
              // Swallowed on purpose: a failure here is not an error, it is a
              // missed optimisation, and `site.js` refetches and reports.
              .catch(function () {{ return null; }}),
          }};
        }}
      }}"
        )
    });

    format!(
        r"
(function () {{
  var root = document.documentElement;
  try {{
    // 'system' and an absent value both mean: let the media query decide, so
    // no attribute is set and the generated CSS handles it.
    var mode = localStorage.getItem('noctua-mode');
    if (mode === 'light' || mode === 'dark') root.setAttribute('data-theme', mode);

    var palette = localStorage.getItem('noctua-palette');
    if (palette) {{
      root.setAttribute('data-palette', palette);{inject}
    }}

    // Only the default page redirects. A URL that names its language is an
    // explicit request — a shared link — and must be honoured.
    var wanted = localStorage.getItem('noctua-locale');
    var here = root.getAttribute('lang');
    if (root.hasAttribute('data-default-locale') && wanted && here && wanted !== here) {{
      var target = document.querySelector('link[rel=alternate][hreflang=\'' + wanted + '\']');
      if (target) {{
        location.replace(target.getAttribute('href'));
      }}
    }}
  }} catch (e) {{
    /* Private browsing denies localStorage. The system preference still
       applies, so there is nothing to recover from. */
  }}
}})();
"
    )
}

fn navigation(palette: &Palette, locale: Locale) -> Markup {
    html! {
        header class="site-header" {
            nav class="nav" aria-label=(t(locale, "Primary", "Principal")) {
                a class="wordmark" href="#main" {
                    span class="wordmark-dot" aria-hidden="true" {}
                    "noctua-colors"
                }

                ul class="nav-links" {
                    li { a href="#model" { (t(locale, "Model", "Modelo")) } }
                    li { a href="#palette" { (t(locale, "Palette", "Paleta")) } }
                    li { a href="#contexts" { (t(locale, "Contexts", "Contextos")) } }
                    li { a href="#contrast" { (t(locale, "Contrast", "Contraste")) } }
                    li { a href="#previews" { (t(locale, "Previews", "Exemplos")) } }
                    li { a href="#integrate" { (t(locale, "Integrate", "Integrar")) } }
                    li {
                        a class="nav-playground" href=(locale.page("playground")) {
                            (t(locale, "Playground", "Laboratório"))
                        }
                    }
                }

                div class="nav-controls" {
                    (controls::palette_controls(palette, locale))
                    // Loading a palette can fail — a stylesheet or a theme
                    // file that will not fetch. Without somewhere to say so
                    // the page would keep the old colors while the picker
                    // claimed otherwise.
                    span id="palette-status" class="visually-hidden"
                         role="status" aria-live="polite" {}
                    (controls::mode_control(locale))
                    (controls::language_switch(locale, "index"))
                }
            }
        }
    }
}

pub(crate) fn footer(locale: Locale) -> Markup {
    html! {
        footer class="site-footer" {
            div class="wrap" {
                p {
                    (t(
                        locale,
                        "Every color on this page was computed from ",
                        "Cada cor desta página foi calculada a partir de ",
                    ))
                    code { "specs/noctua.toml" }
                    (t(locale, " and read back out of ", " e lida de volta de "))
                    code { "dist/" }
                    (t(
                        locale,
                        ". Nothing here is hand-picked, including the colors this page is \
                         painted with.",
                        ". Nada aqui foi escolhido à mão, nem mesmo as cores com que esta \
                         página é pintada.",
                    ))
                }
                p class="colophon" {
                    (t(locale, "Set in ", "Composta em "))
                    a href="assets/fonts/ATTRIBUTION.md" { "Noctua Iosevka" }
                    (t(
                        locale,
                        ", a custom build of Iosevka by Renzhi Li, licensed under the ",
                        ", uma compilação sob medida do Iosevka de Renzhi Li, licenciada sob a ",
                    ))
                    a href="assets/fonts/OFL.md" { "SIL Open Font License 1.1" }
                    (t(
                        locale,
                        ". Three faces, subset to 34 KB.",
                        ". Três variantes, reduzidas a 34 KB.",
                    ))
                }
            }
        }
    }
}
