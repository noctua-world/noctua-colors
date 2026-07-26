//! The gates, run against the shipped specification.
//!
//! This file is the point of the crate. A gate that exists but is never
//! invoked checks nothing — which is exactly what happened to the colour
//! vision simulator between milestones one and three: written, tested against
//! its own invariants, and connected to nothing.

use std::path::Path;

use noctua_check::{Report, Severity};

fn repository_root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn shipped() -> noctua_engine::Palette {
    let path = repository_root().join("specs/noctua.toml");
    noctua_engine::build(&noctua_spec::load(path).expect("valid spec")).expect("builds")
}

#[test]
fn the_shipped_palette_passes_every_gate() {
    let report = noctua_check::run(&shipped());
    assert!(report.checked > 500, "only {} checks ran", report.checked);
    assert!(report.is_ok(), "\n{}", report.summary());
}

#[test]
fn the_repository_passes_the_source_gates() {
    let report = noctua_check::source::check(repository_root());
    assert!(
        report.checked > 1000,
        "only {} lines scanned",
        report.checked
    );
    assert!(report.is_ok(), "\n{}", report.summary());
}

/// Warnings are information rather than noise to be silenced — but a flood of
/// them is noise, and the useful signal is lost inside it.
#[test]
fn warnings_stay_at_a_readable_volume() {
    let report = noctua_check::run(&shipped());
    let warnings = report.warnings().len();
    assert!(warnings < 60, "{warnings} warnings is too many to read");
}

/// Every finding must carry enough to act on.
#[test]
fn every_finding_says_where_and_what_to_do() {
    let mut report: Report = noctua_check::run(&shipped());
    report.absorb(noctua_check::source::check(repository_root()));

    for finding in &report.findings {
        assert!(!finding.where_.is_empty(), "{finding} has no location");
        assert!(
            finding.message.len() > 20,
            "{finding} is too terse to act on"
        );
        if finding.gate == "cvd" {
            assert!(
                finding.message.contains("icon") || finding.message.contains("label"),
                "a colour-vision finding must say what to do instead: {finding}"
            );
        }
    }
}

/// The failure that started all this: a simulator connected to nothing.
#[test]
fn the_colour_vision_gate_is_actually_connected() {
    let report = noctua_check::cvd::check(&shipped());
    assert!(
        report.checked > 100,
        "the CVD gate ran only {} checks",
        report.checked
    );
    assert!(
        !noctua_check::cvd::margins(&shipped()).is_empty(),
        "no margins measured"
    );
}

/// The contrast matrix must reach across families, which is what the engine's
/// own per-family check could never do.
#[test]
fn the_contrast_matrix_crosses_families() {
    let palette = shipped();
    let rows = noctua_check::contrast::matrix(&palette);
    assert!(
        rows.iter()
            .any(|(_, _, fg, bg, _, _)| *fg == "accent" && *bg == "surface"),
        "accent text on the neutral surface is not being checked"
    );
    assert!(rows.len() > 50, "only {} pairs in the matrix", rows.len());
}

/// The three severities must stay meaningfully different, or the split is
/// decoration.
///
/// The shipped palette is expected to report **no failures and no warnings**:
/// everything a different choice could fix has been fixed. What remains are
/// notes — measured limits nothing can act on — and the soft gates proving they
/// still run is what the note count is for.
#[test]
fn the_severity_split_is_real() {
    let report = noctua_check::run(&shipped());
    assert!(report.failures().is_empty(), "\n{}", report.summary());
    assert!(
        report.warnings().is_empty(),
        "a warning is something a different choice would fix, so it should be \
         fixed rather than shipped:\n{}",
        report.summary()
    );
    assert!(
        !report.notes().is_empty(),
        "no notes at all suggests the soft gates are not running"
    );

    for finding in report.notes() {
        assert_eq!(finding.severity, Severity::Note);
        assert!(
            finding.margin.is_some(),
            "a note exists to publish a number: {finding}"
        );
    }
}

/// A note is for a limit, not for a defect that was noisy.
///
/// The colour-vision gate is the only source of notes, and only above its floor
/// — two colours that are literally the same is still a failure, and
/// reclassifying that would be hiding rather than reporting.
#[test]
fn only_unreachable_targets_become_notes() {
    let report = noctua_check::run(&shipped());
    for finding in report.notes() {
        assert_eq!(
            finding.gate, "cvd",
            "an unreachable target outside the colour-vision gate: {finding}"
        );
    }
}
