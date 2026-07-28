//! The `noctua-colors` specification format.
//!
//! One TOML file describes a whole color system. This crate turns it into a
//! validated [`Spec`], or into a diagnostic that says exactly where the file
//! is wrong and what to do about it.
//!
//! # Design
//!
//! **Every field has a default**, so the smallest useful spec is three lines
//! and nobody edits configuration to get a normal result:
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let spec = noctua_spec::parse("example.toml", r"
//! [families.accent]
//! hue = 264
//! ")?;
//!
//! assert_eq!(spec.scale.roles.len(), 12);
//! # Ok(()) }
//! ```
//!
//! **Unknown keys are errors.** A misspelled key that silently does nothing
//! is far worse than a build that stops.
//!
//! **Every problem is reported at once**, each with a span and a fix.
//!
//! This crate performs no color math and knows nothing about output targets.

pub mod curve;
pub mod defaults;
pub mod error;
mod expand;
pub mod model;
mod validate;

pub use curve::{CurveSpec, HueSpec};
pub use error::{Problem, SpecError};
pub use model::{
    Alpha, ApcaTarget, Chart, Consumer, DeltaLTarget, DensityBand, Family, FamilyOverride,
    NamedScale, Neutral, Output, Role, Scale, Spec, Spread, Stops, System, TargetSpec, Theme,
};

/// A short, stable content hash of a specification.
///
/// Lives here rather than in an emitter because the hash identifies *the file* —
/// so every consumer that has a [`Spec`] has its provenance, and the several
/// places that publish it cannot publish different values.
///
/// Thirty-two hex characters of blake3: long enough that a collision is not a
/// thing that happens, short enough to read in a diff.
#[must_use]
pub fn content_hash(bytes: &[u8]) -> String {
    let full = blake3::hash(bytes).to_hex();
    format!("blake3:{}", &full.as_str()[..32])
}

/// Parses and validates a specification.
///
/// `path` is used only for diagnostics; nothing is read from disk.
///
/// # Errors
///
/// Returns every syntax and semantic problem found, in one diagnostic.
pub fn parse(path: &str, text: &str) -> Result<Spec, SpecError> {
    let mut spec: Spec = match toml::from_str(text) {
        Ok(spec) => spec,
        Err(error) => {
            let (message, fix) = explain(error.message());
            let problem = match error.span() {
                Some(span) => Problem::at(span, message).labelled("invalid here").fix(fix),
                None => Problem::whole_file(message),
            };
            return Err(SpecError::new(path, text.to_owned(), vec![problem]));
        }
    };

    // The accent grid becomes ordinary themes before anything else looks at
    // the spec, so validation and every consumer see one uniform list.
    expand::accent_grid(&mut spec);

    spec.source_hash = content_hash(text.as_bytes());

    let problems = validate::check(&spec);
    if problems.is_empty() {
        Ok(spec)
    } else {
        Err(SpecError::new(path, text.to_owned(), problems))
    }
}

/// Turns a serde message into one a person can act on.
///
/// An untagged enum that matches nothing reports "data did not match any
/// variant of untagged enum `HueSpec`" — the name of a Rust type the reader
/// has never seen, and no indication of what would have matched. Every field in
/// this format that accepts more than one shape is such an enum, so this is
/// the error a mistyped spec is most likely to produce.
///
/// Anything not recognised is passed through unchanged rather than replaced
/// with something vaguer.
fn explain(message: &str) -> (String, &'static str) {
    const HUE_FORMS: &str = "a hue is a number, `{ base = <degrees>, torsion = <degrees> }`, \
                             or `{ knots = [[<t>, <degrees>], ...] }`";
    const CURVE_FORMS: &str = "a curve is a number, \
                               `{ ends = [<start>, <end>], peak = <value>, peak_at = <t> }`, \
                               or `{ knots = [[<t>, <value>], ...] }`";

    if message.contains("untagged enum HueSpec") {
        return (
            "this is not a hue. Every part must be a number, and the field \
             names must match one of the accepted forms"
                .to_owned(),
            HUE_FORMS,
        );
    }

    if message.contains("untagged enum CurveSpec") {
        return (
            "this is not a curve. Every part must be a number, and the field \
             names must match one of the accepted forms"
                .to_owned(),
            CURVE_FORMS,
        );
    }

    (
        message.to_owned(),
        "check the field name and the value's type against the reference in README.md",
    )
}

/// Reads and parses a specification from disk.
///
/// # Errors
///
/// Returns a diagnostic if the file cannot be read, or if it is invalid.
pub fn load(path: impl AsRef<std::path::Path>) -> Result<Spec, SpecError> {
    let path = path.as_ref();
    let display = path.display().to_string();

    let text = std::fs::read_to_string(path).map_err(|error| {
        SpecError::new(
            &display,
            String::new(),
            vec![
                Problem::whole_file(format!("could not read {display}: {error}"))
                    .fix("check the path, or run `cargo xtask build` from the repository root"),
            ],
        )
    })?;

    parse(&display, &text)
}

#[cfg(test)]
mod tests {
    /// A mistyped hue or curve is the most likely error in a spec, and serde
    /// reports it by naming a Rust type the reader has never seen. The
    /// message has to say what would have been accepted instead.
    #[test]
    fn a_mistyped_hue_is_explained_in_terms_of_the_format() {
        let error = super::parse(
            "t.toml",
            "[families.a]\nhue = { base = 1, torsion = \"x\" }\n",
        )
        .expect_err("must fail");
        let problem = &error.problems()[0];

        assert!(
            !problem.message().contains("HueSpec"),
            "the message names a Rust type: {}",
            problem.message()
        );
        assert!(problem.message().contains("hue"), "{}", problem.message());
        assert!(problem.help().contains("torsion"), "{}", problem.help());
        assert!(
            problem.span().is_some(),
            "a parse error must point somewhere"
        );
    }

    /// The colour system's version is published to two registries that both
    /// reject a malformed one — but only after a tag has been pushed. Catching
    /// it here turns a failed release into a failed `check`.
    #[test]
    fn a_malformed_system_version_is_rejected() {
        for bad in ["1.2", "1.2.3.4", "v1.2.3", "1.2.x", ""] {
            let spec = format!("[system]\nversion = \"{bad}\"\n[families.a]\nhue = 1\n");
            let error = super::parse("t.toml", &spec)
                .err()
                .unwrap_or_else(|| panic!("\"{bad}\" should not be accepted"));
            assert!(
                error
                    .problems()
                    .iter()
                    .any(|p| p.message().contains("[system]")),
                "\"{bad}\" was rejected, but not for the reason we meant"
            );
        }
    }

    /// Pre-release and build metadata are legitimate semver and must survive.
    #[test]
    fn a_prerelease_system_version_is_accepted() {
        for good in ["0.2.0", "1.0.0-rc.1", "1.0.0+build.5", "10.20.30"] {
            let spec = format!("[system]\nversion = \"{good}\"\n[families.a]\nhue = 1\n");
            let parsed = super::parse("t.toml", &spec)
                .unwrap_or_else(|e| panic!("\"{good}\" should be accepted: {e:?}"));
            assert_eq!(parsed.system.version, good);
        }
    }

    /// Left out entirely, the spec still parses — every other field has a
    /// default and this one must not be the exception that breaks the
    /// "smallest useful spec is three lines" promise.
    #[test]
    fn the_system_table_is_optional() {
        let parsed = super::parse("t.toml", "[families.a]\nhue = 1\n").expect("parses");
        assert_eq!(parsed.system.name, "noctua-colors");
        assert!(!parsed.system.version.is_empty());
    }

    #[test]
    fn a_mistyped_curve_is_explained_too() {
        let error = super::parse("t.toml", "[families.a]\nhue = 1\ncr = { ends = \"x\" }\n")
            .expect_err("must fail");
        let problem = &error.problems()[0];
        assert!(
            !problem.message().contains("CurveSpec"),
            "{}",
            problem.message()
        );
        assert!(problem.help().contains("peak"), "{}", problem.help());
    }

    /// An error this does not recognise must survive unchanged rather than be
    /// replaced by something vaguer.
    #[test]
    fn an_unrecognised_error_is_passed_through() {
        let (message, _) = super::explain("expected a table");
        assert_eq!(message, "expected a table");
    }

    use super::*;

    /// Renders every problem plus its fix, which is what the tests assert on:
    /// a diagnostic without an actionable fix has done half the job.
    fn errors_for(text: &str) -> String {
        let error = parse("test.toml", text).expect_err("should not validate");
        error
            .problems()
            .iter()
            // `Problem::help` is the inherent accessor, not miette's trait
            // method: the same string, without the Option dance.
            .map(|p| format!("{p}\n{}", p.help()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn a_minimal_spec_validates() {
        let spec = parse("test.toml", "[families.accent]\nhue = 264").expect("valid");
        assert_eq!(spec.families.len(), 1);
        assert_eq!(spec.scale.roles.len(), 12);
    }

    #[test]
    fn a_spec_with_no_families_says_how_to_add_one() {
        let text = errors_for("[output]\ngamut = \"srgb\"");
        assert!(text.contains("no color families"), "{text}");
        assert!(
            text.contains("[families.accent]"),
            "should show an example: {text}"
        );
    }

    #[test]
    fn a_misspelled_family_reference_is_suggested() {
        let text = errors_for(
            r#"
            [families.accent]
            hue = 264
            [themes.default.semantic]
            accent = "acent"
            "#,
        );
        assert!(text.contains("does not exist"), "{text}");
        assert!(text.contains("did you mean `accent`?"), "{text}");
    }

    /// Before this was validated a mistyped family in `[semantic]` reached the
    /// engine, which reported it without a span or a suggestion.
    #[test]
    fn a_misspelled_family_in_the_global_semantic_map_is_suggested() {
        let text = errors_for(
            r#"
            [families.accent]
            hue = 264
            [families.danger]
            hue = 30.8
            [semantic]
            rejected = "dangr"
            "#,
        );
        assert!(text.contains("does not exist"), "{text}");
        assert!(text.contains("did you mean `danger`?"), "{text}");
    }

    /// A slot called `surface` would emit a second `--nc-color-surface` from
    /// whatever family it named, silently overriding the page's own. Before
    /// this check the slot was simply emitted, with no diagnostic at all.
    #[test]
    fn a_slot_cannot_squat_on_the_pages_own_token_names() {
        for slot in ["surface", "fg-muted", "border", "ring", "on-danger"] {
            let text = errors_for(&format!(
                "[families.accent]\nhue = 264\n[semantic]\n{slot} = \"accent\"\n"
            ));
            assert!(
                text.contains("the page's own tokens use"),
                "`{slot}` was accepted: {text}"
            );
        }
    }

    /// A `neutral*` slot emits the page's surfaces and text rather than a
    /// fill, so pointing one at a coloured family emits `fg-cool` from an
    /// accent.
    #[test]
    fn a_neutral_slot_must_point_at_a_neutral_family() {
        let text =
            errors_for("[families.accent]\nhue = 264\n[semantic]\nneutral-cool = \"accent\"\n");
        assert!(text.contains("names a neutral temperature"), "{text}");
    }

    #[test]
    fn a_second_scale_named_chart_would_replace_the_categorical_one() {
        let text = errors_for(
            "[families.accent]\nhue = 264\n[[scales]]\nname = \"chart\"\n\
             stops = 3\nhue = 200.0\n",
        );
        assert!(text.contains("already taken"), "{text}");
    }

    #[test]
    fn a_scale_with_no_stops_says_how_to_give_it_some() {
        let text = errors_for(
            "[families.accent]\nhue = 264\n[[scales]]\nname = \"level\"\n\
             stops = 0\nhue = 200.0\n",
        );
        assert!(text.contains("no stops"), "{text}");
        assert!(text.contains("stops = 11"), "{text}");
    }

    #[test]
    fn two_scales_cannot_share_a_name() {
        let text = errors_for(
            "[families.accent]\nhue = 264\n\
             [[scales]]\nname = \"level\"\nstops = 3\nhue = 200.0\n\
             [[scales]]\nname = \"level\"\nstops = 4\nhue = 100.0\n",
        );
        assert!(text.contains("two scales are named"), "{text}");
    }

    #[test]
    fn a_role_referencing_a_missing_role_lists_the_real_ones() {
        let text = errors_for(
            r#"
            [families.accent]
            hue = 264
            [[scale.roles]]
            name = "bg-app"
            light = { lightness = 0.99 }
            dark = { lightness = 0.18 }
            [[scale.roles]]
            name = "text"
            light = { apca = { against = "bg-apppp", lc = 90 } }
            dark = { apca = { against = "bg-app", lc = 90 } }
            "#,
        );
        assert!(text.contains("not a role"), "{text}");
        assert!(text.contains("did you mean `bg-app`?"), "{text}");
    }

    #[test]
    fn a_role_may_not_depend_on_one_declared_later() {
        let text = errors_for(
            r#"
            [families.accent]
            hue = 264
            [[scale.roles]]
            name = "text"
            light = { apca = { against = "bg-app", lc = 90 } }
            dark = { apca = { against = "bg-app", lc = 90 } }
            [[scale.roles]]
            name = "bg-app"
            light = { lightness = 0.99 }
            dark = { lightness = 0.18 }
            "#,
        );
        assert!(text.contains("declared after it"), "{text}");
        assert!(text.contains("move `bg-app` above `text`"), "{text}");
    }

    #[test]
    fn a_self_referencing_role_is_rejected() {
        let text = errors_for(
            r#"
            [families.accent]
            hue = 264
            [[scale.roles]]
            name = "loop"
            light = { delta_l = { against = "loop", amount = 0.1 } }
            dark = { delta_l = { against = "loop", amount = 0.1 } }
            "#,
        );
        assert!(text.contains("targets itself"), "{text}");
    }

    #[test]
    fn an_impossible_contrast_target_says_what_is_reachable() {
        let text = errors_for(
            r#"
            [families.accent]
            hue = 264
            [[scale.roles]]
            name = "bg-app"
            light = { lightness = 0.99 }
            dark = { lightness = 0.18 }
            [[scale.roles]]
            name = "text"
            light = { apca = { against = "bg-app", lc = 200 } }
            dark = { apca = { against = "bg-app", lc = 90 } }
            "#,
        );
        assert!(text.contains("unreachable"), "{text}");
        assert!(text.contains("108"), "should name the ceiling: {text}");
    }

    #[test]
    fn a_target_must_choose_exactly_one_kind() {
        let text = errors_for(
            r#"
            [families.accent]
            hue = 264
            [[scale.roles]]
            name = "confused"
            light = { lightness = 0.5, apca = { against = "confused", lc = 60 } }
            dark = { lightness = 0.5 }
            "#,
        );
        assert!(text.contains("2 light targets"), "{text}");
        assert!(text.contains("exactly one"), "{text}");
    }

    #[test]
    fn out_of_range_relative_chroma_is_rejected() {
        let text = errors_for("[families.accent]\nhue = 264\ncr = 1.4");
        assert!(text.contains("relative chroma"), "{text}");
        assert!(text.contains("between 0 and 1"), "{text}");
    }

    #[test]
    fn every_problem_is_reported_in_one_pass() {
        // Three independent mistakes; fixing them one build at a time would
        // be three builds.
        let error = parse(
            "test.toml",
            r"
            [families.accent]
            hue = 264
            cr = 1.4
            [neutral]
            steps = 1
            tint_strength = 9.0
            ",
        )
        .expect_err("invalid");
        assert!(
            error.problems().len() >= 3,
            "got {} problems",
            error.problems().len()
        );
    }

    #[test]
    fn overlapping_density_bands_are_rejected() {
        let text = errors_for(
            r"
            [families.accent]
            hue = 264
            [neutral]
            density = [{ range = [0.1, 0.5], weight = 2.0 }, { range = [0.4, 0.9], weight = 2.0 }]
            ",
        );
        assert!(text.contains("overlaps"), "{text}");
    }

    #[test]
    fn a_syntax_error_points_at_the_line() {
        let error = parse("test.toml", "[families.accent\nhue = 264").expect_err("invalid");
        assert_eq!(error.problems().len(), 1);
    }

    #[test]
    fn the_default_scale_passes_its_own_validation() {
        // The defaults must satisfy every rule the validator enforces, or the
        // out-of-the-box experience is a failing build.
        parse("test.toml", "[families.accent]\nhue = 264").expect("defaults must be valid");
    }
}
