//! Diagnostics.
//!
//! Two rules shape this module.
//!
//! **Every problem is reported, not just the first.** Fixing one error only to
//! be shown the next is a miserable way to edit a file. Validation collects
//! everything it finds and reports it in one pass.
//!
//! **Every problem ends with what to do about it.** A diagnostic that says
//! what is wrong and stops has done half the job.

use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;

/// Everything wrong with a specification.
#[derive(Debug, Error, Diagnostic)]
#[error("{} problem{} in the specification", problems.len(), if problems.len() == 1 { "" } else { "s" })]
#[diagnostic(code(noctua::spec::invalid))]
pub struct SpecError {
    #[source_code]
    src: NamedSource<String>,

    #[related]
    problems: Vec<Problem>,
}

impl SpecError {
    /// Builds an error over the given source text.
    #[must_use]
    pub fn new(path: &str, text: String, problems: Vec<Problem>) -> Self {
        Self {
            src: NamedSource::new(path, text).with_language("TOML"),
            problems,
        }
    }

    /// The specification text the problems refer to.
    ///
    /// For callers that render their own diagnostics and need to turn a byte
    /// offset into a line and column.
    #[must_use]
    pub fn source_text(&self) -> &str {
        self.src.inner()
    }

    /// The individual problems, in the order they were found.
    #[must_use]
    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }
}

/// One thing wrong, with a place and a fix.
#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
pub struct Problem {
    message: String,

    #[label("{label}")]
    span: Option<SourceSpan>,

    label: String,

    #[help]
    help: String,
}

impl Problem {
    /// Reports a problem at a span in the spec.
    ///
    /// A zero-width span means the value came from a default rather than from
    /// the file, in which case no label is attached — pointing at byte zero
    /// would be a lie.
    #[must_use]
    pub fn at(span: std::ops::Range<usize>, message: impl Into<String>) -> Self {
        let span = if span.is_empty() {
            None
        } else {
            Some(SourceSpan::from((span.start, span.len())))
        };
        Self {
            message: message.into(),
            span,
            label: "here".to_owned(),
            help: String::new(),
        }
    }

    /// Reports a problem with no position, for defaults and whole-file rules.
    #[must_use]
    pub fn whole_file(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            label: String::new(),
            help: String::new(),
        }
    }

    /// Sets the text shown against the underlined span.
    #[must_use]
    pub fn labelled(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the fix. Every problem should have one.
    #[must_use]
    pub fn fix(mut self, help: impl Into<String>) -> Self {
        self.help = help.into();
        self
    }

    /// What is wrong.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// What to do about it. Empty when the problem carries no fix.
    #[must_use]
    pub fn help(&self) -> &str {
        &self.help
    }

    /// Where in the spec, as a byte offset and length.
    ///
    /// `None` where the value came from a default rather than the file.
    ///
    /// Exposed for callers that cannot use `miette`'s renderer — the browser
    /// playground formats its own diagnostics, and without this it could only
    /// report that *a* problem existed.
    #[must_use]
    pub fn span(&self) -> Option<(usize, usize)> {
        self.span.map(|s| (s.offset(), s.len()))
    }
}

/// Suggests the closest of `candidates` to `input`, if any is close enough.
///
/// A misspelled family name is the most likely error in a spec, and "did you
/// mean" turns a hunt through the file into a one-word fix.
#[must_use]
pub fn did_you_mean<'a>(input: &str, candidates: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut best: Option<(usize, &str)> = None;
    for candidate in candidates {
        let distance = edit_distance(input, candidate);
        if best.is_none_or(|(d, _)| distance < d) {
            best = Some((distance, candidate));
        }
    }

    // Accept a suggestion only when it is closer than roughly a third of the
    // word; beyond that the "suggestion" is noise.
    best.filter(|(distance, _)| *distance * 3 <= input.len().max(3))
        .map(|(_, candidate)| candidate.to_owned())
}

/// Levenshtein distance, iterative with a single row.
fn edit_distance(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b_chars.len()).collect();

    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b_chars.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            let next_diagonal = row[j + 1];
            row[j + 1] = (row[j + 1] + 1).min(row[j] + 1).min(diagonal + cost);
            diagonal = next_diagonal;
        }
    }
    row[b_chars.len()]
}

#[cfg(test)]
// Comparisons here are against literal values the code returns verbatim.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_counts_single_edits() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("accent", "accent"), 0);
        assert_eq!(edit_distance("accent", "accnt"), 1);
        assert_eq!(edit_distance("accent", "acccent"), 1);
        assert_eq!(edit_distance("accent", "arcent"), 1);
        assert_eq!(edit_distance("abc", "xyz"), 3);
    }

    #[test]
    fn a_near_miss_is_suggested() {
        let candidates = ["accent", "danger", "success"];
        assert_eq!(
            did_you_mean("acent", candidates.into_iter()),
            Some("accent".to_owned())
        );
        assert_eq!(
            did_you_mean("dangr", candidates.into_iter()),
            Some("danger".to_owned())
        );
    }

    #[test]
    fn a_wild_guess_is_not_suggested() {
        // Offering "accent" for "zzzzzzzz" is worse than offering nothing.
        let candidates = ["accent", "danger"];
        assert_eq!(did_you_mean("zzzzzzzz", candidates.into_iter()), None);
    }

    #[test]
    fn suggestions_need_candidates() {
        assert_eq!(did_you_mean("accent", std::iter::empty()), None);
    }

    #[test]
    fn a_default_valued_problem_carries_no_label() {
        // Defaults have no position in the file; pointing at byte zero would
        // underline something the author never wrote.
        let problem = Problem::at(0..0, "something is wrong");
        assert!(problem.span.is_none());

        let problem = Problem::at(10..20, "something is wrong");
        assert!(problem.span.is_some());
    }
}
