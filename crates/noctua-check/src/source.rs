//! Invariants about the source tree rather than the palette.
//!
//! Two rules that no amount of color math can enforce, and that were until now
//! enforced by somebody remembering to look.
//!
//! **No hardcoded colors.** The whole thesis of this project is that colors
//! are computed, not listed. An invariant checked by hand is an invariant that
//! holds until the first busy afternoon.
//!
//! **Crate boundaries.** `noctua-core` knows nothing about specs or targets;
//! `noctua-emit` performs no color math. Both are easy to violate by adding
//! one convenient import, and neither violation announces itself.
//!
//! # The escape hatch
//!
//! A handful of literals are legitimate — a hex parser needs hex to parse, and
//! APCA's published anchor values are reference data. Rather than encode a
//! clever exemption rule, the gate requires an explicit marker on the line:
//!
//! ```text
//! let anchor = hex("#767676"); // allow-literal: published reference pair
//! ```
//!
//! That makes every exception visible in review and forces a reason to be
//! written down. Black and white are exempt without a marker, being the only
//! two colors with no free parameters to get wrong.

use std::path::{Path, PathBuf};

use crate::{Finding, Report, Severity};

const GATE: &str = "source";

/// The marker that permits a color literal, with a reason after the colon.
pub const ALLOW_MARKER: &str = "allow-literal:";

/// Directories scanned for color literals.
const SCANNED: &[&str] = &["crates", "docs-site", "examples", "xtask"];

/// Colors with no free parameters, permitted without a marker.
fn is_parameterless(literal: &str) -> bool {
    matches!(
        literal.to_ascii_lowercase().as_str(),
        "#000" | "#fff" | "#000000" | "#ffffff"
    )
}

/// Finds `#rgb`, `#rrggbb` and `#rrggbbaa` literals in a line.
fn literals(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '#' {
            let digits: String = chars[i + 1..]
                .iter()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            let after = i + 1 + digits.len();
            // Only exact lengths are colors. `#12345` is a malformed-input
            // fixture, not a color, and flagging it would train people to
            // ignore this gate.
            let bounded = chars.get(after).is_none_or(|c| !c.is_ascii_alphanumeric());
            if bounded && matches!(digits.len(), 3 | 6 | 8) {
                found.push(format!("#{digits}"));
            }
            i = after.max(i + 1);
        } else {
            i += 1;
        }
    }
    found
}

/// Scans the repository for color literals that nothing permits.
///
/// `system/`, `tests/golden/` and `target/` are excluded: they are generated,
/// and the colors in them are the entire point.
#[must_use]
pub fn no_hardcoded_colors(root: &Path) -> Report {
    let mut report = Report::default();

    for directory in SCANNED {
        let base = root.join(directory);
        if !base.exists() {
            continue;
        }
        for file in walk(&base) {
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let relative = file
                .strip_prefix(root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");

            for (number, line) in text.lines().enumerate() {
                report.checked += 1;
                if line.contains(ALLOW_MARKER) {
                    continue;
                }
                for literal in literals(line) {
                    if is_parameterless(&literal) {
                        continue;
                    }
                    report.findings.push(Finding {
                        gate: GATE,
                        severity: Severity::Fail,
                        where_: format!("{relative}:{}", number + 1),
                        message: format!(
                            "hardcoded color `{literal}`. Compute it, or mark the line \
                             `{ALLOW_MARKER} <reason>` if it is genuinely reference data"
                        ),
                        margin: None,
                    });
                }
            }
        }
    }

    report
}

/// Color-math entry points that must not appear in an emitter.
const MATH_CALLS: &[&str] = &[
    "max_chroma(",
    "map_into_gamut(",
    "apca(",
    "wcag21(",
    "delta_e_ok(",
    "simulate(",
];

/// Checks that each crate stays inside its own remit.
#[must_use]
pub fn crate_boundaries(root: &Path) -> Report {
    let mut report = Report::default();

    // `noctua-core` must have no workspace dependencies at all.
    let manifest = root.join("crates/noctua-core/Cargo.toml");
    if let Ok(text) = std::fs::read_to_string(&manifest) {
        report.checked += 1;
        let after = text.split("[dependencies]").nth(1).unwrap_or("");
        let section = after.split("\n[").next().unwrap_or("");
        if section.contains("noctua-") {
            report.findings.push(Finding {
                gate: GATE,
                severity: Severity::Fail,
                where_: "crates/noctua-core/Cargo.toml".to_owned(),
                message: "noctua-core must have no workspace dependencies; it knows \
                          nothing about specs or output targets"
                    .to_owned(),
                margin: None,
            });
        }
    }

    // `noctua-emit` must format colors, not compute them.
    let source = root.join("crates/noctua-emit/src");
    if source.exists() {
        for file in walk(&source) {
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let relative = file
                .strip_prefix(root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");

            // Tests may call anything: they build fixtures. Every file in this
            // crate puts its tests last, behind a single `#[cfg(test)]`.
            let production = text.split("#[cfg(test)]").next().unwrap_or(&text);

            for (number, line) in production.lines().enumerate() {
                report.checked += 1;
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with('*') {
                    continue;
                }
                for call in MATH_CALLS {
                    if line.contains(call) {
                        report.findings.push(Finding {
                            gate: GATE,
                            severity: Severity::Fail,
                            where_: format!("{relative}:{}", number + 1),
                            message: format!(
                                "`{call}` is color math, which belongs in the engine. An \
                                 emitter that computes a color makes the gates check \
                                 something other than what ships"
                            ),
                            margin: None,
                        });
                    }
                }
            }
        }
    }

    report
}

/// Every source file under `base`, ignoring generated and vendored trees.
fn walk(base: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![base.to_path_buf()];

    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if path.is_dir() {
                // `public` is the rendered docs site: generated output, like
                // `system`. The colors in it are the palette being displayed,
                // which is the entire point of the page.
                if !matches!(
                    name.as_str(),
                    "target" | "system" | "public" | "node_modules" | "vendor"
                ) {
                    stack.push(path);
                }
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("rs" | "css" | "scss" | "js" | "ts" | "html" | "qml")
            ) {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Runs every source-level gate.
#[must_use]
pub fn check(root: &Path) -> Report {
    let mut report = no_hardcoded_colors(root);
    report.absorb(crate_boundaries(root));
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_finds_colour_literals() {
        assert_eq!(literals("let c = \"#b07a4e\";"), vec!["#b07a4e"]); // allow-literal: fixture for this gate's own detector
        assert_eq!(literals("background: #abc;"), vec!["#abc"]); // allow-literal: fixture for this gate's own detector
        assert_eq!(literals("\"#b07a4e80\""), vec!["#b07a4e80"]); // allow-literal: fixture for this gate's own detector
        assert_eq!(literals("a #fff and a #000000"), vec!["#fff", "#000000"]);
    }

    /// Flagging malformed-input fixtures would train people to ignore this.
    #[test]
    fn it_ignores_things_that_are_not_colours() {
        assert!(literals("issue #12345 and #1234567").is_empty());
        assert!(literals("no colours here").is_empty());
        assert!(literals("#").is_empty());
        assert!(literals("#zz").is_empty());
    }

    #[test]
    fn black_and_white_need_no_excuse() {
        assert!(is_parameterless("#000000"));
        assert!(is_parameterless("#FFFFFF"));
        assert!(is_parameterless("#fff"));
        assert!(!is_parameterless("#b07a4e")); // allow-literal: fixture: an unmarked and a marked literal
    }

    #[test]
    fn the_marker_permits_a_line_and_the_failure_explains_it() {
        let scratch = std::env::temp_dir().join("noctua-source-marker");
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("crates/x/src")).expect("mkdir");
        std::fs::write(
            scratch.join("crates/x/src/lib.rs"),
            "let a = \"#b07a4e\";\nlet b = \"#7bb07a\"; // allow-literal: reference data\n",
        )
        .expect("write");

        let report = no_hardcoded_colors(&scratch);
        assert_eq!(
            report.findings.len(),
            1,
            "only the unmarked line should fail"
        );
        assert!(report.findings[0].where_.ends_with(":1"));
        assert!(report.findings[0].message.contains("#b07a4e")); // allow-literal: fixture for the boundary detector
        assert!(
            report.findings[0].message.contains(ALLOW_MARKER),
            "the failure must say how to permit a genuine exception"
        );

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn a_core_crate_that_grew_a_workspace_dependency_is_caught() {
        let scratch = std::env::temp_dir().join("noctua-source-boundary");
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("crates/noctua-core")).expect("mkdir");
        std::fs::write(
            scratch.join("crates/noctua-core/Cargo.toml"),
            "[package]\nname = \"noctua-core\"\n\n[dependencies]\nnoctua-spec.workspace = true\n",
        )
        .expect("write");

        let report = crate_boundaries(&scratch);
        assert_eq!(report.findings.len(), 1);
        assert!(
            report.findings[0]
                .message
                .contains("no workspace dependencies")
        );

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn an_emitter_doing_colour_math_is_caught() {
        let scratch = std::env::temp_dir().join("noctua-source-math");
        let _ = std::fs::remove_dir_all(&scratch);
        std::fs::create_dir_all(scratch.join("crates/noctua-emit/src")).expect("mkdir");
        std::fs::write(
            scratch.join("crates/noctua-emit/src/bad.rs"),
            "fn f() { let c = gamut.max_chroma(0.5, 264.0); }\n#[cfg(test)]\nmod t { fn g() { apca(a, b); } }\n",
        )
        .expect("write");

        let report = crate_boundaries(&scratch);
        assert_eq!(
            report.findings.len(),
            1,
            "tests may build fixtures; production may not"
        );
        assert!(report.findings[0].message.contains("max_chroma("));

        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn the_repository_itself_passes_both_gates() {
        let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
        let report = check(root);
        assert!(report.checked > 1000, "the gate scanned almost nothing");
        assert!(report.is_ok(), "\n{}", report.summary());
    }
}
