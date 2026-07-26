//! Every token a consumer names must be a token the compiler ships.
//!
//! This is the gate that makes the export path safe to change. A role renamed
//! in the spec, a family removed, a prefix adjusted — each of those quietly
//! turns `var(--color-accent)` into nothing at all. CSS has no error for an
//! undefined custom property: the declaration is simply dropped, and the
//! element renders with whatever it inherited. Text goes the color of its
//! parent, a button loses its fill, and nothing anywhere reports a problem.
//!
//! So the references are checked against the generated stylesheet, and the
//! failure that would have been silent becomes a failed build instead.
//!
//! Only *this* repository's own consumers are checked — the docs site and the
//! examples. Sibling projects are checked when they build, against whichever
//! version of `dist/` they have.
//!
//! # What this does and does not prove
//!
//! The token set is the union of every stylesheet the compiler emits, because
//! the claim being checked is "this name is a token the compiler ships". It
//! is not "this name is reachable from the one file this consumer happens to
//! link", which would mean resolving each consumer's own `@import` graph. A
//! consumer that links one theme file alone and then asks for a token defined
//! in `ramp.css` would pass this gate and still render wrong — so consumers
//! link `index.css`, which imports all of them, and the guide says so.

use std::collections::BTreeSet;
use std::path::Path;

use crate::{Finding, Report, Severity};

const GATE: &str = "references";

/// Where consumers live, relative to the repository root.
const CONSUMERS: &[&str] = &["docs-site", "examples"];

/// Directories inside those that hold generated output rather than sources.
const GENERATED: &[&str] = &["public", "vendor", "target", "node_modules"];

/// Checks every custom-property reference under the consumer directories
/// against the tokens the compiler emitted.
///
/// `stylesheets` are the generated files, read rather than regenerated so the
/// gate tests what actually shipped.
#[must_use]
pub fn check<'a>(root: &Path, stylesheets: impl IntoIterator<Item = &'a str>) -> Report {
    let mut report = Report::default();
    let defined: BTreeSet<String> = stylesheets.into_iter().flat_map(defined_tokens).collect();

    if defined.is_empty() {
        report.findings.push(Finding {
            gate: GATE,
            severity: Severity::Fail,
            where_: "dist/css".to_owned(),
            message: "no custom properties found; the stylesheets did not build".to_owned(),
            margin: None,
        });
        return report;
    }

    for directory in CONSUMERS {
        let path = root.join(directory);
        if !path.is_dir() {
            continue;
        }
        walk(&path, root, &defined, &mut report);
    }

    report
}

/// The custom properties the stylesheet defines.
///
/// A definition is `--name:`; a *use* is `var(--name)`. The generated files
/// contain both — the numeric aliases resolve to the canonical names — and
/// the colon is what tells them apart, since a use is always followed by `)`
/// or a comma.
///
/// Scanned anywhere on the line rather than only at the start, so a
/// minified or single-line rule is read the same as a pretty-printed one.
fn defined_tokens(stylesheet: &str) -> BTreeSet<String> {
    let mut defined = BTreeSet::new();

    for line in stylesheet.lines() {
        let mut rest = line;
        while let Some(at) = rest.find("--") {
            rest = &rest[at..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            rest = &rest[name.len()..];

            if name.len() > 2 && rest.starts_with(':') {
                defined.insert(name);
            }
        }
    }

    defined
}

fn walk(directory: &Path, root: &Path, defined: &BTreeSet<String>, report: &mut Report) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };

    // Sorted, so the same tree always produces the same report.
    let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();

    for path in paths {
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        if path.is_dir() {
            if !GENERATED.contains(&name.as_ref()) {
                walk(&path, root, defined, report);
            }
            continue;
        }

        let interesting = matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("css" | "html" | "js" | "qml" | "scss")
        );
        if !interesting {
            continue;
        }

        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };

        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        for (number, line) in text.lines().enumerate() {
            report.checked += 1;
            for reference in references(line) {
                // A consumer may define its own non-color variables — spacing,
                // radii — and those are its business. Only the palette's
                // namespace is the compiler's contract.
                if !is_palette_token(&reference) || defined.contains(&reference) {
                    continue;
                }
                report.findings.push(Finding {
                    gate: GATE,
                    severity: Severity::Fail,
                    where_: format!("{relative}:{}", number + 1),
                    message: format!(
                        "`{reference}` is not a token the compiler ships. \
                         CSS drops an undefined custom property silently, so this \
                         renders as an inherited color rather than as an error"
                    ),
                    margin: None,
                });
            }
        }
    }
}

/// Whether a name belongs to the compiler's output rather than the consumer's
/// own variables.
///
/// A name ending in a hyphen is a prefix, not a token — it comes from prose
/// in a comment, or from a stylesheet built by string concatenation in
/// script. Neither is a reference this gate can resolve, and treating them as
/// one would mean the gate cries wolf on every file that documents itself.
fn is_palette_token(name: &str) -> bool {
    !name.ends_with('-') && (name.starts_with("--color-") || name.starts_with("--nc-"))
}

/// Reads every stylesheet in a directory, sorted so the result is stable.
///
/// Returns an empty vector when the directory is missing, which the caller
/// turns into a failure rather than a silent pass.
#[must_use]
pub fn read_stylesheets(directory: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut paths: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "css"))
        .collect();
    paths.sort();

    paths
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect()
}

/// Every `var(--name)` on a line.
fn references(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = line;

    while let Some(at) = rest.find("var(") {
        rest = &rest[at + 4..];
        let name: String = rest
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if name.starts_with("--") {
            found.push(name);
        }
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLESHEET: &str = "\
:root {
  --color-accent: oklch(0.66 0.1 58);
  --color-fg: oklch(0.28 0.002 59);
  --nc-accent-solid: oklch(0.66 0.1 58);
  --color-accent-9: var(--nc-accent-solid);
}
";

    #[test]
    fn definitions_are_told_apart_from_uses() {
        let defined: BTreeSet<String> = defined_tokens(STYLESHEET);
        assert!(defined.contains("--color-accent"));
        assert!(defined.contains("--nc-accent-solid"));
        assert!(defined.contains("--color-accent-9"));
        assert_eq!(defined.len(), 4, "{defined:?}");
    }

    #[test]
    fn references_are_read_in_every_shape_a_stylesheet_uses() {
        assert_eq!(references("color: var(--color-fg);"), ["--color-fg"]);
        assert_eq!(
            references("border: 1px solid var( --color-border )"),
            ["--color-border"]
        );
        assert_eq!(
            references(
                "background: color-mix(in oklab, var(--color-accent) 20%, var(--color-surface))"
            ),
            ["--color-accent", "--color-surface"]
        );
        assert!(references("color: red").is_empty());
    }

    #[test]
    fn a_consumers_own_variables_are_its_own_business() {
        assert!(!is_palette_token("--gap"));
        assert!(!is_palette_token("--radius"));
        assert!(is_palette_token("--color-accent"));
        assert!(is_palette_token("--nc-accent-solid"));
    }

    /// Both of these appear in this repository, and neither is a reference.
    #[test]
    fn a_bare_prefix_is_not_a_reference() {
        // From a comment: "every color here is a `var(--color-*)`".
        assert!(!is_palette_token("--color-"));
        // From script: "var(--color-" + token + ")".
        assert!(!is_palette_token("--nc-"));
    }

    /// The gate must notice its own stylesheet failing to build, rather than
    /// reporting a clean run over zero tokens.
    #[test]
    fn an_empty_stylesheet_is_a_failure_not_a_pass() {
        let report = check(Path::new("."), [""]);
        assert!(!report.is_ok());
        assert!(
            report.summary().contains("did not build"),
            "{}",
            report.summary()
        );
    }

    /// A token defined in one emitted file counts as shipped even when the
    /// reference is checked against all of them — the neutral ramp lives in
    /// `ramp.css`, not in the theme's own file.
    #[test]
    fn tokens_from_every_emitted_file_count() {
        let ramp = ":root { --nc-gray-2: oklch(0.12 0 0); }";
        let both: BTreeSet<String> = [STYLESHEET, ramp]
            .into_iter()
            .flat_map(defined_tokens)
            .collect();
        assert!(both.contains("--nc-gray-2"));
        assert!(both.contains("--color-accent"));
    }

    /// A gate that cannot catch the failure it was written for is worse than
    /// no gate, because it reads as coverage.
    #[test]
    fn a_mistyped_token_is_caught_with_its_file_and_line() {
        let root = std::env::temp_dir().join("noctua-references-gate");
        let consumer = root.join("examples/broken");
        std::fs::create_dir_all(&consumer).expect("temp tree");
        std::fs::write(
            consumer.join("page.css"),
            "a { color: var(--color-accent); }\nb { color: var(--color-accnt); }\n",
        )
        .expect("write");

        let report = check(&root, [STYLESHEET]);
        std::fs::remove_dir_all(&root).ok();

        let failures = report.failures();
        assert_eq!(failures.len(), 1, "{}", report.summary());
        assert!(failures[0].message.contains("--color-accnt"));
        assert_eq!(failures[0].where_, "examples/broken/page.css:2");
    }

    /// The whole repository, against what it actually shipped.
    #[test]
    fn every_consumer_reference_resolves() {
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let sheets = read_stylesheets(&root.join("dist/css"));
        assert!(
            !sheets.is_empty(),
            "dist/ must exist — run `cargo xtask build`"
        );

        let report = check(root, sheets.iter().map(String::as_str));
        assert!(report.checked > 0, "the gate scanned nothing");
        assert!(report.is_ok(), "\n{}", report.summary());
    }
}
