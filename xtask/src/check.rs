//! `cargo xtask check` — the single gate.
//!
//! Everything CI runs, in the order that fails fastest on the most likely
//! mistake. A spec that does not parse should say so before waiting on a
//! clippy pass.

use std::path::Path;
use std::process::Command;

use crate::{build, ui};

/// Runs every gate.
///
/// # Errors
///
/// The first stage that fails, with what to do about it.
pub(crate) fn run(root: &Path, spec_path: &Path, colors_only: bool) -> Result<(), String> {
    let mut failures: Vec<String> = Vec::new();

    ui::heading("specification");
    // Deliberately `palette`, not `run`: `check` must never write dist/ before
    // comparing against it, or the hand-edit guard checks nothing.
    let palette = build::palette(root, spec_path)?;

    ui::gap();
    ui::heading("quality gates");
    failures.extend(gates(root, &palette));

    ui::gap();
    ui::heading("generated output");
    failures.extend(generated_output(root, spec_path, &palette)?);

    if !colors_only {
        ui::gap();
        ui::heading("workspace");
        for (label, args) in [
            ("formatting", vec!["fmt", "--all", "--check"]),
            (
                "lints",
                vec![
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ],
            ),
            ("tests", vec!["test", "--workspace", "--quiet"]),
        ] {
            match cargo(root, &args) {
                Ok(()) => ui::ok(label),
                Err(error) => {
                    ui::failure(&format!("{label}:\n{error}"));
                    failures.push(label.to_owned());
                }
            }
        }
    }

    ui::gap();
    if failures.is_empty() {
        ui::heading("check passed");
        Ok(())
    } else {
        Err(format!("check failed: {}", failures.join(", ")))
    }
}

/// Runs cargo in `directory`, returning its output on failure.
fn cargo(directory: &Path, args: &[&str]) -> Result<(), String> {
    run_cargo(directory, args, None)
}

/// The same, with an explicit target directory.
fn cargo_with_target(directory: &Path, args: &[&str], target: &Path) -> Result<(), String> {
    run_cargo(directory, args, Some(target))
}

/// Every quality gate, reported together.
///
/// Collected rather than short-circuited: a gate that stops at the first
/// problem turns one bad build into five.
fn gates(root: &Path, palette: &noctua_engine::Palette) -> Vec<String> {
    let palette_report = noctua_check::run(palette);
    let source_report = noctua_check::source::check(root);
    let palette_checks = palette_report.checked;
    let source_lines = source_report.checked;

    // Read rather than regenerated: the point is to check what shipped.
    let sheets = noctua_check::references::read_stylesheets(&root.join("dist/css"));
    let reference_report = noctua_check::references::check(root, sheets.iter().map(String::as_str));
    let reference_lines = reference_report.checked;

    let mut report = palette_report;
    report.absorb(source_report);
    report.absorb(reference_report);

    for finding in report.warnings() {
        ui::warn(&finding.to_string());
    }
    for finding in report.notes() {
        ui::note(&finding.to_string());
    }

    if report.failures().is_empty() {
        ui::ok(&format!(
            "{palette_checks} palette checks, {source_lines} lines of source, \
             {reference_lines} lines of consumers, {} warning(s), {} note(s)",
            report.warnings().len(),
            report.notes().len()
        ));
        return Vec::new();
    }

    for finding in report.failures() {
        ui::failure(&finding.to_string());
    }
    vec![format!(
        "{} quality gate failure(s)",
        report.failures().len()
    )]
}

/// That `dist/` is in sync, and that what it contains actually works.
///
/// # Errors
///
/// Only when the spec or `dist/` cannot be read. A file being out of sync is
/// a returned failure, not an error, so the remaining checks still run.
fn generated_output(
    root: &Path,
    spec_path: &Path,
    palette: &noctua_engine::Palette,
) -> Result<Vec<String>, String> {
    let mut failures = Vec::new();

    let relative = build::relative_to(root, spec_path);
    let spec_text = std::fs::read_to_string(spec_path)
        .map_err(|error| format!("could not read {relative}: {error}"))?;
    let drift = noctua_emit::dist::check(&root.join("dist"), palette, &relative, &spec_text)
        .map_err(|error| format!("could not read dist/: {error}"))?;

    if drift.is_empty() {
        ui::ok("dist/ matches the specification");
    } else {
        for item in &drift {
            ui::failure(&item.to_string());
        }
        ui::detail("generated files are never edited by hand; change the spec and rebuild");
        failures.push(format!("{} generated file(s) out of sync", drift.len()));
    }

    // The generated Rust crate is a deliverable, and a deliverable that does
    // not compile is not one. Cheap to check and it has caught a real manifest
    // mistake before.
    //
    // Built into the workspace's own target directory rather than inside
    // dist/. Otherwise compiling leaves artifacts among the generated files,
    // and the determinism check — which diffs two builds of dist/ — starts
    // comparing incremental-compilation fingerprints.
    match cargo_with_target(
        &root.join("dist/rust"),
        &["build", "--quiet"],
        &root.join("target/generated-crate"),
    ) {
        Ok(()) => ui::ok("the generated Rust crate compiles"),
        Err(error) => {
            ui::failure(&error);
            failures.push("the generated Rust crate does not compile".to_owned());
        }
    }

    // The example consumer, run rather than merely compiled: it asserts the
    // neutral ramp is monotone, so running it tests the output and not only
    // the manifest. It sits outside the workspace on purpose — a sibling
    // project is not a member of this one — which is exactly why the
    // workspace `cargo test` would never reach it.
    let example = root.join("examples/consumer-rust");
    if example.is_dir() {
        match cargo_with_target(&example, &["run", "--quiet"], &root.join("target/examples")) {
            Ok(()) => ui::ok("the example consumer builds and runs against dist/rust"),
            Err(error) => {
                ui::failure(&error);
                failures.push("the example consumer does not build against dist/rust".to_owned());
            }
        }
    }

    // The npm package's claims about dist/, checked in Rust so this fires on a
    // machine with only rustup. `npm pack --dry-run` in CI is the authority;
    // this is what catches the mistake before it is pushed.
    failures.extend(crate::packaging::check(root, env!("CARGO_PKG_VERSION"))?);

    Ok(failures)
}

fn run_cargo(directory: &Path, args: &[&str], target: Option<&Path>) -> Result<(), String> {
    let mut command = Command::new(env!("CARGO"));
    command.args(args).current_dir(directory);
    if let Some(target) = target {
        command.env("CARGO_TARGET_DIR", target);
    }

    let output = command
        .output()
        .map_err(|error| format!("could not run cargo: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let mut message = String::from_utf8_lossy(&output.stderr).into_owned();
    if message.trim().is_empty() {
        message = String::from_utf8_lossy(&output.stdout).into_owned();
    }
    // Enough to identify the problem without burying the summary.
    Err(message.lines().take(30).collect::<Vec<_>>().join("\n"))
}

#[cfg(test)]
mod tests {
    fn shipped_palette() -> noctua_engine::Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../specs/noctua.toml");
        noctua_engine::build(&noctua_spec::load(path).expect("valid")).expect("builds")
    }

    /// The two independent readings of the semantic contract — the emitter's,
    /// which decides what ships, and the gate's, which decides what is checked
    /// — must resolve every token to the same step.
    ///
    /// They are written separately on purpose: a gate that imported the
    /// emitter's view would agree with it by construction, and a bug in that
    /// view would pass its own check. Independence nobody compares is just
    /// drift, so this is where they are compared.
    #[test]
    fn the_gate_and_the_emitter_resolve_every_token_alike() {
        let palette = shipped_palette();
        for theme in &palette.themes {
            for mode in &theme.modes {
                let emitted: Vec<(String, String)> = noctua_emit::tokens::semantic_tokens(mode)
                    .into_iter()
                    .map(|alias| (alias.name, alias.target))
                    .collect();
                let gated: Vec<(String, String)> = noctua_check::contrast::semantic_view(mode)
                    .into_iter()
                    .map(|(name, (family, role))| (name, format!("{family}-{role}")))
                    .collect();

                assert_eq!(
                    emitted,
                    gated,
                    "{}/{}: the emitter and the gate disagree about the contract",
                    theme.name,
                    mode.mode.id()
                );
            }
        }
    }

    /// The docs-site literal check and the source gate scan the same files.
    /// They are separate because `noctua-docs` does not depend on the gates,
    /// which means their rules can diverge — and a rule that is stricter in
    /// one than the other fails a file that the other says is fine.
    #[test]
    fn both_literal_checks_use_the_same_marker() {
        assert_eq!(noctua_check::source::ALLOW_MARKER, "allow-literal:");
    }

    /// The documentation site renders the contrast matrix from a table of its
    /// own, because `noctua-docs` reads `dist/` and never depends on the
    /// gates. That independence is deliberate, and it means the two tables
    /// can drift — so they are compared here, in the one crate that can see
    /// both.
    ///
    /// The drift this catches is not cosmetic. The site once showed a soft
    /// pair with the same cross as a hard failure, which told a reader the
    /// palette was broken while `cargo xtask check` said it shipped.
    #[test]
    fn the_site_and_the_gate_agree_on_every_pair() {
        use noctua_check::Severity;

        let palette = shipped_palette();
        let gated_pairs = noctua_check::contrast::pairs(&palette.themes[0].modes[0]);

        for (fg, bg, minimum, severity) in noctua_docs::sections::contrast_pairs() {
            let gated = gated_pairs
                .iter()
                .find(|p| p.foreground == fg && p.background == bg)
                .unwrap_or_else(|| {
                    panic!("the site shows `{fg}` on `{bg}`, which the compiler does not gate")
                });

            assert!(
                (gated.minimum - minimum).abs() < f64::EPSILON,
                "`{fg}` on `{bg}`: the site says {minimum} Lc, the gate says {}",
                gated.minimum
            );

            let expected = match gated.severity {
                Severity::Fail => "fail",
                Severity::Warn => "warn",
                Severity::Note => "note",
            };
            assert_eq!(
                severity, expected,
                "`{fg}` on `{bg}`: the site calls it `{severity}`, the gate calls it `{expected}`"
            );
        }
    }
}
