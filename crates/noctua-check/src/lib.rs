//! Quality gates.
//!
//! The palette is tested code. These are the tests.
//!
//! # Two principles
//!
//! **Report the margin, never just a verdict.** Knowing that success and
//! danger sit 0.31 apart under deuteranopia tells you how much room you have;
//! "pass" tells you nothing, and the day it becomes "fail" you have no idea
//! how far you drifted.
//!
//! **Collect everything, then fail.** A gate that stops at the first problem
//! turns one bad build into five, because each run reveals only the next
//! thing wrong.
//!
//! # Hard and soft
//!
//! Not every pair carries the same weight. Body text on a surface is an
//! accessibility floor and failing it is a defect. An accent used *as* text is
//! a judgement call that a component can mitigate — a bolder weight, a larger
//! size — so it warns rather than fails. Flattening the two would either
//! block builds on taste or let real failures through, and both are worse.

pub mod contrast;
pub mod cvd;
pub mod references;
pub mod source;
pub mod spacing;
pub mod wcag;

use std::fmt;

use noctua_engine::Palette;

/// How seriously to take a finding.
///
/// Three levels, and the distinction between the lower two is the whole point.
/// A **warning** says *you could fix this* — someone chose a number, and a
/// different choice would clear it. A **note** says *this is the measured limit*:
/// it is published because the number matters, not because anyone can act on it.
///
/// Without that split, the colour-vision findings — twenty-nine of them, none
/// fixable by any palette, because ten semantic colours cannot be held apart
/// under dichromacy — sat permanently in the warning list. A build that is
/// always yellow teaches people to skip the yellow lines, which is exactly
/// where a real regression would appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Measured, published, and already at the limit of what is achievable.
    /// Nothing to do; the number is the point.
    Note,
    /// A judgement call, or a case a component can mitigate.
    Warn,
    /// A defect. The build stops.
    Fail,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Note => write!(f, "note"),
            Self::Warn => write!(f, "warn"),
            Self::Fail => write!(f, "FAIL"),
        }
    }
}

/// One thing a gate noticed.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Which gate produced this.
    pub gate: &'static str,
    /// Whether it stops the build.
    pub severity: Severity,
    /// Where the problem is, in the palette or the source tree.
    pub where_: String,
    /// What is wrong.
    pub message: String,
    /// How much room is left, in the gate's own units. `None` where a margin
    /// is not meaningful.
    pub margin: Option<f64>,
}

impl fmt::Display for Finding {
    /// Deliberately without the severity.
    ///
    /// Callers that group findings by severity already show it, and a line
    /// reading `warn  warn [cvd] ...` is the kind of small sloppiness that
    /// makes output look untrustworthy. [`Report::summary`] adds it back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.gate, self.where_, self.message)?;
        if let Some(margin) = self.margin {
            write!(f, " (margin {margin:+.4})")?;
        }
        Ok(())
    }
}

/// Everything every gate found.
#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Findings, in the order the gates ran.
    pub findings: Vec<Finding>,
    /// How many individual checks were performed, so a report of zero
    /// findings can be told apart from a gate that never ran.
    pub checked: usize,
}

impl Report {
    /// Merges another report into this one.
    pub fn absorb(&mut self, other: Self) {
        self.findings.extend(other.findings);
        self.checked += other.checked;
    }

    /// Findings that stop the build.
    #[must_use]
    pub fn failures(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Fail)
            .collect()
    }

    /// Findings worth reading but not worth stopping for.
    #[must_use]
    pub fn warnings(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Warn)
            .collect()
    }

    /// Findings that record a measured limit rather than something to fix.
    #[must_use]
    pub fn notes(&self) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.severity == Severity::Note)
            .collect()
    }

    /// Whether the build may proceed.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.failures().is_empty()
    }

    /// A human-readable summary, failures first.
    #[must_use]
    pub fn summary(&self) -> String {
        use fmt::Write as _;
        let mut out = String::new();

        for finding in self.failures() {
            writeln!(out, "  {} {finding}", finding.severity).expect("string write");
        }
        for finding in self.warnings() {
            writeln!(out, "  {} {finding}", finding.severity).expect("string write");
        }
        for finding in self.notes() {
            writeln!(out, "  {} {finding}", finding.severity).expect("string write");
        }

        write!(
            out,
            "\n{} checks, {} failure(s), {} warning(s), {} note(s)",
            self.checked,
            self.failures().len(),
            self.warnings().len(),
            self.notes().len()
        )
        .expect("string write");
        out
    }
}

/// Runs every gate that inspects a built palette.
///
/// Source-level gates live in [`source`] and take a repository path instead.
#[must_use]
pub fn run(palette: &Palette) -> Report {
    let mut report = Report::default();
    report.absorb(contrast::check(palette));
    report.absorb(cvd::check(palette));
    report.absorb(spacing::check(palette));
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(severity: Severity) -> Finding {
        Finding {
            gate: "test",
            severity,
            where_: "somewhere".to_owned(),
            message: "something".to_owned(),
            margin: Some(-0.5),
        }
    }

    #[test]
    fn only_failures_stop_the_build() {
        let mut report = Report::default();
        report.findings.push(finding(Severity::Warn));
        assert!(report.is_ok(), "a warning must not stop the build");

        report.findings.push(finding(Severity::Fail));
        assert!(!report.is_ok());
        assert_eq!(report.failures().len(), 1);
        assert_eq!(report.warnings().len(), 1);
    }

    #[test]
    fn the_summary_leads_with_failures() {
        let mut report = Report {
            checked: 2,
            findings: Vec::new(),
        };
        report.findings.push(finding(Severity::Warn));
        report.findings.push(finding(Severity::Fail));

        let summary = report.summary();
        assert!(
            summary.find("FAIL").expect("a failure") < summary.find("warn").expect("a warning"),
            "failures must come first:\n{summary}"
        );
        assert!(summary.contains("2 checks"), "{summary}");
    }

    /// A report of zero findings from zero checks is not a pass.
    #[test]
    fn the_summary_says_how_much_was_actually_checked() {
        let report = Report::default();
        assert!(report.is_ok());
        assert!(report.summary().contains("0 checks"));
    }

    #[test]
    fn margins_are_printed_because_a_verdict_alone_is_not_actionable() {
        assert!(
            finding(Severity::Fail)
                .to_string()
                .contains("margin -0.5000")
        );
    }
}
