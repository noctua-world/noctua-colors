//! The contrast matrix.
//!
//! The engine already guarantees that each role hits its target *against its
//! own family's step 1*. That is necessary and nowhere near sufficient,
//! because no interface is built out of one family. Accent text sits on the
//! neutral surface. A danger message sits on a danger-tinted background. A
//! focus ring has to be visible against the page.
//!
//! Those are the pairs that actually get shipped, and every one of them
//! crosses a family boundary — so none of them were checked until this gate
//! existed.
//!
//! # Where the pairs come from
//!
//! Not from configuration. They are derived from the semantic contract, which
//! already says what fills each role, so adding a family or retargeting a
//! theme extends the matrix automatically and nobody edits a list.
//!
//! # What `on-{slot}` can and cannot promise
//!
//! `on-danger` is the danger family's own lightest step, and `danger` is its
//! solid — which the engine solved to sit exactly `45 + contrast_shift` Lc from
//! that step in dark mode. So the contrast between the two is not a property
//! anyone can tune here: it *is* the role target plus the shift, and for the
//! five families carrying a negative shift it lands between Lc 31 and 43.
//!
//! Gating it at 45 would therefore fail the build for a reason no change to
//! this file could fix, and the shift is not negotiable — it is what separates
//! those families under dichromacy. So the row is gated at 30, the floor for a
//! fill's foreground to be *seen*, and the honest statement is this: **a status
//! fill is not a text background.** The pattern that carries text is `fg` on
//! `{slot}-bg`, which is gated at 90 and passes everywhere.

use noctua_core::apca;
use noctua_engine::{Palette, ResolvedMode};

use crate::{Finding, Report, Severity};

const GATE: &str = "contrast";

/// One row of the matrix.
#[derive(Debug)]
pub struct Pair {
    /// Semantic name of the foreground.
    pub foreground: String,
    /// Semantic name of the background.
    pub background: String,
    /// Minimum APCA contrast, as a magnitude.
    pub minimum: f64,
    /// Whether falling short is a defect or a judgement call.
    pub severity: Severity,
    /// Why this threshold, in one clause.
    ///
    /// Static rather than generated, so the set of reasons stays finite and the
    /// documentation site can translate it.
    pub reason: &'static str,
}

/// The variant suffix if this slot is a neutral — `""` for `neutral`, `"-cool"`
/// for `neutral-cool` — otherwise `None`.
fn surface_variant(slot: &str) -> Option<&str> {
    slot.strip_prefix("neutral")
        .filter(|rest| rest.is_empty() || rest.starts_with('-'))
}

/// The pairs an application actually ships, for one mode.
///
/// Thresholds follow APCA's published guidance: 90 for body text, 75 for
/// larger body text, 60 for headlines, 45 for large or bold text, 30 for
/// non-text elements that still have to be seen, 15 as the floor at which
/// anything is discernible at all.
///
/// The soft rows are soft for a reason, not to make the build pass. An accent
/// used as text is a component-level decision — bolder, larger, or not as text
/// at all — and a system that refuses to ship until every accent clears body
/// text contrast would have no accents left.
///
/// # Why this is generated
///
/// It was a hand-written table of eighteen rows, which was readable while the
/// contract had six slots. It now has more than thirty, and a hand-written
/// table would be four hundred lines whose defect mode is a *missing* row —
/// invisible, because nothing reports a pair nobody thought of. Four of
/// twenty-three tokens were in fact ungated that way: every `*-border`.
///
/// # One row per distinct colour
///
/// Contexts are grouped by the family they resolve to, and only the first slot
/// of each family produces rows. `rejected`, `error`, `negative` and `declined`
/// are all the `danger` family, so gating all four would measure the same two
/// colours four times and report the same failure four times under different
/// names. The row is named after the slot that reached the family first, which
/// is spec order.
#[must_use]
pub fn pairs(mode: &ResolvedMode) -> Vec<Pair> {
    let mut out = Vec::new();
    for slot in mode.semantic.keys() {
        if let Some(variant) = surface_variant(slot) {
            surface_rows(&mut out, variant);
        }
    }
    if mode.semantic.contains_key("accent") {
        accent_rows(&mut out);
    }
    context_rows(&mut out, mode);
    out
}

fn row(
    out: &mut Vec<Pair>,
    foreground: String,
    background: String,
    minimum: f64,
    severity: Severity,
    reason: &'static str,
) {
    out.push(Pair {
        foreground,
        background,
        minimum,
        severity,
        reason,
    });
}

/// The page, for one neutral temperature.
fn surface_rows(out: &mut Vec<Pair>, variant: &str) {
    let table: [(String, String, f64, Severity, &'static str); 7] = [
        (
            format!("fg{variant}"),
            format!("surface{variant}"),
            90.0,
            Severity::Fail,
            "body text",
        ),
        (
            format!("fg{variant}"),
            format!("surface-subtle{variant}"),
            90.0,
            Severity::Fail,
            "body text on a marked row",
        ),
        (
            format!("fg{variant}"),
            format!("surface-raised{variant}"),
            90.0,
            Severity::Fail,
            "body text on a card",
        ),
        (
            format!("fg-muted{variant}"),
            format!("surface{variant}"),
            60.0,
            Severity::Fail,
            "secondary text",
        ),
        (
            format!("border-strong{variant}"),
            format!("surface{variant}"),
            15.0,
            Severity::Fail,
            "the strongest border must be visible",
        ),
        (
            format!("border{variant}"),
            format!("surface{variant}"),
            8.0,
            Severity::Warn,
            "an ordinary border is felt more than seen",
        ),
        (
            "ring".to_owned(),
            format!("surface{variant}"),
            30.0,
            Severity::Fail,
            "a focus ring nobody can see is a keyboard trap",
        ),
    ];
    for (foreground, background, minimum, severity, reason) in table {
        row(out, foreground, background, minimum, severity, reason);
    }

    // The base text on a tinted surface. Swapping the surface alone is far more
    // common than swapping every token with it, and the two temperatures were
    // designed to be interchangeable — this is where that claim is checked.
    if !variant.is_empty() {
        row(
            out,
            "fg".to_owned(),
            format!("surface{variant}"),
            90.0,
            Severity::Fail,
            "body text",
        );
    }
}

/// Rows only the accent has: it is the one context that is also a button.
fn accent_rows(out: &mut Vec<Pair>) {
    // Held to APCA's floor for large or bold text. The accent can meet it
    // because it carries no contrast shift; a status family cannot — see the
    // module documentation.
    row(
        out,
        "on-accent".to_owned(),
        "accent".to_owned(),
        45.0,
        Severity::Fail,
        "the label on a filled button",
    );
    row(
        out,
        "on-accent".to_owned(),
        "accent-hover".to_owned(),
        45.0,
        Severity::Fail,
        "the label on a hovered button",
    );
    // A second tier on a pair the context rows already gate at 30: below 30 an
    // accent is not visible as a fill at all, which is a defect; between 30 and
    // 45 it is visible but should not carry body text, which is a component's
    // call.
    row(
        out,
        "accent".to_owned(),
        "surface".to_owned(),
        45.0,
        Severity::Warn,
        "accent as text, which a component can mitigate",
    );
}

/// One set of rows per distinct context colour.
fn context_rows(out: &mut Vec<Pair>, mode: &ResolvedMode) {
    let mut covered: Vec<&str> = Vec::new();
    for (slot, family) in &mode.semantic {
        if surface_variant(slot).is_some() || covered.contains(&family.as_str()) {
            continue;
        }
        covered.push(family);

        row(
            out,
            slot.clone(),
            "surface".to_owned(),
            30.0,
            Severity::Fail,
            "a semantic fill must be visible against the page",
        );
        row(
            out,
            "fg".to_owned(),
            format!("{slot}-bg"),
            90.0,
            Severity::Fail,
            "body text inside a callout",
        );
        // Thirty, not the forty-five a button label gets. See the module
        // documentation: for a family with a contrast shift this pair is
        // *arithmetically* capped below forty-five, and the pattern that does
        // carry text is `fg` on `{slot}-bg`, gated at ninety just above.
        if slot != "accent" {
            row(
                out,
                format!("on-{slot}"),
                slot.clone(),
                30.0,
                Severity::Fail,
                "a fill's own foreground must be visible on it",
            );
        }
        row(
            out,
            format!("{slot}-border"),
            "surface".to_owned(),
            8.0,
            Severity::Warn,
            "a status border must be visible",
        );
    }
}

/// Resolves a semantic name to the encoded color it points at.
///
/// Public so the JSON target can publish the same measurements this gate makes,
/// from the same resolution — a published number that came from a second
/// lookup would be a number nobody checked.
pub fn resolve(mode: &ResolvedMode, semantic: &str) -> Option<noctua_core::Rgb> {
    let alias = semantic_view(mode)
        .into_iter()
        .find(|(name, _)| name == semantic)?;
    let (family, role) = alias.1;
    let resolved = mode.families.get(&family)?;
    resolved
        .steps
        .iter()
        .find(|step| step.role == role)
        .map(|step| step.primary().rgb)
}

/// The semantic contract, as `(name, (family, role))`.
///
/// Duplicated from `noctua-emit` deliberately: a gate that imported the
/// emitter's view of the contract would agree with it by construction, and
/// then a bug in that view would pass its own check. Written from the contract
/// rather than transcribed from the other implementation, so the two are two
/// readings of the same rule rather than one reading and a copy.
///
/// The xtask test `the_gate_and_the_emitter_resolve_every_token_alike` compares
/// them, because independence that nobody checks is just drift.
pub fn semantic_view(mode: &ResolvedMode) -> Vec<(String, (String, String))> {
    let mut out = Vec::new();
    let mut push = |name: String, family: &str, role: &str| {
        out.push((name, (family.to_owned(), role.to_owned())));
    };

    for (slot, family) in &mode.semantic {
        if let Some(variant) = surface_variant(slot) {
            push(format!("surface{variant}"), family, "bg-app");
            push(format!("surface-subtle{variant}"), family, "bg-subtle");
            push(format!("surface-raised{variant}"), family, "bg-element");
            push(format!("fg{variant}"), family, "text-strong");
            push(format!("fg-muted{variant}"), family, "text-muted");
            push(format!("border{variant}"), family, "border-element");
            push(format!("border-strong{variant}"), family, "border-strong");
            continue;
        }

        push(slot.clone(), family, "solid");
        push(format!("{slot}-hover"), family, "solid-hover");
        push(format!("{slot}-bg"), family, "bg-subtle");
        push(format!("{slot}-border"), family, "border-element");
        push(format!("on-{slot}"), family, "bg-app");
        if slot == "accent" {
            push("ring".to_owned(), family, "border-strong");
        }
    }

    out
}

/// One measured pair.
#[derive(Debug, Clone)]
pub struct Measured {
    /// Semantic name of the foreground.
    pub foreground: String,
    /// Semantic name of the background.
    pub background: String,
    /// APCA contrast, as a magnitude.
    pub lc: f64,
    /// WCAG 2.1 contrast ratio, for compliance reporting.
    pub wcag: f64,
    /// The threshold this pair is held to.
    pub minimum: f64,
    /// Whether falling short is a defect or a judgement call.
    pub severity: Severity,
    /// Why this threshold, in one clause.
    pub reason: &'static str,
}

impl Measured {
    /// Whether the pair clears its threshold.
    #[must_use]
    pub fn passes(&self) -> bool {
        self.lc >= self.minimum
    }
}

/// Every pair of one mode, measured.
///
/// The gate's own numbers, exposed so the JSON target can publish them rather
/// than compute them: an emitter that did its own colour math would publish
/// figures nothing had checked, which is worse than publishing none.
#[must_use]
pub fn measure(mode: &ResolvedMode) -> Vec<Measured> {
    pairs(mode)
        .into_iter()
        .filter_map(|pair| {
            let fg = resolve(mode, &pair.foreground)?;
            let bg = resolve(mode, &pair.background)?;
            Some(Measured {
                foreground: pair.foreground,
                background: pair.background,
                lc: apca(fg, bg).abs(),
                wcag: noctua_core::wcag21(fg, bg),
                minimum: pair.minimum,
                severity: pair.severity,
                reason: pair.reason,
            })
        })
        .collect()
}

/// Checks every declared pair, in every theme and mode.
#[must_use]
pub fn check(palette: &Palette) -> Report {
    let mut report = Report::default();

    for theme in &palette.themes {
        for mode in &theme.modes {
            for pair in pairs(mode) {
                let (Some(foreground), Some(background)) = (
                    resolve(mode, &pair.foreground),
                    resolve(mode, &pair.background),
                ) else {
                    // A spec that defines no `warning` family simply has no
                    // warning rows. That is not a failure.
                    continue;
                };

                report.checked += 1;
                let achieved = apca(foreground, background).abs();
                let margin = achieved - pair.minimum;
                if margin < 0.0 {
                    report.findings.push(Finding {
                        gate: GATE,
                        severity: pair.severity,
                        where_: format!(
                            "{}/{} {} on {}",
                            theme.name,
                            mode.mode.id(),
                            pair.foreground,
                            pair.background
                        ),
                        // Four decimals for the same reason the site uses
                        // them: at one, a pair short by 0.0003 printed as
                        // "Lc 45.0, needs 45" and read as a contradiction.
                        message: format!(
                            "Lc {achieved:.4}, needs {:.0} ({})",
                            pair.minimum, pair.reason
                        ),
                        margin: Some(margin),
                    });
                }
            }
        }
    }

    report
}

/// The full matrix as `(theme, mode, foreground, background, Lc, minimum)`.
///
/// For the docs site and the compliance report, which want every value rather
/// than only the failures.
#[must_use]
pub fn matrix(palette: &Palette) -> Vec<(String, &'static str, String, String, f64, f64)> {
    let mut rows = Vec::new();
    for theme in &palette.themes {
        for mode in &theme.modes {
            for pair in pairs(mode) {
                if let (Some(fg), Some(bg)) = (
                    resolve(mode, &pair.foreground),
                    resolve(mode, &pair.background),
                ) {
                    rows.push((
                        theme.name.clone(),
                        mode.mode.id(),
                        pair.foreground,
                        pair.background,
                        apca(fg, bg).abs(),
                        pair.minimum,
                    ));
                }
            }
        }
    }
    rows
}

/// Every checked pair with the colors it resolved to.
///
/// Exposed for the WCAG compliance report, which needs the same pairs measured
/// with a different metric.
#[must_use]
pub fn pairs_with_colors(
    palette: &Palette,
) -> Vec<(
    String,
    &'static str,
    String,
    String,
    noctua_core::Rgb,
    noctua_core::Rgb,
)> {
    let mut rows = Vec::new();
    for theme in &palette.themes {
        for mode in &theme.modes {
            for pair in pairs(mode) {
                if let (Some(fg), Some(bg)) = (
                    resolve(mode, &pair.foreground),
                    resolve(mode, &pair.background),
                ) {
                    rows.push((
                        theme.name.clone(),
                        mode.mode.id(),
                        pair.foreground,
                        pair.background,
                        fg,
                        bg,
                    ));
                }
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        noctua_engine::build(&noctua_spec::load(path).expect("valid")).expect("builds")
    }

    fn shipped_pairs() -> Vec<Pair> {
        pairs(&shipped().themes[0].modes[0])
    }

    #[test]
    fn the_matrix_crosses_family_boundaries() {
        // The whole reason this gate exists: the engine only ever checked a
        // role against its own family's step 1.
        let rows = shipped_pairs();
        assert!(
            rows.iter()
                .any(|p| p.foreground == "accent" && p.background == "surface"),
            "accent text on the neutral surface must be checked"
        );
        assert!(
            rows.iter()
                .any(|p| p.foreground == "fg" && p.background == "danger-bg"),
            "text inside a danger callout must be checked"
        );
    }

    /// The gap this table was generated to close: every `*-border` token was
    /// ungated while the table was written by hand, because nothing reports a
    /// row nobody thought of.
    ///
    /// Stated as *colours*, not names. An alias resolves to the same step as
    /// the slot it aliases, so `rejected` is covered by the row that gates
    /// `danger` — measuring it again would report one defect four times.
    #[test]
    fn every_semantic_colour_appears_in_at_least_one_pair() {
        let palette = shipped();
        let mode = &palette.themes[0].modes[0];
        let rows = pairs(mode);

        let gated: Vec<(String, String)> = rows
            .iter()
            .flat_map(|p| [&p.foreground, &p.background])
            .filter_map(|name| {
                semantic_view(mode)
                    .into_iter()
                    .find(|(token, _)| token == name)
                    .map(|(_, target)| target)
            })
            .collect();

        for (token, target) in semantic_view(mode) {
            // A hover fill is only ever a background, and `accent-hover` has
            // its own row as one; `-hover` as a foreground is not shipped.
            if token.ends_with("-hover") {
                continue;
            }
            assert!(
                gated.contains(&target),
                "--nc-color-{token} resolves to {}-{}, which nothing gates",
                target.0,
                target.1
            );
        }
    }

    /// Two rows measuring the same colours at the same threshold would report
    /// one defect twice, which is how thirty aliases turn one failure into
    /// thirty lines.
    #[test]
    fn no_two_rows_measure_the_same_thing() {
        let mut seen: Vec<(String, String, u64)> = Vec::new();
        for pair in shipped_pairs() {
            let key = (
                pair.foreground.clone(),
                pair.background.clone(),
                pair.minimum.to_bits(),
            );
            assert!(
                !seen.contains(&key),
                "{} on {} at {} is listed twice",
                pair.foreground,
                pair.background,
                pair.minimum
            );
            seen.push(key);
        }
    }

    #[test]
    fn every_pair_is_checked_in_every_theme_and_mode() {
        let palette = shipped();
        let report = check(&palette);
        let expected = palette.themes.len() * 2 * shipped_pairs().len();
        assert_eq!(report.checked, expected, "some pairs went unchecked");
    }

    #[test]
    fn a_focus_ring_is_a_hard_requirement() {
        let rows = shipped_pairs();
        let ring = rows
            .iter()
            .find(|p| p.foreground == "ring")
            .expect("a focus ring pair");
        assert_eq!(
            ring.severity,
            Severity::Fail,
            "an invisible focus ring is a keyboard trap"
        );
        assert!(
            ring.minimum >= 30.0,
            "APCA's floor for a visible non-text element"
        );
    }

    #[test]
    fn findings_report_the_margin() {
        let palette = shipped();
        for finding in check(&palette).findings {
            assert!(finding.margin.is_some(), "{finding}");
            assert!(
                finding.margin.expect("margin") < 0.0,
                "only shortfalls are reported"
            );
        }
    }

    #[test]
    fn the_matrix_covers_the_same_ground_as_the_gate() {
        let palette = shipped();
        assert_eq!(matrix(&palette).len(), check(&palette).checked);
    }
}
