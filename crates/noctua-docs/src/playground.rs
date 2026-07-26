//! The playground: the compiler itself, running in the browser.
//!
//! # Why this is a second page
//!
//! The rest of the site is deliberately one page, because a reference you
//! scroll beats a hierarchy you navigate. The playground is not reference —
//! it is a tool, and it costs a WebAssembly module of a few hundred kilobytes.
//! Putting it on the main page would charge every visitor that download to
//! read a paragraph about relative chroma. So it is a route of its own, and
//! the module is fetched only once someone has chosen to open it.
//!
//! # What makes it trustworthy
//!
//! It runs the real crates — the same parser, the same solver, the same
//! gates, compiled to a different target. There is no JavaScript
//! reimplementation of the color model here to drift from the Rust one,
//! because there is no second implementation at all.

use maud::{DOCTYPE, Markup, html};

use crate::Palette;
use crate::controls;
use crate::i18n::{Locale, t};

/// Renders the playground page.
#[must_use]
pub(crate) fn render(palette: &Palette, locale: Locale) -> String {
    let markup = html! {
        (DOCTYPE)
        html lang=(locale.tag()) data-default-locale[locale.is_default()] {
            (head(locale))
            body {
                a class="skip-link" href="#main" {
                    (t(locale, "Skip to content", "Pular para o conteúdo"))
                }

                nav class="nav" aria-label=(t(locale, "Primary", "Principal")) {
                    a class="nav-brand" href=(locale.page("index")) { "noctua-colors" }
                    div class="nav-links" {
                        a href=(locale.page("index")) {
                            (t(locale, "Back to the reference", "Voltar à referência"))
                        }
                    }
                    div class="nav-controls" {
                        (controls::mode_control(locale))
                        (controls::language_switch(locale, "playground"))
                    }
                }

                // The script writes status text, gate summaries and copy
                // feedback at runtime, so its wording travels here rather
                // than in the markup — there is nowhere else it could read
                // the page's language from.
                (workspace(palette, locale))

                (crate::page::footer(locale))

                // A module, so it can `import` the generated bindings. No
                // bundler: the browser resolves it, which is the whole point
                // of emitting an ES module from wasm-bindgen.
                script type="module" src="js/playground.js" {}
            }
        }
    };
    format!("{}\n", markup.into_string())
}

/// The editor, the result pane, and the strings the script needs.
fn workspace(palette: &Palette, locale: Locale) -> Markup {
    html! {
        main id="main" class="playground"
             data-s-compiled=(t(locale, "themes compiled", "temas compilados"))
             data-s-compiled-one=(t(locale, "theme compiled", "tema compilado"))
             data-s-failing=(t(locale, "not compiling", "não compila"))
             data-s-checks=(t(locale, "checks", "verificações"))
             data-s-failing-count=(t(locale, "failing", "falhando"))
             data-s-warnings=(t(locale, "warnings", "avisos"))
             data-s-warning-one=(t(locale, "warning", "aviso"))
             data-s-all-passed=(t(locale, "Every gate passed.", "Todas as verificações passaram."))
             data-s-no-files=(t(locale, "This target produced no files.", "Este alvo não produziu arquivos."))
             data-s-copied=(t(locale, "link copied", "link copiado"))
             data-s-in-bar=(t(locale, "the link is in the address bar", "o link está na barra de endereços"))
             data-s-copy-failed=(t(
                 locale,
                 "could not copy — the link is in the address bar",
                 "não foi possível copiar — o link está na barra de endereços",
             ))
             data-s-load-failed=(t(locale, "the compiler did not load", "o compilador não carregou"))
             data-s-wasm-error=(t(
                 locale,
                 "The WebAssembly module failed to load: ",
                 "O módulo WebAssembly não carregou: ",
             ))
             data-s-reference-works=(t(
                 locale,
                 "The reference page works without it.",
                 "A página de referência funciona sem ele.",
             )) {
            header class="playground-head" {
                h1 { (t(locale, "Playground", "Laboratório")) }
                p class="muted" {
                    (t(
                        locale,
                        "The compiler, running here. Edit the specification and every \
                         ramp, every gate and every generated file is recomputed by the \
                         same Rust that runs on the command line.",
                        "O compilador, rodando aqui. Edite a especificação e cada \
                         escala, cada verificação e cada arquivo gerado é recalculado \
                         pelo mesmo Rust que roda na linha de comando.",
                    ))
                }
            }

            div class="playground-bar" {
                button type="button" id="pg-reset" class="control" {
                    (t(
                        locale,
                        "Reset to the shipped spec",
                        "Restaurar a especificação original",
                    ))
                }
                button type="button" id="pg-share" class="control" {
                    (t(
                        locale,
                        "Copy a link to this spec",
                        "Copiar um link para esta especificação",
                    ))
                }
                span id="pg-status" class="pg-status" role="status" aria-live="polite" {
                    (t(locale, "Loading the compiler…", "Carregando o compilador…"))
                }
            }

            div class="playground-grid" {
                section class="pg-editor" aria-labelledby="pg-editor-heading" {
                    h2 id="pg-editor-heading" class="pg-heading" {
                        (t(locale, "Specification", "Especificação"))
                    }
                    // `spellcheck` off: this is code, and a red
                    // squiggle under every token name is noise.
                    textarea id="pg-spec" class="pg-textarea" spellcheck="false"
                             aria-label=(t(
                                 locale,
                                 "Color specification, in TOML",
                                 "Especificação de cores, em TOML",
                             )) {
                        (t(locale, SPEC_PLACEHOLDER, SPEC_PLACEHOLDER_PT))
                    }
                }

                (result_pane(locale))
            }

            (fallback(palette, locale))
        }
    }
}

/// The tabs and panels the compiler's output lands in.
fn result_pane(locale: Locale) -> Markup {
    html! {
        section class="pg-output" aria-labelledby="pg-output-heading" {
            h2 id="pg-output-heading" class="pg-heading" {
                (t(locale, "Result", "Resultado"))
            }

            div id="pg-error" class="pg-error" hidden {}

            div class="pg-tabs" role="tablist"
                aria-label=(t(locale, "What to show", "O que mostrar")) {
                @for (index, (id, label)) in views(locale).iter().enumerate() {
                    button type="button" class="tab" role="tab"
                           id={ "pg-tab-" (id) }
                           aria-controls={ "pg-panel-" (id) }
                           aria-selected=(if index == 0 { "true" } else { "false" })
                           tabindex=(if index == 0 { "0" } else { "-1" })
                           data-view=(id) {
                        (label)
                    }
                }
            }

            @for (index, (id, label)) in views(locale).iter().enumerate() {
                div class="pg-panel" role="tabpanel"
                    id={ "pg-panel-" (id) }
                    aria-labelledby={ "pg-tab-" (id) }
                    hidden[index != 0] {
                    @if *id == "emit" {
                        (emit_controls(locale))
                    }
                    div class="pg-panel-body" id={ "pg-body-" (id) } {
                        p class="muted" {
                            (t(locale, "Compiling ", "Compilando "))
                            (label) "…"
                        }
                    }
                }
            }
        }
    }
}

/// Everything in `<head>`: metadata, stylesheets and the pre-paint bootstrap.
fn head(locale: Locale) -> Markup {
    html! {
        head {
            meta charset="utf-8";
            meta name="viewport" content="width=device-width, initial-scale=1";
            title {
                (t(
                    locale,
                    "Playground — noctua-colors",
                    "Laboratório — noctua-colors",
                ))
            }
            meta name="description" content=(t(
                locale,
                "Edit a color specification and watch the compiler resolve it, gate it and \
                 emit every target — in the browser, running the same Rust that runs on the \
                 command line.",
                "Edite uma especificação de cores e veja o compilador resolvê-la, verificá-la \
                 e emitir todos os alvos — no navegador, rodando o mesmo Rust que roda na \
                 linha de comando.",
            ));

            @for other in Locale::all() {
                link rel="alternate" hreflang=(other.tag())
                     href=(other.page("playground"));
            }

            link rel="preload" href="assets/fonts/NoctuaIosevka-Regular.woff2"
                 as="font" type="font/woff2" crossorigin;
            link rel="stylesheet" href="assets/fonts/fonts.css";
            link rel="stylesheet" href="tokens/css/index.css";
            link rel="stylesheet" href="css/site.css";
            link rel="stylesheet" href="css/motion.css";

            // No palette to inject: `index.css` above imports every theme,
            // so whatever `data-palette` restores is already defined.
            script { (maud::PreEscaped(crate::page::theme_bootstrap(None))) }
        }
    }
}

/// The views the result pane can show.
///
/// The identifiers are stable and untranslated — the script and the CSS key
/// off them — while only the labels change language.
fn views(locale: Locale) -> [(&'static str, &'static str); 3] {
    [
        ("ramps", t(locale, "Ramps", "Escalas")),
        ("gates", t(locale, "Gates", "Verificações")),
        ("emit", t(locale, "Generated files", "Arquivos gerados")),
    ]
}

/// Shown in the editor before the module has loaded and replaced it.
///
/// Not the real spec: that is embedded in the WebAssembly module and read
/// from there, so there is exactly one copy of it in the build.
const SPEC_PLACEHOLDER: &str = "# Loading the shipped specification…";

/// The same, in Portuguese. A comment, so it is still valid TOML.
const SPEC_PLACEHOLDER_PT: &str = "# Carregando a especificação original…";

fn emit_controls(locale: Locale) -> Markup {
    html! {
        div class="pg-emit-bar" {
            label class="pg-label" for="pg-target" { (t(locale, "Target", "Alvo")) }
            select id="pg-target" class="control" {
                // Filled from the module, so the list is whatever the
                // compiler actually registers rather than a copy of it.
            }
            select id="pg-file" class="control"
                   aria-label=(t(locale, "File", "Arquivo")) {}
        }
    }
}

/// What the page says when script or WebAssembly is unavailable.
///
/// A blank tool with no explanation is the worst outcome; this at least says
/// what was meant to be here and points at the thing that works without it.
fn fallback(palette: &Palette, locale: Locale) -> Markup {
    html! {
        noscript class="pg-fallback" {
            h2 {
                (t(
                    locale,
                    "The playground needs JavaScript and WebAssembly",
                    "O laboratório precisa de JavaScript e WebAssembly",
                ))
            }
            p {
                (t(
                    locale,
                    "It compiles specifications in the browser, so there is no version of it \
                     that runs without them. The reference page works fully without either, and \
                     shows the same ",
                    "Ele compila especificações no navegador, então não existe versão que \
                     funcione sem eles. A página de referência funciona por completo sem \
                     nenhum dos dois, e mostra os mesmos ",
                ))
                (palette.themes.len())
                (t(locale, " themes already compiled.", " temas já compilados."))
            }
            p {
                a href=(locale.page("index")) {
                    (t(locale, "Go to the reference", "Ir para a referência"))
                }
            }
        }
    }
}
