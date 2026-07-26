//! Two languages, chosen at build time.
//!
//! # Why the translations sit inline
//!
//! Every translatable string is written as `t(locale, "English", "Português")`
//! at the point it is used, rather than as a key looked up in a table
//! elsewhere. A key registry drifts: a string gets reworded in one language,
//! a key is renamed and one of its two entries is missed, and the failure is a
//! page that silently shows a key name or the wrong language. Here both
//! versions are arguments to the same call — one cannot exist without the
//! other, the compiler enforces it, and a reviewer reads the pair together.
//!
//! # Why the pages are rendered rather than switched
//!
//! Each locale is a complete, separate HTML file, built by the same
//! generator. The alternative — shipping a dictionary and rewriting the DOM
//! on load — costs a flash of the wrong language on every visit, breaks the
//! page for anyone without script, and makes the rendered output depend on
//! runtime state. Rendering twice costs a few hundred kilobytes of disk and
//! nothing at all at runtime.
//!
//! # Why the files are siblings
//!
//! `index.pt.html` next to `index.html`, not `pt/index.html`. Every asset on
//! this site is referenced relatively, and a subdirectory would mean every
//! one of those paths needing a `../` that is correct in one locale and wrong
//! in the other. Siblings share a directory, so the paths are simply the same.

/// A language the site is published in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    /// English. The project's working language and the default.
    En,
    /// Brazilian Portuguese.
    Pt,
}

impl Locale {
    /// Both locales, in the order they are built.
    #[must_use]
    pub const fn all() -> [Self; 2] {
        [Self::En, Self::Pt]
    }

    /// The BCP 47 tag for the `lang` attribute.
    ///
    /// `pt-BR` rather than `pt`: the translation is Brazilian, and a screen
    /// reader picks its voice and its pronunciation from this.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Pt => "pt-BR",
        }
    }

    /// The short label shown in the language switch.
    #[must_use]
    pub const fn short(self) -> &'static str {
        match self {
            Self::En => "EN",
            Self::Pt => "PT",
        }
    }

    /// This language's own name for itself, for the switch's accessible name.
    ///
    /// An endonym, deliberately: someone looking for Portuguese is looking
    /// for "Português", not for "Portuguese".
    #[must_use]
    pub const fn endonym(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Pt => "Português",
        }
    }

    /// Whether this locale owns the unsuffixed URL.
    ///
    /// Only the default page may redirect to a stored preference. Every other
    /// page's URL names a language explicitly, and an explicit request has to
    /// win — otherwise a Portuguese link sent to someone whose preference is
    /// English opens in English, and the link is useless for the one thing
    /// links are for.
    #[must_use]
    pub const fn is_default(self) -> bool {
        matches!(self, Self::En)
    }

    /// The other locale.
    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::En => Self::Pt,
            Self::Pt => Self::En,
        }
    }

    /// Turns a base file name into this locale's file name.
    ///
    /// English keeps the plain name so the default URL stays clean, and every
    /// other locale is suffixed before the extension.
    #[must_use]
    pub fn page(self, base: &str) -> String {
        match self {
            Self::En => format!("{base}.html"),
            Self::Pt => format!("{base}.pt.html"),
        }
    }
}

/// Picks the string for a locale.
///
/// Both translations are required arguments, so neither can be forgotten.
#[must_use]
pub const fn t(locale: Locale, en: &'static str, pt: &'static str) -> &'static str {
    match locale {
        Locale::En => en,
        Locale::Pt => pt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_locale_picks_its_own_string() {
        assert_eq!(t(Locale::En, "Palette", "Paleta"), "Palette");
        assert_eq!(t(Locale::Pt, "Palette", "Paleta"), "Paleta");
    }

    #[test]
    fn english_keeps_the_plain_file_name() {
        assert_eq!(Locale::En.page("index"), "index.html");
        assert_eq!(Locale::En.page("playground"), "playground.html");
    }

    /// Siblings, not subdirectories: the relative asset paths in the two
    /// pages have to be identical or one locale loads a broken stylesheet.
    #[test]
    fn every_locale_lives_in_the_same_directory() {
        for locale in Locale::all() {
            let page = locale.page("index");
            assert!(!page.contains('/'), "{page} is not a sibling");
        }
    }

    /// Exactly one locale can own the plain URL, and it has to be the one
    /// whose file name has no suffix.
    #[test]
    fn one_locale_owns_the_unsuffixed_url() {
        let defaults: Vec<Locale> = Locale::all()
            .into_iter()
            .filter(|l| l.is_default())
            .collect();
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].page("index"), "index.html");
    }

    #[test]
    fn the_other_locale_round_trips() {
        for locale in Locale::all() {
            assert_eq!(locale.other().other(), locale);
            assert_ne!(locale.other(), locale);
        }
    }

    #[test]
    fn brazilian_portuguese_is_tagged_as_such() {
        // `pt` alone would let a screen reader choose European Portuguese.
        assert_eq!(Locale::Pt.tag(), "pt-BR");
    }
}
