//! The header controls: theme, mode and language.
//!
//! # Icons are inline
//!
//! Every icon here is an inline `<svg>` drawn with `currentColor` and no
//! fill of its own. Inline because a sprite sheet or an icon font is another
//! request before the header can paint, and `currentColor` because an icon
//! with a color baked into it would be the one thing on this site that did
//! not come out of the compiler.
//!
//! # The mode control is three states, not two
//!
//! A two-position switch cannot express "follow the operating system", so a
//! visitor who wanted that had no way back once they touched it. Three
//! explicit choices — light, dark, system — make the third a destination
//! rather than the absence of a decision, and `system` is what a fresh
//! visitor gets.

use maud::{Markup, PreEscaped, html};

use crate::Palette;
use crate::i18n::{Locale, t};

/// Stroke-drawn icon body, sized and coloured by the CSS around it.
fn icon(paths: &'static str) -> Markup {
    html! {
        svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor"
            stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"
            aria-hidden="true" focusable="false" {
            (PreEscaped(paths))
        }
    }
}

/// Overlapping swatches: the palette being chosen.
fn theme_icon() -> Markup {
    icon(r#"<circle cx="9" cy="9" r="5"/><circle cx="15" cy="15" r="5"/>"#)
}

fn sun_icon() -> Markup {
    icon(
        r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>"#,
    )
}

fn moon_icon() -> Markup {
    icon(r#"<path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5z"/>"#)
}

/// A display: the system deciding rather than the visitor.
fn system_icon() -> Markup {
    icon(r#"<rect x="2.5" y="4" width="19" height="13" rx="2"/><path d="M8 21h8M12 17v4"/>"#)
}

fn globe_icon() -> Markup {
    icon(
        r#"<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a14 14 0 0 1 0 18 14 14 0 0 1 0-18z"/>"#,
    )
}

/// The palette pickers.
///
/// Two controls when the spec describes two axes — an accent hue and a
/// saturation — because that is what the palettes actually vary along.
/// Offering thirty-six names in one list would make the reader reconstruct
/// the grid in their head.
///
/// Falls back to a single list of theme names when the spec writes its themes
/// out by hand and there are no axes to offer.
pub(crate) fn palette_controls(palette: &Palette, locale: Locale) -> Markup {
    if palette.axes.is_grid() {
        html! {
            (axis_select(
                "accent-select",
                &palette.axes.accents,
                &default_accent(palette),
                t(locale, "Accent color", "Cor de destaque"),
                locale,
                accent_label,
                true,
            ))
            (axis_select(
                "saturation-select",
                &palette.axes.saturations,
                &default_saturation(palette),
                t(locale, "Saturation", "Saturação"),
                locale,
                saturation_label,
                false,
            ))
        }
    } else {
        let names = palette.theme_names();
        let default = palette.default_theme().to_owned();
        html! {
            (axis_select(
                "palette-select",
                &names.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
                &default,
                t(locale, "Color theme", "Tema de cores"),
                locale,
                saturation_label,
                true,
            ))
        }
    }
}

/// Which accent the default palette uses.
fn default_accent(palette: &Palette) -> String {
    let default = palette.default_theme();
    palette
        .axes
        .accents
        .iter()
        .find(|accent| {
            palette
                .axes
                .saturations
                .iter()
                .any(|s| palette.axes.theme(accent, s) == Some(default))
        })
        .cloned()
        .unwrap_or_default()
}

/// Which saturation the default palette uses.
fn default_saturation(palette: &Palette) -> String {
    let default = palette.default_theme();
    palette
        .axes
        .saturations
        .iter()
        .find(|saturation| {
            palette
                .axes
                .accents
                .iter()
                .any(|a| palette.axes.theme(a, saturation) == Some(default))
        })
        .cloned()
        .unwrap_or_default()
}

/// One axis as a select.
///
/// The visible label is gone and the icon carries the meaning, so the select
/// still needs an accessible name — `aria-label` supplies one, and the icon is
/// hidden from assistive technology rather than read as decoration.
fn axis_select(
    id: &str,
    options: &[String],
    selected: &str,
    label: &str,
    locale: Locale,
    render: fn(&str, Locale) -> &str,
    with_icon: bool,
) -> Markup {
    html! {
        div class="select-shell" {
            select id=(id) class="select" aria-label=(label) {
                @for name in options {
                    option value=(name) selected[name == selected] {
                        (render(name, locale))
                    }
                }
            }
            span class="select-affix" aria-hidden="true" {
                @if with_icon { (theme_icon()) }
                span class="select-chevron" {
                    (icon(r#"<path d="M6 9l6 6 6-6"/>"#))
                }
            }
        }
    }
}

/// What an accent is called on screen.
///
/// The spec's names are identifiers; these are the words for them. Anything
/// the spec adds later falls through to its own name rather than vanishing.
fn accent_label(name: &str, locale: Locale) -> &str {
    match name {
        "ochre" => t(locale, "ochre", "ocre"),
        "amber" => t(locale, "amber", "âmbar"),
        "lime" => t(locale, "lime", "lima"),
        "jade" => t(locale, "jade", "jade"),
        "teal" => t(locale, "teal", "verde-azulado"),
        "azure" => t(locale, "azure", "azul-celeste"),
        "blue" => t(locale, "blue", "azul"),
        "indigo" => t(locale, "indigo", "índigo"),
        "violet" => t(locale, "violet", "violeta"),
        "magenta" => t(locale, "magenta", "magenta"),
        "rose" => t(locale, "rose", "rosa"),
        "clay" => t(locale, "clay", "terracota"),
        "umber" => t(locale, "umber", "umbro"),
        other => other,
    }
}

/// What a saturation is called on screen.
fn saturation_label(name: &str, locale: Locale) -> &str {
    match name {
        "balanced" => t(locale, "balanced", "equilibrado"),
        "vivid" => t(locale, "vivid", "vívido"),
        "sober" => t(locale, "sober", "sóbrio"),
        other => other,
    }
}

/// Light, dark, or whatever the operating system says.
pub(crate) fn mode_control(locale: Locale) -> Markup {
    let modes = [
        ("light", t(locale, "Light", "Claro"), sun_icon()),
        ("dark", t(locale, "Dark", "Escuro"), moon_icon()),
        ("system", t(locale, "System", "Sistema"), system_icon()),
    ];

    html! {
        div class="mode-control" role="group"
            aria-label=(t(locale, "Appearance", "Aparência")) {
            @for (value, label, glyph) in modes {
                // `system` is pressed to begin with: an untouched visitor is
                // following their operating system, and the control has to
                // say so rather than claim a choice they did not make.
                button type="button" class="mode-option" data-mode=(value)
                       aria-pressed=(if value == "system" { "true" } else { "false" })
                       title=(label) {
                    (glyph)
                    span class="visually-hidden" { (label) }
                }
            }
        }
    }
}

/// The language switch.
///
/// A link, not a button: the two languages are two documents, so this is
/// navigation. It works with script disabled, it can be opened in a new tab,
/// and the address bar always says which language you are reading.
///
/// # It shows the language you are in, not the one you would get
///
/// The code reads `EN` on the English page. Showing the *destination* made the
/// control ambiguous — `PT` beside a globe reads equally well as "this page is
/// Portuguese" and as "press for Portuguese", and the two readings are
/// opposites. Showing the current language removes that: it states a fact
/// about the page rather than making a claim the reader has to interpret.
///
/// The cost is that the affordance is no longer in the visible text, so it
/// moves into the accessible name and the tooltip, both of which spell out the
/// action. `hreflang` still describes the destination, and the endonym carries
/// its own `lang` so a screen reader pronounces "Português" in Portuguese
/// rather than reading it as English.
pub(crate) fn language_switch(locale: Locale, base: &str) -> Markup {
    let other = locale.other();
    let action = format!(
        "{}{}",
        t(locale, "Switch to ", "Mudar para "),
        other.endonym()
    );

    html! {
        a class="lang-switch" href=(other.page(base)) hreflang=(other.tag())
          title=(action)
          data-locale=(other.tag()) {
            (globe_icon())
            span class="lang-code" { (locale.short()) }
            span class="visually-hidden" {
                (t(locale, "Switch to ", "Mudar para "))
                span lang=(other.tag()) { (other.endonym()) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_control_offers_all_three_states() {
        let html = mode_control(Locale::En).into_string();
        for mode in ["light", "dark", "system"] {
            assert!(
                html.contains(&format!("data-mode=\"{mode}\"")),
                "{mode} missing"
            );
        }
    }

    /// Without a decision from the visitor, the operating system decides —
    /// and the control has to show that rather than a state nobody chose.
    #[test]
    fn system_is_the_state_shown_before_any_choice() {
        let html = mode_control(Locale::En).into_string();
        assert_eq!(html.matches("aria-pressed=\"true\"").count(), 1);
        let at = html.find("data-mode=\"system\"").expect("a system option");
        assert!(
            html[at..].starts_with("data-mode=\"system\" aria-pressed=\"true\""),
            "system is not the pressed option"
        );
    }

    #[test]
    fn every_icon_only_button_still_has_a_name() {
        let html = mode_control(Locale::Pt).into_string();
        // Three buttons, each with visually hidden text and a title.
        assert_eq!(html.matches("visually-hidden").count(), 3);
        assert!(html.contains("Claro") && html.contains("Escuro") && html.contains("Sistema"));
    }

    /// The visible code names the page you are on; the link goes to the other
    /// one. Showing the destination instead made the control ambiguous —
    /// `PT` reads just as well as "this page is Portuguese".
    #[test]
    fn the_switch_shows_the_current_language_and_links_to_the_other() {
        let en = language_switch(Locale::En, "index").into_string();
        assert!(en.contains(">EN<"), "the English page must show EN: {en}");
        assert!(
            !en.contains(">PT<"),
            "it must not show the destination: {en}"
        );
        assert!(en.contains("href=\"index.pt.html\""), "{en}");
        assert!(en.contains("hreflang=\"pt-BR\""), "{en}");

        let pt = language_switch(Locale::Pt, "index").into_string();
        assert!(
            pt.contains(">PT<"),
            "the Portuguese page must show PT: {pt}"
        );
        assert!(!pt.contains(">EN<"), "{pt}");
        assert!(pt.contains("href=\"index.html\""), "{pt}");
    }

    /// With the destination gone from the visible text, the only thing saying
    /// this is a control is its name — so that has to state the action.
    #[test]
    fn the_switch_still_says_what_pressing_it_does() {
        let en = language_switch(Locale::En, "index").into_string();
        assert!(en.contains("title=\"Switch to Português\""), "{en}");
        assert!(
            en.contains("Switch to "),
            "the accessible name must name the action"
        );
        // The endonym is tagged so a screen reader pronounces it correctly.
        assert!(en.contains("lang=\"pt-BR\">Português"), "{en}");

        let pt = language_switch(Locale::Pt, "index").into_string();
        assert!(pt.contains("title=\"Mudar para English\""), "{pt}");
        assert!(pt.contains("lang=\"en\">English"), "{pt}");
    }

    #[test]
    fn the_switch_follows_the_page_it_is_on() {
        let html = language_switch(Locale::En, "playground").into_string();
        assert!(html.contains("playground.pt.html"), "{html}");
    }

    /// Icons must inherit their color. One with a value baked in would be the
    /// only thing on this site that did not come out of the compiler.
    #[test]
    fn icons_take_their_color_from_the_text_around_them() {
        let html = mode_control(Locale::En).into_string();
        assert!(html.contains("stroke=\"currentColor\""));
        assert!(html.contains("fill=\"none\""));
    }

    /// The spec can add an accent or a saturation at any time; an unnamed one
    /// must appear under its own identifier rather than vanish.
    #[test]
    fn an_unknown_axis_value_still_shows_its_name() {
        assert_eq!(accent_label("experimental", Locale::Pt), "experimental");
        assert_eq!(accent_label("blue", Locale::Pt), "azul");
        assert_eq!(saturation_label("experimental", Locale::Pt), "experimental");
        assert_eq!(saturation_label("vivid", Locale::Pt), "vívido");
    }

    /// Every accent the shipped spec offers has a Portuguese name, or the
    /// translated page quietly falls back to English for it.
    #[test]
    fn every_shipped_accent_is_translated() {
        for accent in [
            "ochre", "amber", "lime", "jade", "teal", "azure", "blue", "indigo", "violet",
            "magenta", "rose", "clay", "umber",
        ] {
            let pt = accent_label(accent, Locale::Pt);
            // `magenta` is the same word in both, so identity is only a
            // failure when the English differs.
            if !matches!(accent, "magenta" | "jade") {
                assert_ne!(pt, accent, "{accent} has no Portuguese name");
            }
        }
    }
}
