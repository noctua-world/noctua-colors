//! The page's content.
//!
//! Every number rendered here was read from `system/json/palette.json`. Where
//! the copy quotes a figure — a chroma, an Lc, a separation — it is
//! interpolated from the data rather than typed, so the prose cannot go stale
//! while the palette moves under it.

use maud::{Markup, PreEscaped, html};

use crate::data::{ModePalette, Palette, Scale, Step};
use crate::i18n::{Locale, t};

/// The opening statement.
#[must_use]
pub fn hero(palette: &Palette, locale: Locale) -> Markup {
    let mode = palette.mode(palette.default_theme(), "light");
    let accent = mode.and_then(|m| m.families.get("accent"));
    let solid = accent.and_then(|f| f.steps.iter().find(|s| s.role == "solid"));

    html! {
        section class="hero" {
            div class="wrap" {
                p class="eyebrow reveal" {
                    (t(locale, "a colour system", "um sistema de cores"))
                }
                h1 class="reveal" {
                    (t(locale, "Colours that were ", "Cores que foram "))
                    em { (t(locale, "solved", "resolvidas")) }
                    (t(locale, ", not picked.", ", não escolhidas."))
                }
                p class="lead reveal" {
                    (t(
                        locale,
                        "Most colour systems are a list of hex values somebody chose. This one \
                         is the output of a compiler: every step of every ramp was solved \
                         against a perceptual contrast target, in every palette, and checked \
                         before it shipped. Link one file and you have all of it — light and \
                         dark included.",
                        "A maioria dos sistemas de cores é uma lista de valores hexadecimais \
                         que alguém escolheu. Este é a saída de um compilador: cada passo de \
                         cada rampa foi resolvido contra um alvo de contraste perceptual, em \
                         todas as paletas, e verificado antes de ser publicado. Vincule um \
                         arquivo e você tem tudo — claro e escuro inclusos.",
                    ))
                }

                p class="hero-actions reveal" {
                    a class="button button-primary" href="#install" {
                        (t(locale, "Install it", "Instalar"))
                    }
                    a class="button button-secondary" href="#palette" {
                        (t(locale, "Browse the palettes", "Ver as paletas"))
                    }
                }

                @if let Some(step) = solid {
                    div class="hero-proof reveal" {
                        // Painted from the token rather than from a literal,
                        // so it follows the palette the reader picked. The
                        // figures beside it are updated by `site.js`; without
                        // that the hero would keep quoting the default
                        // palette's hex while the page showed another.
                        div class="hero-swatch" style="background: var(--nc-color-accent)" {}
                        div class="hero-proof-text" {
                            p {
                                (t(locale, "This swatch is ", "Esta amostra é "))
                                code id="hero-hex" { (step.primary().hex) }
                                (t(
                                    locale,
                                    " — the accent family's ",
                                    " — o passo ",
                                ))
                                code { "solid" }
                                (t(
                                    locale,
                                    " step. It was not chosen. Its hue came from measuring an \
                                     existing palette, its lightness was solved from a contrast \
                                     target, and its chroma is ",
                                    " da família accent. Ela não foi escolhida. Seu matiz veio da \
                                     medição de uma paleta existente, sua luminosidade foi \
                                     resolvida a partir de um alvo de contraste, e seu croma é ",
                                ))
                                strong id="hero-chroma" {
                                    (format!("{:.0}%", step.primary().relative_chroma * 100.0))
                                }
                                (t(
                                    locale,
                                    " of the most sRGB can show at that lightness and hue.",
                                    " do máximo que o sRGB consegue exibir naquela luminosidade e \
                                     naquele matiz.",
                                ))
                            }
                            p class="muted" {
                                code id="hero-css" { (step.primary().css) }
                            }
                        }
                    }
                }

                ul class="hero-stats reveal" {
                    (stat(&palette.themes.len().to_string(), t(locale, "palettes", "paletas")))
                    (stat(
                        &palette.gray_ramp().len().to_string(),
                        t(locale, "neutral steps", "passos neutros"),
                    ))
                    (stat(
                        &palette.roles.len().to_string(),
                        t(locale, "roles per family", "papéis por família"),
                    ))
                    (stat(&palette.gamuts.len().to_string(), t(locale, "gamuts", "gamuts")))
                }
            }
        }
    }
}

fn stat(value: &str, label: &str) -> Markup {
    html! {
        li { strong { (value) } span { (label) } }
    }
}

/// How to actually get the colours — the section the site did not have.
///
/// It leads with the smallest thing that works, a single `<link>` to a single
/// palette, because that is what most arrivals want and because the number
/// beside it is the argument: one palette is a twenty-fifth of the weight of
/// all of them.
///
/// The palette name is taken from the palette rather than typed, so this stays
/// correct if the default is ever renamed.
#[must_use]
pub fn install(palette: &Palette, locale: Locale) -> Markup {
    let default = palette.default_theme();
    let cdn = format!(
        "<link rel=\"stylesheet\"\n      \
         href=\"https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/palette/{default}.css\">"
    );
    let npm = format!(
        "npm install @noctua-world/colors\n\n/* then, in your CSS */\n@import \"@noctua-world/colors/palette/{default}.css\";"
    );
    let tailwind = format!(
        "@import \"tailwindcss\";\n@import \"@noctua-world/colors/tailwind/palette/{default}.css\";\n\n\
         <!-- then -->\n<div class=\"bg-surface text-fg border-border dark:bg-surface-subtle\">"
    );
    let rust = format!(
        "cargo add noctua-colors-tokens --features {}\n\nuse noctua_colors_tokens::{}::light::accent;\nlet hex = accent::SOLID.hex;",
        default.replace('-', "_"),
        default.replace('-', "_")
    );

    let routes: [(&str, &str, &String); 4] = [
        ("A browser", "install-cdn", &cdn),
        ("npm", "install-npm", &npm),
        ("Tailwind v4", "install-tailwind", &tailwind),
        ("Rust", "install-rust", &rust),
    ];

    html! {
        section id="install" class="section" {
            div class="wrap" {
                h2 class="reveal" { (t(locale, "Install", "Instalar")) }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "Pick the one that matches you. Each is complete — there is no step two.",
                        "Escolha o que combina com você. Cada um está completo — não existe \
                         passo dois.",
                    ))
                }

                div class="tabs reveal" {
                    div class="tab-strip" role="tablist"
                        aria-label=(t(locale, "Installation routes", "Formas de instalação")) {
                        @for (index, (label, id, _)) in routes.iter().enumerate() {
                            button type="button" class="tab" role="tab"
                                   id=(format!("tab-{id}"))
                                   aria-controls=(format!("panel-{id}"))
                                   aria-selected=(if index == 0 { "true" } else { "false" })
                                   tabindex=(if index == 0 { "0" } else { "-1" }) {
                                (label)
                            }
                        }
                    }
                    @for (index, (_, id, code)) in routes.iter().enumerate() {
                        div class="tab-panel" role="tabpanel"
                            id=(format!("panel-{id}"))
                            aria-labelledby=(format!("tab-{id}"))
                            hidden[index != 0] {
                            pre { code { (PreEscaped(html_escape(code))) } }
                        }
                    }
                }

                p class="section-note reveal" {
                    (t(
                        locale,
                        "That is one palette, complete: the neutral ramp, every semantic name, \
                         and both modes — 29 KB over the wire. The file carrying all ",
                        "Isso é uma paleta, completa: a rampa neutra, todos os nomes semânticos \
                         e os dois modos — 29 KB na rede. O arquivo com as ",
                    ))
                    strong { (palette.themes.len().to_string()) }
                    (t(
                        locale,
                        " is 741 KB, which is why the small one exists. Use it only if your \
                         users switch palettes at runtime.",
                        " paletas tem 741 KB, e é por isso que o pequeno existe. Use-o apenas \
                         se seus usuários trocarem de paleta em tempo de execução.",
                    ))
                }
            }
        }
    }
}

/// The three ideas, shown rather than described.
#[must_use]
pub fn model(palette: &Palette, locale: Locale) -> Markup {
    let mode = palette.mode(palette.default_theme(), "light");

    html! {
        section id="model" class="section" {
            div class="wrap" {
                h2 class="reveal" { (t(locale, "How it works", "Como funciona")) }

                div class="cards" {
                    (relative_chroma_card(mode, locale))
                    (contrast_card(locale))
                    (torsion_card(locale))
                }
            }
        }
    }
}

fn relative_chroma_card(mode: Option<&ModePalette>, locale: Locale) -> Markup {
    html! {
        article class="card reveal" {
            h3 { (t(locale, "Relative chroma", "Croma relativo")) }
            p {
                (t(
                    locale,
                    "Chroma is stored as a fraction of what the target gamut can \
                     actually show at a given lightness and hue — never as an \
                     absolute number. One definition renders correctly on sRGB and \
                     more vividly on a wide-gamut display, without being redefined.",
                    "O croma é guardado como uma fração do que o gamut de destino \
                     realmente consegue exibir em dada luminosidade e matiz — nunca \
                     como um número absoluto. Uma única definição se comporta bem no \
                     sRGB e fica mais vívida num monitor de gamut amplo, sem ser \
                     redefinida.",
                ))
            }
            @if let Some(m) = mode {
                @if let Some(accent) = m.families.get("accent") {
                    @if let Some(step) = accent.steps.iter().find(|s| s.role == "solid") {
                        (gamut_comparison(step, locale))
                    }
                }
            }
        }
    }
}

fn contrast_card(locale: Locale) -> Markup {
    html! {
        article class="card reveal" {
            h3 {
                (t(
                    locale,
                    "Contrast-anchored steps",
                    "Passos ancorados no contraste",
                ))
            }
            p {
                (t(
                    locale,
                    "A step's lightness is ",
                    "A luminosidade de um passo é ",
                ))
                em { (t(locale, "solved", "resolvida")) }
                (t(
                    locale,
                    ", not authored. Each role declares what it must achieve against \
                     a reference — APCA contrast for text and solids, perceptual \
                     lightness separation for surfaces and borders — and the engine \
                     finds the lightness that hits it.",
                    ", não escrita à mão. Cada papel declara o que precisa alcançar \
                     em relação a uma referência — contraste APCA para textos e \
                     preenchimentos, separação perceptual de luminosidade para \
                     superfícies e bordas — e o motor encontra a luminosidade que \
                     satisfaz esse alvo.",
                ))
            }
            p class="muted small" {
                (t(
                    locale,
                    "APCA rather than WCAG 2.x, because WCAG passes \
                     light-grey-on-black pairs that are genuinely hard to read. \
                     Designing against it produces dark themes that satisfy an audit \
                     and hurt to use.",
                    "APCA em vez de WCAG 2.x, porque o WCAG aprova combinações de \
                     cinza-claro sobre preto que são genuinamente difíceis de ler. \
                     Projetar contra ele produz temas escuros que passam na \
                     auditoria e cansam a vista.",
                ))
            }
        }
    }
}

fn torsion_card(locale: Locale) -> Markup {
    html! {
        article class="card reveal" {
            h3 { (t(locale, "Hue torsion", "Torção de matiz")) }
            p {
                (t(
                    locale,
                    "Hue shifts deliberately along a ramp — shadows cooler, \
                     highlights warmer, or the reverse. It is what makes a ramp look \
                     designed rather than computed.",
                    "O matiz se desloca de propósito ao longo da escala — sombras \
                     mais frias, luzes mais quentes, ou o contrário. É o que faz uma \
                     escala parecer projetada em vez de calculada.",
                ))
            }
            p class="muted small" {
                (t(
                    locale,
                    "Kept in a separate field from the corrective term that \
                     compensates Oklab's blue-toward-purple drift. Merging intent \
                     with a workaround makes \"was this on purpose?\" unanswerable a \
                     year later.",
                    "Fica num campo separado do termo corretivo que compensa o desvio \
                     do azul em direção ao roxo no Oklab. Misturar intenção com \
                     contorno técnico torna \"isso foi de propósito?\" impossível de \
                     responder um ano depois.",
                ))
            }
        }
    }
}

/// The same token in every emitted gamut, side by side.
fn gamut_comparison(step: &Step, locale: Locale) -> Markup {
    html! {
        div class="gamut-compare" {
            @for color in &step.renditions {
                div class="gamut-row" {
                    span class="gamut-chip" style=(format!("background: {}", color.css)) {}
                    span class="gamut-name" { (color.gamut) }
                    span class="gamut-value" { (format!("C {:.4}", color.oklch.c)) }
                }
            }
            p class="muted small" {
                (t(
                    locale,
                    "Different numbers, not the same color repeated. The wider gamut resolves \
                     the same relative chroma against more room.",
                    "Números diferentes, não a mesma cor repetida. O gamut mais amplo resolve o \
                     mesmo croma relativo contra mais espaço disponível.",
                ))
            }
        }
    }
}

/// Every family, every step, clickable.
#[must_use]
pub fn palette_browser(palette: &Palette, locale: Locale) -> Markup {
    let theme = palette.default_theme();

    html! {
        section id="palette" class="section" {
            div class="wrap" {
                h2 class="reveal" { (t(locale, "The palette", "A paleta")) }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "Every family in every theme, in both modes. Tap any swatch to copy it \
                         — as hex, OKLCH, a CSS variable, a Rust path or a Tailwind class.",
                        "Cada família em cada tema, nos dois modos. Toque em qualquer amostra \
                         para copiá-la — como hexadecimal, OKLCH, variável CSS, caminho Rust ou \
                         classe Tailwind.",
                    ))
                }

                // Only the default palette is rendered. Every theme was once
                // written into the page with `hidden` on all but one, which at
                // thirty-six palettes is two megabytes of markup to show one
                // of them. The rest are built client-side from
                // `tokens/json/themes/<name>.json` when selected — see
                // `renderRamps` in `docs-site/js/site.js`, which mirrors
                // `ramp_table` and `swatch` below.
                //
                // The default stays server-rendered so the page is complete
                // without script.
                // The client-side renderer has no other way to learn the
                // page's language, so its wording travels as data attributes —
                // the same arrangement the swatch detail panel uses.
                div id="ramp-browser"
                    data-label-hue=(t(locale, "hue", "matiz"))
                    data-label-chart=(t(locale, "categorical", "categórica"))
                    data-label-chart-labelled=(t(
                        locale,
                        "categorical, labelled",
                        "categórica, rotulada",
                    ))
                    data-label-ordered=(t(locale, "ordered", "ordenada"))
                    data-label-roles=(t(
                        locale,
                        "Roles are canonical; the numbers are aliases for interop.",
                        "Os papéis são canônicos; os números são apelidos para \
                         interoperabilidade.",
                    ))
                    data-label-gamuts=(t(
                        locale,
                        "gamuts emitted per token.",
                        "gamuts emitidos por token.",
                    ))
                    data-gamut-count=(palette.gamuts.len()) {
                    // Both modes are emitted and CSS decides which one shows —
                    // see `[data-mode]` in `site.css`. Marking the dark group
                    // `hidden` and letting script unhide it meant a dark-mode
                    // visitor painted the *light* table first and watched it be
                    // replaced on every reload, and meant the page was simply
                    // wrong in dark mode with script off.
                    @for mode_name in ["light", "dark"] {
                        @if let Some(mode) = palette.mode(theme, mode_name) {
                            div class="ramp-group reveal"
                                data-theme-name=(theme)
                                data-mode=(mode_name) {
                                (ramp_table(mode, palette, locale))
                            }
                        }
                    }
                }

                h3 class="reveal" { (t(locale, "Neutral ramps", "Escalas neutras")) }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "Separate from the twelve functional roles and shared by both modes. \
                         Interfaces need far finer gray resolution than twelve steps, and they \
                         need it concentrated where surfaces actually live — just below white \
                         and just above black — rather than spread evenly.",
                        "Separada dos doze papéis funcionais e compartilhada pelos dois modos. \
                         Interfaces precisam de uma resolução de cinzas bem mais fina que doze \
                         passos, e precisam dela concentrada onde as superfícies realmente ficam \
                         — logo abaixo do branco e logo acima do preto — em vez de distribuída \
                         por igual.",
                    ))
                }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "The cool and warm ramps are the same lightnesses at a different \
                         temperature — step placement depends only on the count and the \
                         density, so gray-7 and gray-cool-7 differ in tint and nothing else, \
                         and one can be swapped for the other without moving any contrast.",
                        "As escalas fria e quente têm exatamente as mesmas luminosidades em \
                         outra temperatura — o posicionamento dos passos depende apenas da \
                         quantidade e da densidade, então gray-7 e gray-cool-7 diferem só no \
                         matiz, e uma pode substituir a outra sem mover nenhum contraste.",
                    ))
                }
                @for (ramp, steps) in &palette.gray_ramps {
                    h4 class="reveal ramp-name" { code { "--" (palette.prefix) "-" (ramp) "-*" } }
                    div class="gray-ramp reveal" {
                        @for step in steps {
                            (swatch(step, ramp, step.index.to_string().as_str(), &palette.prefix))
                        }
                    }
                }
            }
        }
    }
}

/// What kind of scale this is, in one word.
///
/// The difference is not cosmetic: a chart spreads hues *around the wheel* so a
/// legend can tell six series apart, while an ordered scale walks a hue *path*
/// so a reader can tell which of two stops is worse without one.
///
/// Read off the scale rather than guessed from its name. Testing the stem
/// against `chart` was correct until a second categorical set existed, at which
/// point it labelled every one of them "ordered".
fn scale_kind(scale: &Scale, locale: Locale) -> &'static str {
    if !scale.is_categorical() {
        t(locale, "ordered", "ordenada")
    } else if scale.labelled {
        t(locale, "categorical, labelled", "categórica, rotulada")
    } else {
        t(locale, "categorical", "categórica")
    }
}

fn ramp_table(mode: &ModePalette, palette: &Palette, locale: Locale) -> Markup {
    html! {
        div class="ramps" {
            @for (family, resolved) in &mode.families {
                div class="ramp" {
                    div class="ramp-head" {
                        h4 { (family) }
                        span class="muted small" {
                            (format!(
                                "{} {:.0}°",
                                t(locale, "hue", "matiz"),
                                resolved.base_hue
                            ))
                        }
                    }
                    div class="ramp-steps" {
                        @for step in &resolved.steps {
                            (swatch(step, family, &step.role, &palette.prefix))
                        }
                    }
                }
            }
            @for (name, scale) in &mode.scales {
                div class="ramp" {
                    div class="ramp-head" {
                        h4 { (name) }
                        span class="muted small" { (scale_kind(scale, locale)) }
                    }
                    div class="ramp-steps" {
                        @for step in &scale.steps {
                            (swatch(step, name, &step.role, &palette.prefix))
                        }
                    }
                }
            }
        }
        p class="muted small ramp-note" {
            (t(
                locale,
                "Roles are canonical; the numbers are aliases for interop. ",
                "Os papéis são canônicos; os números são apelidos para interoperabilidade. ",
            ))
            (format!(
                "{} {}",
                palette.gamuts.len(),
                t(locale, "gamuts emitted per token.", "gamuts emitidos por token."),
            ))
        }
    }
}

/// One swatch, carrying everything the detail panel needs.
///
/// Painted with its own token rather than with the colour that token resolved
/// to when the page was generated. Only the default palette is server-rendered,
/// so a baked value meant every tile in this grid showed the *default* palette
/// on reload — the visitor's choice arrived with the JSON, three requests
/// later, and the whole browser repainted in front of them. The token follows
/// whichever stylesheet the bootstrap restored, so the colours are right at
/// first paint and the JSON only refreshes the numbers.
fn swatch(step: &Step, family: &str, role: &str, prefix: &str) -> Markup {
    let color = step.primary();
    let stem = format!("{family}-{role}");
    html! {
        button type="button"
               class="swatch"
               style=(format!("background: var(--{prefix}-{stem})"))
               data-stem=(stem)
               data-hex=(color.hex)
               data-css=(color.css)
               data-l=(format!("{:.4}", color.oklch.l))
               data-c=(format!("{:.4}", color.oklch.c))
               data-h=(format!("{:.2}", color.oklch.h))
               data-cr=(format!("{:.3}", color.relative_chroma))
               data-headroom=(format!("{:.4}", color.chroma_headroom))
               // Light text on dark swatches and vice versa, decided from the
               // step's own lightness rather than guessed.
               data-ink=(ink(step))
               aria-label=(format!("{stem}, {}", color.hex)) {
            span class="swatch-role" { (role) }
            span class="swatch-hex" { (color.hex) }
        }
    }
}

/// Every semantic context, painted from the tokens themselves.
///
/// Not a table of hex values: each chip is filled with `var(--nc-color-<slot>)`
/// and lettered with `var(--nc-color-on-<slot>)`, so it follows whichever palette
/// and mode the visitor picked and cannot show a colour the stylesheet is not
/// actually serving. A hex string here would be a claim; this is the token.
#[must_use]
pub fn contexts(palette: &Palette, locale: Locale) -> Markup {
    let mode = palette.mode(palette.default_theme(), "light");

    html! {
        section id="contexts" class="section" {
            div class="wrap" {
                h2 class="reveal" { (t(locale, "Contexts", "Contextos")) }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "Every name an application can code against. Ten of these are families \
                         with a hue of their own, because their meanings have to be told apart \
                         without a legend; the rest are aliases onto one of those, because a \
                         family costs a full ramp in every theme, mode and gamut and an alias \
                         costs one line.",
                        "Todos os nomes que uma aplicação pode usar. Dez deles são famílias com \
                         matiz própria, porque seus significados precisam ser distinguidos sem \
                         legenda; o resto são apelidos para uma dessas, porque uma família custa \
                         uma escala inteira em cada tema, modo e gamut, e um apelido custa uma \
                         linha.",
                    ))
                }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "Colour alone cannot separate ten meanings for a dichromat — measured, \
                         and published in system/reports/colour-vision.md. Pair a status colour \
                         with an icon or a label.",
                        "Cor sozinha não separa dez significados para um dicromata — medido, e \
                         publicado em system/reports/colour-vision.md. Acompanhe uma cor de estado \
                         com um ícone ou um rótulo.",
                    ))
                }

                (context_chips(mode, &palette.prefix, locale))
                (categorical_scales(mode, &palette.prefix, locale))
                (ordered_scales(mode, &palette.prefix, locale))
                (translucency(mode, &palette.prefix, locale))
            }
        }
    }
}

/// One chip per context, filled with the token rather than with a hex string.
///
/// Grouped by the family behind them, and collapsed into a `<details>` per
/// group. Three hundred and fifty chips in one grid is a wall a reader
/// scrolls past rather than reads, and the grouping answers the question they
/// actually arrive with — not "which contexts exist" but "which other names
/// share this colour", which is the thing that decides whether two states in
/// the same view can be told apart.
///
/// `<details>` rather than script, so the groups open and close with this file
/// blocked. The filter above them is the enhancement.
fn context_chips(mode: Option<&ModePalette>, prefix: &str, locale: Locale) -> Markup {
    // Contexts only. A `neutral*` slot is the page itself — its tokens are the
    // surfaces and text this section is already drawn with.
    let mut groups: indexmap::IndexMap<&str, Vec<&str>> = indexmap::IndexMap::new();
    if let Some(m) = mode {
        for (slot, family) in &m.slots {
            if is_surface_slot(slot) {
                continue;
            }
            groups
                .entry(family.as_str())
                .or_default()
                .push(slot.as_str());
        }
    }
    let total: usize = groups.values().map(Vec::len).sum();

    html! {
        div class="context-browser reveal" {
            div class="context-filter" {
                label class="visually-hidden" for="context-filter" {
                    (t(locale, "Filter contexts", "Filtrar contextos"))
                }
                input type="search" id="context-filter" class="input"
                      placeholder=(t(locale, "Filter contexts…", "Filtrar contextos…"))
                      // The script writes a count into the status line beside
                      // this, and takes its wording from here rather than from
                      // a string of its own — the page is built once per
                      // language and nothing a script writes may be in the
                      // wrong one.
                      data-label-matches=(t(locale, "matching", "correspondentes"))
                      data-label-none=(t(
                          locale,
                          "no context matches",
                          "nenhum contexto corresponde",
                      ));
                p id="context-count" class="muted small" role="status" aria-live="polite" {
                    (format!("{total} "))
                    (t(locale, "contexts", "contextos"))
                }
            }

            @for (family, slots) in &groups {
                details class="context-group" open {
                    summary {
                        code { (family) }
                        span class="muted small" {
                            (format!(" · {} ", slots.len()))
                            (t(locale, "contexts", "contextos"))
                        }
                    }
                    ul class="context-grid" {
                        @for slot in slots {
                            li class="context-chip" data-slot=(slot)
                               style=(format!(
                                   "background: var(--{prefix}-color-{slot}); \
                                    color: var(--{prefix}-color-on-{slot});"
                               )) {
                                code { (slot) }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The ordinal scales, as strips.
fn ordered_scales(mode: Option<&ModePalette>, prefix: &str, locale: Locale) -> Markup {
    html! {
        h3 class="reveal" { (t(locale, "Ordered scales", "Escalas ordenadas")) }
        p class="section-lead reveal" {
            (t(
                locale,
                "A traffic light, read in order. Stops are spaced by perceptual distance along \
                 the hue path rather than evenly in the parameter, and lightness descends by \
                 equal steps of stop index — because lightness is the only axis left when hue \
                 is unavailable, and only the index guarantees the step is even.",
                "Um semáforo, lido em ordem. Os passos são espaçados por distância perceptual \
                 ao longo do caminho de matiz, e não igualmente no parâmetro, e a luminosidade \
                 desce em passos iguais de índice — porque a luminosidade é o único eixo que \
                 sobra quando a matiz falta, e só o índice garante que o passo seja igual.",
            ))
        }
        @if let Some(m) = mode {
            @for (name, scale) in m.scales.iter().filter(|(_, s)| !s.is_categorical()) {
                (scale_strip(name, scale, prefix))
            }
        }
    }
}

/// The categorical sets, as strips.
///
/// Separate from the ordered ones because they answer a different question. A
/// reader looking for "which colour is series three" wants a set laid out for a
/// legend; a reader looking for "is this worse than that" wants a path. Showing
/// them together invited each to be read as the other.
fn categorical_scales(mode: Option<&ModePalette>, prefix: &str, locale: Locale) -> Markup {
    html! {
        h3 class="reveal" { (t(locale, "Categorical scales", "Escalas categóricas")) }
        p class="section-lead reveal" {
            (t(
                locale,
                "Hues spread around the wheel rather than along a path, at equal perceptual \
                 intervals rather than equal angles — a fixed rotation through the greens \
                 changes appearance far less than the same rotation through the blues. Each \
                 set also spans a range of lightness, because hue is precisely the axis a \
                 dichromat is missing, and lightness is what is left.",
                "Matizes distribuídas ao redor do círculo, não ao longo de um caminho, em \
                 intervalos perceptuais iguais e não em ângulos iguais — uma rotação fixa pelos \
                 verdes muda muito menos a aparência que a mesma rotação pelos azuis. Cada \
                 conjunto também percorre uma faixa de luminosidade, porque a matiz é \
                 exatamente o eixo que falta a um dicromata, e a luminosidade é o que sobra.",
            ))
        }
        p class="section-lead reveal" {
            (t(
                locale,
                "Six is what a generated set can keep apart under all three dichromacies. A \
                 set marked labelled goes past that deliberately and says so: its legend has \
                 to name every series, and the measured margins are published in \
                 system/reports/colour-vision.md rather than assumed away.",
                "Seis é o que um conjunto gerado consegue manter distinguível sob as três \
                 dicromacias. Um conjunto marcado como rotulado passa disso de propósito e diz \
                 isso: sua legenda precisa nomear cada série, e as margens medidas são \
                 publicadas em system/reports/colour-vision.md em vez de ignoradas.",
            ))
        }
        @if let Some(m) = mode {
            @for (name, scale) in m.scales.iter().filter(|(_, s)| s.is_categorical()) {
                (scale_strip(name, scale, prefix))
            }
        }
    }
}

/// One scale as a row of stops, painted from its own tokens.
fn scale_strip(name: &str, scale: &Scale, prefix: &str) -> Markup {
    html! {
        div class="scale-strip reveal" {
            span class="scale-name" { code { (name) } }
            @for step in &scale.steps {
                span class="scale-stop"
                     style=(format!("background: var(--{prefix}-{name}-{});", step.role))
                     data-ink=(ink(step))
                     title=(format!("{name}-{}", step.role)) {
                    (step.role)
                }
            }
        }
    }
}

/// The translucency ladder, over two backdrops.
fn translucency(mode: Option<&ModePalette>, prefix: &str, locale: Locale) -> Markup {
    html! {
        h3 class="reveal" { (t(locale, "Translucency", "Translucidez")) }
        p class="section-lead reveal" {
            (t(
                locale,
                "Real alpha, not a hex solved to composite over one backdrop. Each stop is \
                 color-mix(in oklab, <token> N%, transparent), which is premultiplied — so it \
                 is the token at that opacity and composites correctly over anything. Shown \
                 here over two different surfaces for that reason.",
                "Alfa real, não um hex resolvido para compor sobre um fundo fixo. Cada passo é \
                 color-mix(in oklab, <token> N%, transparent), que é pré-multiplicado — então é \
                 o token naquela opacidade e compõe corretamente sobre qualquer coisa. Mostrado \
                 aqui sobre duas superfícies diferentes por esse motivo.",
            ))
        }
        p class="section-lead reveal" {
            (t(
                locale,
                "No contrast gate can audit these: contrast is a property of two opaque \
                 colours, and a wash has none until it is composited.",
                "Nenhuma verificação de contraste pode auditá-los: contraste é uma propriedade \
                 de duas cores opacas, e uma camada translúcida não tem nenhum antes de ser \
                 composta.",
            ))
        }
        @if let Some(m) = mode {
            @for backdrop in ["surface", "surface-raised"] {
                div class="alpha-ladder reveal"
                    style=(format!("background: var(--{prefix}-color-{backdrop});")) {
                    span class="scale-name" { code { (backdrop) } }
                    @for stem in m.alpha.keys() {
                        @if stem.starts_with("neutral-a") {
                            span class="alpha-stop"
                                 style=(format!("background: var(--{prefix}-{stem});"))
                                 title=(stem) {}
                        }
                    }
                }
            }
        }
    }
}

/// Light text on a dark step and vice versa, from the step's own lightness.
fn ink(step: &Step) -> &'static str {
    if step.primary().oklch.l > 0.6 {
        "dark"
    } else {
        "light"
    }
}

/// Whether a slot is a temperature of the page rather than a context.
fn is_surface_slot(slot: &str) -> bool {
    slot.strip_prefix("neutral")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('-'))
}

/// The contrast matrix, with failures obvious.
#[must_use]
pub fn contrast_matrix(palette: &Palette, locale: Locale) -> Markup {
    let theme = palette.default_theme();

    html! {
        section id="contrast" class="section" {
            div class="wrap" {
                h2 class="reveal" { (t(locale, "Contrast", "Contraste")) }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "Every pair an application actually ships, measured in APCA. Text on a \
                         card, a label on a filled button, a focus ring against the page. Most \
                         of these cross a family boundary, which is exactly where a per-family \
                         check cannot see.",
                        "Cada par que uma aplicação de fato entrega, medido em APCA. Texto sobre \
                         um cartão, rótulo sobre um botão preenchido, anel de foco sobre a \
                         página. A maioria cruza a fronteira entre famílias, que é exatamente \
                         onde uma verificação por família não enxerga.",
                    ))
                }

                p class="section-lead reveal" {
                    (t(
                        locale,
                        "Lc is APCA's lightness contrast, running from 0 to about 108. It is \
                         not a ratio and does not compare to WCAG's: 90 is the threshold for \
                         body text, 75 for larger body text, 60 for headlines, 45 for large or \
                         bold text, 30 for a non-text element that still has to be seen, and 15 \
                         is the floor at which anything is discernible at all.",
                        "Lc é o contraste de luminosidade do APCA, de 0 a cerca de 108. Não é \
                         uma razão e não se compara com a do WCAG: 90 é o limiar para texto \
                         corrido, 75 para texto corrido maior, 60 para títulos, 45 para texto \
                         grande ou em negrito, 30 para um elemento não textual que ainda \
                         precisa ser visto, e 15 é o piso em que qualquer coisa é perceptível.",
                    ))
                }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "The numbers below are measured in your browser, from the pixels this \
                         page is actually painted with — not read from a table. They are shown \
                         to four decimal places because that is what the palette is quantized \
                         to, so a pair that misses its target by a ten-thousandth says so \
                         rather than rounding to the number it missed. Expect the last decimal \
                         to differ from the compiler's: it measures its own values, this \
                         measures eight-bit pixels.",
                        "Os números abaixo são medidos no seu navegador, a partir dos pixels \
                         com que esta página é de fato pintada — não lidos de uma tabela. \
                         Aparecem com quatro casas decimais porque é a precisão em que a paleta \
                         é quantizada, então um par que erra o alvo por um décimo de milésimo \
                         diz isso em vez de arredondar para o número que não alcançou. Espere \
                         que a última casa difira da do compilador: ele mede os próprios \
                         valores, isto mede pixels de oito bits.",
                    ))
                }

                // As with the ramp groups above: both emitted, CSS picks.
                @for mode_name in ["light", "dark"] {
                    @if let Some(mode) = palette.mode(theme, mode_name) {
                        div class="matrix reveal" data-mode=(mode_name) {
                            (pair_grid(mode, locale))
                        }
                    }
                }

                p class="muted small reveal" {
                    (t(
                        locale,
                        "Computed live in the browser from the emitted tokens, using the same \
                         APCA implementation the compiler gates on.",
                        "Calculado ao vivo no navegador a partir dos tokens emitidos, usando a \
                         mesma implementação de APCA que o compilador usa como critério.",
                    ))
                }
            }
        }
    }
}

/// The pairs the contrast matrix shows, with the compiler's severity for each.
///
/// Exposed so `xtask` can check it against `noctua_check::contrast::PAIRS`.
/// This crate deliberately does not depend on the gates — it reads `system/` and
/// nothing else — so the two tables are kept in step by a test rather than by
/// a dependency.
#[must_use]
pub fn contrast_pairs() -> Vec<(&'static str, &'static str, f64, &'static str)> {
    PAIRS
        .iter()
        .map(|(fg, bg, minimum, _, severity)| (*fg, *bg, *minimum, *severity))
        .collect()
}

/// `(foreground, background, minimum Lc, purpose, severity)`.
///
/// A readable sample of the compiler's matrix, not all of it. The gate now
/// generates well over a hundred rows — one set per context, one per neutral
/// temperature — and a page that printed every one would be a spreadsheet
/// nobody scrolls. These are the pairs a reader is deciding about: can I set
/// text on this, can I see this border, is the focus ring visible.
///
/// Every row here must still exist in `noctua_check::contrast::pairs` with the
/// same threshold and severity; the xtask test
/// `the_site_and_the_gate_agree_on_every_pair` enforces exactly that, in the
/// one crate that can see both.
const PAIRS: [(&str, &str, f64, &str, &str); 12] = [
    ("fg", "surface", 90.0, "body text", "fail"),
    ("fg", "surface-raised", 90.0, "body text on a card", "fail"),
    ("fg-muted", "surface", 60.0, "secondary text", "fail"),
    (
        "on-accent",
        "accent",
        45.0,
        "the label on a filled button",
        "fail",
    ),
    (
        "ring",
        "surface",
        30.0,
        "a focus ring nobody can see is a keyboard trap",
        "fail",
    ),
    (
        "border-strong",
        "surface",
        15.0,
        "the strongest border must be visible",
        "fail",
    ),
    (
        "accent",
        "surface",
        45.0,
        "accent as text, which a component can mitigate",
        "warn",
    ),
    (
        "danger",
        "surface",
        30.0,
        "a semantic fill must be visible against the page",
        "fail",
    ),
    (
        "fg",
        "danger-bg",
        90.0,
        "body text inside a callout",
        "fail",
    ),
    (
        "on-danger",
        "danger",
        30.0,
        "a fill's own foreground must be visible on it",
        "fail",
    ),
    (
        "danger-border",
        "surface",
        8.0,
        "a status border must be visible",
        "warn",
    ),
    ("fg", "surface-cool", 90.0, "body text", "fail"),
];

/// The `for` column, translated.
///
/// Keyed on the English purpose because that is what the shared table holds —
/// the table is compared against `noctua_check::contrast::PAIRS`, so its
/// contents stay in the project's working language and only the display is
/// localized.
fn purpose_label(purpose: &str, locale: Locale) -> &str {
    match purpose {
        "body text" => t(locale, "body text", "texto corrido"),
        "body text on a card" => t(locale, "body text on a card", "texto sobre um cartão"),
        "secondary text" => t(locale, "secondary text", "texto secundário"),
        "the label on a filled button" => t(
            locale,
            "label on a filled button",
            "rótulo em botão preenchido",
        ),
        "a focus ring nobody can see is a keyboard trap" => t(locale, "focus ring", "anel de foco"),
        "the strongest border must be visible" => t(locale, "strongest border", "borda mais forte"),
        "accent as text, which a component can mitigate" => {
            t(locale, "accent as text", "destaque como texto")
        }
        "a semantic fill must be visible against the page" => {
            t(locale, "a semantic fill", "preenchimento semântico")
        }
        "body text inside a callout" => {
            t(locale, "text inside a callout", "texto dentro de um aviso")
        }
        "a fill's own foreground must be visible on it" => {
            t(locale, "a fill's foreground", "frente sobre preenchimento")
        }
        "a status border must be visible" => t(locale, "a status border", "borda de estado"),
        other => other,
    }
}

fn pair_grid(mode: &ModePalette, locale: Locale) -> Markup {
    // Rendered as data attributes and measured client-side, so the numbers on
    // the page cannot disagree with the tokens the page is painted with.
    //
    // The last column is the compiler's own severity for the pair, and it has
    // to travel with the row: a soft pair falling short is a judgement call
    // that a component can mitigate, and showing it with the same mark as a
    // real defect erases the distinction the gates are built around. The
    // xtask test `the_site_and_the_gate_agree_on_every_pair` checks this table
    // against `noctua_check::contrast::PAIRS`, because two copies drift.
    let pairs = PAIRS;

    html! {
        table class="contrast-table" {
            caption class="visually-hidden" {
                (t(
                    locale,
                    "APCA contrast for every shipped pair",
                    "Contraste APCA para cada par entregue",
                ))
            }
            thead {
                tr {
                    th scope="col" { (t(locale, "foreground", "frente")) }
                    th scope="col" { (t(locale, "background", "fundo")) }
                    th scope="col" { (t(locale, "for", "uso")) }
                    th scope="col" class="numeric" {
                        abbr title=(t(
                            locale,
                            "Lightness contrast, the APCA measure. Roughly 0 to 108.",
                            "Contraste de luminosidade, a medida do APCA. Vai de 0 a 108 \
                             aproximadamente.",
                        )) { "Lc" }
                    }
                    th scope="col" class="numeric" { (t(locale, "needs", "exige")) }
                }
            }
            tbody {
                @for (fg, bg, minimum, purpose, severity) in pairs {
                    @if mode.semantic.contains_key(fg) && mode.semantic.contains_key(bg) {
                        tr class="contrast-row" data-fg=(fg) data-bg=(bg)
                           data-min=(format!("{minimum}")) data-severity=(severity) {
                            td { code { (format!("--nc-color-{fg}")) } }
                            td { code { (format!("--nc-color-{bg}")) } }
                            td class="muted" { (purpose_label(purpose, locale)) }
                            td class="numeric measured" { "—" }
                            td class="numeric muted" { (format!("{minimum:.0}")) }
                        }
                    }
                }
            }
        }
    }
}

/// Realistic interface, rendered in the live theme.
#[must_use]
pub fn previews(_palette: &Palette, locale: Locale) -> Markup {
    html! {
        section id="previews" class="section" {
            div class="wrap" {
                h2 class="reveal" { (t(locale, "In context", "Em contexto")) }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "A palette is judged in use, not as a grid of squares. Everything below \
                         is built from the semantic tokens and follows whichever theme and mode \
                         is selected above.",
                        "Uma paleta se julga em uso, não como uma grade de quadrados. Tudo \
                         abaixo é construído com os tokens semânticos e acompanha o tema e o \
                         modo selecionados acima.",
                    ))
                }

                div class="preview-grid reveal" {
                    (deployment_card(locale))
                    (notifications_card(locale))
                    (form_card(locale))
                }
            }
        }
    }
}

fn deployment_card(locale: Locale) -> Markup {
    html! {
        article class="preview-card" {
            header class="preview-head" {
                h3 { (t(locale, "Deployment", "Implantação")) }
                span class="badge badge-success" {
                    (t(locale, "healthy", "saudável"))
                }
            }
            dl class="preview-stats" {
                div {
                    dt { (t(locale, "Requests", "Requisições")) }
                    dd { (t(locale, "18,402", "18.402")) }
                }
                div { dt { "p99" } dd { "42 ms" } }
                div {
                    dt { (t(locale, "Errors", "Erros")) }
                    dd { (t(locale, "0.02%", "0,02%")) }
                }
            }
            div class="preview-actions" {
                button type="button" class="button button-primary" {
                    (t(locale, "Deploy", "Implantar"))
                }
                button type="button" class="button" {
                    (t(locale, "History", "Histórico"))
                }
            }
        }
    }
}

fn notifications_card(locale: Locale) -> Markup {
    html! {
        article class="preview-card" {
            header class="preview-head" {
                h3 { (t(locale, "Notifications", "Notificações")) }
            }
            div class="callout callout-danger" {
                strong { (t(locale, "Build failed", "Compilação falhou")) }
                p {
                    (t(
                        locale,
                        "Three contrast targets were not met on the accent family.",
                        "Três alvos de contraste não foram atingidos na família \
                         accent.",
                    ))
                }
            }
            div class="callout callout-warning" {
                strong {
                    (t(locale, "Colour-vision margin", "Margem de visão de cores"))
                }
                p {
                    (t(
                        locale,
                        "success and danger sit close under deuteranopia.",
                        "success e danger ficam próximos sob deuteranopia.",
                    ))
                }
            }
            div class="callout callout-info" {
                strong { (t(locale, "Regenerated", "Regerado")) }
                p {
                    (t(
                        locale,
                        "29 files written to system/.",
                        "29 arquivos escritos em system/.",
                    ))
                }
            }
        }
    }
}

fn form_card(locale: Locale) -> Markup {
    html! {
        article class="preview-card" {
            header class="preview-head" {
                h3 { (t(locale, "New family", "Nova família")) }
            }
            form class="preview-form" onsubmit="return false" {
                label {
                    span { (t(locale, "Name", "Nome")) }
                    input type="text" value="accent" readonly;
                }
                label {
                    span { (t(locale, "Base hue", "Matiz base")) }
                    input type="text" value=(t(locale, "59.3", "59,3")) readonly;
                }
                label {
                    span { (t(locale, "Torsion", "Torção")) }
                    input type="text" value=(t(locale, "-7.0", "-7,0")) readonly;
                }
                div class="preview-actions" {
                    button type="button" class="button button-primary" {
                        (t(locale, "Save", "Salvar"))
                    }
                    button type="button" class="button button-quiet" {
                        (t(locale, "Cancel", "Cancelar"))
                    }
                }
            }
        }
    }
}

/// Copy-paste integration, per target.
#[must_use]
pub fn integration(locale: Locale) -> Markup {
    let targets: [(&str, &str, &str); 5] = [
        (
            "Plain CSS",
            "css",
            r#"<!-- three files: the dense grays, the semantic contract, and
     the default theme's values -->
<link rel="stylesheet" href="system/css/ramp.css">
<link rel="stylesheet" href="system/css/contexts.css">
<link rel="stylesheet" href="system/css/ochre-balanced.css">

<!-- or index.css: all of the above plus every other theme, and a name
     that survives a theme being renamed -->

.card {
  background: var(--nc-color-surface-raised);
  color: var(--nc-color-fg);
  border: 1px solid var(--nc-color-border);
}"#,
        ),
        (
            "Tailwind v4",
            "tailwind",
            r#"@import "tailwindcss";
@import "../noctua-colors/system/tailwind/theme.css";

<!-- then -->
<div class="bg-surface text-fg border-border">"#,
        ),
        (
            "Rust",
            "rust",
            r#"noctua-colors-tokens = { path = "../noctua-colors/system/rust" }

use noctua_colors_tokens::balanced::dark::accent;
let hex = accent::SOLID.hex;"#,
        ),
        (
            "TypeScript",
            "ts",
            r#"import { palette } from "../noctua-colors/system/ts/index.js";

const solid = palette.themes.balanced.light.families.accent.steps[8];
solid.renditions[0].hex;"#,
        ),
        (
            "QML",
            "qml",
            r#"import "."

Rectangle {
    color: NoctuaDark.surface
    border.color: NoctuaDark.border
}"#,
        ),
    ];

    html! {
        section id="integrate" class="section" {
            div class="wrap" {
                h2 class="reveal" { (t(locale, "Integrate", "Integrar")) }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "Generated artifacts are committed, so every consumption path works \
                         with no build step: submodule, subtree, sparse checkout, a plain copy, \
                         or a raw URL.",
                        "Os artefatos gerados são versionados, então todo caminho de consumo \
                         funciona sem etapa de build: submódulo, subtree, checkout esparso, \
                         cópia simples ou URL direta.",
                    ))
                }

                div class="tabs reveal" {
                    div class="tab-strip" role="tablist"
                        aria-label=(t(locale, "Integration targets", "Alvos de integração")) {
                        @for (index, (label, id, _)) in targets.iter().enumerate() {
                            button type="button" class="tab" role="tab"
                                   id=(format!("tab-{id}"))
                                   aria-controls=(format!("panel-{id}"))
                                   aria-selected=(if index == 0 { "true" } else { "false" })
                                   tabindex=(if index == 0 { "0" } else { "-1" }) {
                                (label)
                            }
                        }
                    }
                    @for (index, (_, id, code)) in targets.iter().enumerate() {
                        div class="tab-panel" role="tabpanel"
                            id=(format!("panel-{id}"))
                            aria-labelledby=(format!("tab-{id}"))
                            hidden[index != 0] {
                            pre { code { (PreEscaped(html_escape(code))) } }
                        }
                    }
                }
            }
        }
    }
}

/// Escapes text for embedding inside an element.
fn html_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The detail panel a swatch opens.
///
/// Populated client-side from the swatch's data attributes, so the numbers
/// shown are the ones the page is painted with.
#[must_use]
pub fn detail_panel(locale: Locale) -> Markup {
    html! {
        // The panel's contents are built client-side, so its labels travel as
        // data attributes rather than as markup — the script has no other way
        // to know which language it is rendering into.
        div id="detail" class="detail" hidden aria-live="polite"
            data-label-token=(t(locale, "token", "token"))
            data-label-lightness=(t(locale, "lightness", "luminosidade"))
            data-label-chroma=(t(locale, "chroma", "croma"))
            data-label-hue=(t(locale, "hue", "matiz"))
            data-label-relative=(t(locale, "relative chroma", "croma relativo"))
            data-label-headroom=(t(locale, "to gamut edge", "até a borda do gamut"))
            data-label-copy=(t(locale, "Copy as", "Copiar como"))
            data-label-close=(t(locale, "Close", "Fechar")) {}
    }
}

/// The opening of the compiler page.
///
/// It says who the page is for, out loud, because the reader who wanted a
/// stylesheet and landed here should be told in one sentence and given a way
/// back rather than left to work it out.
#[must_use]
pub fn how_it_works_intro(locale: Locale) -> Markup {
    html! {
        section class="hero hero-compact" {
            div class="wrap" {
                p class="eyebrow reveal" {
                    (t(locale, "the compiler", "o compilador"))
                }
                h1 class="reveal" {
                    (t(locale, "How these colours were made", "Como estas cores foram feitas"))
                }
                p class="lead reveal" {
                    (t(
                        locale,
                        "This page is about the program, not the palette. If you came for a \
                         stylesheet, everything you need is on the front page — this is here \
                         because colours are only worth trusting if the method is.",
                        "Esta página é sobre o programa, não sobre a paleta. Se você veio por \
                         uma folha de estilo, tudo o que precisa está na página inicial — isto \
                         existe porque cores só merecem confiança se o método merecer.",
                    ))
                }
                p class="hero-actions reveal" {
                    a class="button button-secondary" href=(locale.page("index")) {
                        (t(locale, "Back to the colours", "Voltar para as cores"))
                    }
                }
            }
        }
    }
}

/// What the gates measured, including what they could not fix.
///
/// The limit is stated as prominently as the capability. A colour system that
/// publishes only its successes is asking to be trusted on a claim nobody can
/// check; this one publishes the numbers where it falls short, because those
/// are the numbers that change what a reader should do.
#[must_use]
pub fn limits(locale: Locale) -> Markup {
    html! {
        section id="limits" class="section" {
            div class="wrap" {
                h2 class="reveal" {
                    (t(
                        locale,
                        "What the gates found, and could not fix",
                        "O que os portões encontraram, e não puderam corrigir",
                    ))
                }
                p class="section-lead reveal" {
                    (t(
                        locale,
                        "The quality gates were wired up after the emitters and immediately \
                         reported 175 failures against a palette that had passed everything \
                         else. The cause was structural: every semantic solid was anchored to \
                         the same contrast target, so they all landed at the same lightness and \
                         differed only in hue — and hue is precisely the axis dichromacy \
                         removes.",
                        "Os portões de qualidade foram ligados depois dos emissores e \
                         imediatamente relataram 175 falhas contra uma paleta que havia passado \
                         em todo o resto. A causa era estrutural: cada sólido semântico estava \
                         ancorado no mesmo alvo de contraste, então todos caíam na mesma \
                         luminosidade e diferiam apenas no matiz — e o matiz é exatamente o \
                         eixo que a dicromacia remove.",
                    ))
                }

                div class="prose reveal" {
                    p {
                        (t(
                            locale,
                            "Families now separate in lightness as well as hue, which is the \
                             only lever that survives. That is not a complete fix, and the \
                             system says so rather than pretending. Searched across every \
                             combination subject to fills staying visible and ramps staying \
                             sane, the best achievable worst-case separation for a six-family \
                             semantic set is ",
                            "As famílias agora se separam em luminosidade além do matiz, que é \
                             a única alavanca que sobrevive. Isso não é uma correção completa, \
                             e o sistema diz isso em vez de fingir. Buscando em todas as \
                             combinações, sujeitas a os preenchimentos permanecerem visíveis e \
                             as rampas permanecerem sãs, a melhor separação de pior caso \
                             alcançável para um conjunto semântico de seis famílias é ",
                        ))
                        strong { "0.0163" }
                        (t(
                            locale,
                            " — under one just-noticeable difference, and there are now ten \
                             families.",
                            " — abaixo de uma diferença minimamente perceptível, e agora são \
                             dez famílias.",
                        ))
                    }
                    p {
                        (t(
                            locale,
                            "So the gate reports margins and warns; it fails only when two \
                             colours are literally the same. Every number is published in ",
                            "Então o portão relata margens e avisa; ele falha apenas quando \
                             duas cores são literalmente iguais. Todos os números são \
                             publicados em ",
                        ))
                        code { "system/reports/colour-vision.md" }
                        (t(
                            locale,
                            ". This is the reason WCAG 1.4.1 exists: never convey information \
                             by colour alone. The palette gets you as far as colour can, and \
                             tells you exactly how far that is.",
                            ". Esta é a razão de a WCAG 1.4.1 existir: nunca transmita \
                             informação apenas por cor. A paleta leva você até onde a cor \
                             consegue, e diz exatamente até onde isso é.",
                        ))
                    }
                    p {
                        (t(
                            locale,
                            "The same measurement decided the categorical scale. Eight \
                             generated colours bottom out at 0.0416 separation; six reach \
                             0.0724. The default is six, and asking for more warns rather than \
                             silently shipping a chart a dichromat cannot read.",
                            "A mesma medição decidiu a escala categórica. Oito cores geradas \
                             chegam ao fundo em 0,0416 de separação; seis alcançam 0,0724. O \
                             padrão é seis, e pedir mais gera um aviso em vez de publicar \
                             silenciosamente um gráfico que um dicromata não consegue ler.",
                        ))
                    }
                }
            }
        }
    }
}

/// Why the colours can be trusted, on the product page, kept short.
///
/// The full argument is a page of its own; this is the paragraph that earns
/// the click, and the link that offers it.
#[must_use]
pub fn trust(palette: &Palette, locale: Locale) -> Markup {
    html! {
        section id="trust" class="section" {
            div class="wrap" {
                h2 class="reveal" {
                    (t(
                        locale,
                        "Why these colours are trustworthy",
                        "Por que estas cores são confiáveis",
                    ))
                }
                div class="prose reveal" {
                    p {
                        (t(
                            locale,
                            "Nothing here was picked by eye. A colour's saturation is stored as \
                             a fraction of the most a display can actually show at that \
                             lightness and hue, so the same token is richer on a wide-gamut \
                             screen without being redefined. And a step's lightness is solved \
                             from a contrast target using APCA, which — unlike WCAG 2.x — \
                             models polarity, and so does not rate a too-weak dark-mode pair as \
                             better than a comfortable light-mode one.",
                            "Nada aqui foi escolhido a olho. A saturação de uma cor é \
                             armazenada como uma fração do máximo que uma tela consegue \
                             realmente exibir naquela luminosidade e matiz, então o mesmo token \
                             fica mais rico em uma tela de gamut amplo sem ser redefinido. E a \
                             luminosidade de um passo é resolvida a partir de um alvo de \
                             contraste usando APCA, que — ao contrário do WCAG 2.x — modela \
                             polaridade, e portanto não avalia um par escuro fraco demais como \
                             melhor que um par claro confortável.",
                        ))
                    }
                    p {
                        (t(
                            locale,
                            "Every build runs 48,441 checks across every palette, mode and \
                             pair, and fails on a regression. Colour-vision simulation runs \
                             too — and where a pair genuinely cannot be told apart, that is \
                             published rather than hidden.",
                            "Cada build executa 48.441 verificações em todas as paletas, modos \
                             e pares, e falha em uma regressão. A simulação de visão de cores \
                             também roda — e onde um par realmente não pode ser distinguido, \
                             isso é publicado em vez de escondido.",
                        ))
                    }
                    p {
                        (t(
                            locale,
                            "There are ",
                            "São ",
                        ))
                        strong { (palette.themes.len().to_string()) }
                        (t(
                            locale,
                            " palettes, and every one of them went through the same gates.",
                            " paletas, e todas passaram pelos mesmos portões.",
                        ))
                    }
                    p {
                        a class="button button-secondary" href=(locale.page("how-it-works")) {
                            (t(locale, "How it works, in full", "Como funciona, na íntegra"))
                        }
                    }
                }
            }
        }
    }
}
