//! Turning canonical names into each target's spelling.
//!
//! Role names are canonical in kebab-case — `bg-app`, `text-strong` — because
//! that is what CSS wants and CSS is the widest consumer. Every other target
//! converts from it, and the conversion lives here rather than in seven
//! emitters that would each drift.

/// `bg-element-hover` becomes `bgElementHover`, for QML and JavaScript.
#[must_use]
pub fn camel(kebab: &str) -> String {
    let mut out = String::with_capacity(kebab.len());
    let mut capitalize = false;
    for ch in kebab.chars() {
        if ch == '-' || ch == '_' {
            capitalize = true;
        } else if capitalize {
            out.extend(ch.to_uppercase());
            capitalize = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Words QML's JavaScript dialect will not accept as a property name.
///
/// Only the ones a colour token could plausibly be called. `new` is not
/// hypothetical — it is in the shipped specification, as the context for a
/// newly-arrived item — and `readonly property color new` is a syntax error, not
/// a warning: the whole singleton fails to load and every token in it goes with
/// it.
const QML_RESERVED: &[&str] = &[
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

/// A QML property name: camel-case, and never a reserved word.
///
/// A reserved word gets a trailing underscore — `new` becomes `new_`. Renaming
/// it to something prettier would mean the QML name no longer derives from the
/// token name, and then a Qt author has to consult a table to find out what a
/// token is called here.
#[must_use]
pub fn qml_property(kebab: &str) -> String {
    let mut name = camel(kebab);
    if QML_RESERVED.contains(&name.as_str()) {
        name.push('_');
    }
    name
}

/// `bg-element-hover` becomes `BgElementHover`, for QML singleton file names.
#[must_use]
pub fn pascal(kebab: &str) -> String {
    let camel = camel(kebab);
    let mut chars = camel.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// `bg-element-hover` becomes `BG_ELEMENT_HOVER`, for Rust constants.
#[must_use]
pub fn screaming_snake(kebab: &str) -> String {
    kebab.replace('-', "_").to_uppercase()
}

/// `bg-element-hover` becomes `bg_element_hover`, for Rust modules.
#[must_use]
pub fn snake(kebab: &str) -> String {
    kebab.replace('-', "_").to_lowercase()
}

/// Makes a name safe to use as an identifier in generated code.
///
/// Family and theme names come from a spec a human wrote, so they can contain
/// anything TOML allows as a key. Anything that is not alphanumeric becomes an
/// underscore, and a leading digit gets a prefix, because `1st` is not an
/// identifier in any of the target languages.
#[must_use]
pub fn identifier(raw: &str) -> String {
    let mut out: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

/// Escapes a string for embedding in a double-quoted JSON or JavaScript
/// literal.
#[must_use]
pub fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                write!(out, "\\u{:04x}", c as u32).expect("string write");
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_becomes_camel() {
        assert_eq!(camel("bg-app"), "bgApp");
        assert_eq!(camel("bg-element-hover"), "bgElementHover");
        assert_eq!(camel("solid"), "solid");
        assert_eq!(camel(""), "");
    }

    /// `readonly property color new` does not compile, and the failure takes
    /// the whole singleton with it.
    #[test]
    fn a_reserved_word_cannot_reach_a_qml_property_name() {
        assert_eq!(qml_property("new"), "new_");
        assert_eq!(qml_property("default"), "default_");
        assert_eq!(qml_property("accent-hover"), "accentHover");
        for word in QML_RESERVED {
            assert_ne!(&qml_property(word), word, "{word} escaped unescaped");
        }
    }

    #[test]
    fn kebab_becomes_pascal() {
        assert_eq!(pascal("bg-app"), "BgApp");
        assert_eq!(pascal("noctua"), "Noctua");
        assert_eq!(pascal(""), "");
    }

    #[test]
    fn kebab_becomes_screaming_snake() {
        assert_eq!(screaming_snake("bg-app"), "BG_APP");
        assert_eq!(screaming_snake("text-strong"), "TEXT_STRONG");
    }

    #[test]
    fn kebab_becomes_snake() {
        assert_eq!(snake("bg-app"), "bg_app");
        assert_eq!(snake("Accent"), "accent");
    }

    #[test]
    fn identifiers_survive_whatever_a_spec_author_wrote() {
        assert_eq!(identifier("accent"), "accent");
        assert_eq!(identifier("brand accent"), "brand_accent");
        assert_eq!(identifier("café"), "caf_");
        assert_eq!(identifier("2024"), "_2024");
        assert_eq!(identifier(""), "_");
        assert_eq!(identifier("a-b"), "a_b");
    }

    /// Every generated identifier must be usable in every target language.
    #[test]
    fn generated_identifiers_are_always_valid() {
        for raw in [
            "accent",
            "brand accent",
            "2024",
            "",
            "a-b-c",
            "!!!",
            "Ünïcödé",
        ] {
            let id = identifier(raw);
            assert!(!id.is_empty());
            assert!(
                !id.chars().next().expect("non-empty").is_ascii_digit(),
                "{raw} produced {id}, which starts with a digit"
            );
            assert!(
                id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "{raw} produced {id}, which is not an identifier"
            );
        }
    }

    #[test]
    fn strings_are_escaped_for_embedding() {
        assert_eq!(escape("plain"), "plain");
        assert_eq!(escape("say \"hi\""), "say \\\"hi\\\"");
        assert_eq!(escape("a\\b"), "a\\\\b");
        assert_eq!(escape("line\nbreak"), "line\\nbreak");
        assert_eq!(escape("\u{1}"), "\\u0001");
    }
}
