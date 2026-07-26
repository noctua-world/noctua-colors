//! Perceptual spacing and lightness consistency.
//!
//! Three gates that all ask whether the ramps hold together.
//!
//! **Adjacent steps** must be neither indistinguishable nor jarring. A pair
//! closer than the just-noticeable difference is two names for one color; a
//! pair far apart reads as a missing step.
//!
//! **Cross-family lightness** must agree. At a given step, every family should
//! sit at roughly the same lightness, or a design that swaps accent for danger
//! gets a different weight of color and the layout visibly shifts. This is the
//! gate that catches a family whose curve was tuned in isolation.
//!
//! **The neutral temperatures** must be tellable apart. A `gray-warm` nobody can
//! distinguish from `gray` is not a subtle variant, it is a duplicate under a
//! second name — and that shipped: the first version of the feature emitted the
//! two ramps as byte-identical hex at most steps, 0.0015 apart at their furthest,
//! because nothing asserted otherwise.
//!
//! # Why that one is checked on the peak
//!
//! Because the ends of the ramp cannot hold a tint at all, and no setting
//! changes it. Tint is stored as *relative* chroma, and the gamut allows a
//! maximum chroma of 0.0060 at the lightest step and 0.0236 at the darkest — so
//! even at full strength the extremes stay within a fraction of a
//! just-noticeable difference of each other. Requiring separation at every step
//! would be requiring the impossible.
//!
//! What is achievable, and what matters, is that the temperature reads
//! *somewhere*: in the mid-tones, where the gamut is widest and where borders
//! and muted text live. So the check is on the widest separation the ramp
//! reaches, and the margin says how much room that leaves.

use noctua_core::{JND, delta_e_ok};
use noctua_engine::{Palette, ResolvedStep};

use crate::{Finding, Report, Severity};

const GATE: &str = "spacing";

/// Adjacent steps closer than this are two names for one color.
const MINIMUM_STEP: f64 = 0.012;

/// Adjacent steps further apart than this read as a missing step.
const MAXIMUM_STEP: f64 = 0.26;

/// How far apart two families may sit in lightness at the same step.
///
/// Roles are anchored on contrast, and different hues reach a given contrast
/// at different lightnesses, so some spread is expected and correct. What this
/// catches is a family that has drifted out of the group entirely.
const MAXIMUM_LIGHTNESS_SPREAD: f64 = 0.10;

/// Separation two neutral temperatures should reach at their furthest step.
///
/// One and a half just-noticeable differences. A single JND is the threshold at
/// which two swatches side by side can be told apart at all, which is too thin a
/// margin for a variant whose entire purpose is to be recognisably a different
/// temperature — half a JND of drift and the distinction is gone.
const TEMPERATURE_TARGET: f64 = 0.025;

// A floor above the ceiling would silently disable both halves of the gate, and
// a temperature target under the just-noticeable difference would let the gate
// certify two ramps nobody can tell apart.
const _: () = assert!(MINIMUM_STEP > 0.0 && MINIMUM_STEP < MAXIMUM_STEP);
const _: () = assert!(TEMPERATURE_TARGET > JND);

/// Checks adjacent-step distance, monotonicity and cross-family agreement.
#[must_use]
pub fn check(palette: &Palette) -> Report {
    let mut report = Report::default();

    for theme in &palette.themes {
        for mode in &theme.modes {
            let off_ramp = |role: &str| palette.shiftable_roles.iter().any(|r| r == role);

            for family in mode.families.values() {
                for pair in family.steps.windows(2) {
                    // A transition into or out of a solid step is the scale's
                    // one deliberate discontinuity, not a gap in the ramp.
                    if off_ramp(&pair[0].role) || off_ramp(&pair[1].role) {
                        continue;
                    }
                    report.checked += 1;
                    let apart = delta_e_ok(
                        pair[0].primary().oklch.to_oklab(),
                        pair[1].primary().oklch.to_oklab(),
                    );
                    let where_ = format!(
                        "{}/{} {} steps {}-{}",
                        theme.name,
                        mode.mode.id(),
                        family.name,
                        pair[0].index,
                        pair[1].index
                    );

                    if apart < MINIMUM_STEP {
                        report.findings.push(Finding {
                            gate: GATE,
                            severity: Severity::Fail,
                            where_: where_.clone(),
                            message: format!("only {apart:.4} apart, which is indistinguishable"),
                            margin: Some(apart - MINIMUM_STEP),
                        });
                    } else if apart > MAXIMUM_STEP {
                        report.findings.push(Finding {
                            gate: GATE,
                            severity: Severity::Warn,
                            where_,
                            message: format!("{apart:.4} apart, which reads as a missing step"),
                            margin: Some(MAXIMUM_STEP - apart),
                        });
                    }
                }

                // Lightness must not reverse along the ramp. Solid steps are
                // excluded: they are chosen for recognition, and every real
                // twelve-step scale steps off its own trajectory there.
                let lightnesses: Vec<f64> = family
                    .steps
                    .iter()
                    .filter(|s| !off_ramp(&s.role))
                    .map(|s| s.primary().oklch.l)
                    .collect();
                report.checked += 1;
                let ascending = lightnesses.windows(2).all(|p| p[1] >= p[0]);
                let descending = lightnesses.windows(2).all(|p| p[1] <= p[0]);
                if !ascending && !descending {
                    report.findings.push(Finding {
                        gate: GATE,
                        severity: Severity::Fail,
                        where_: format!("{}/{} {}", theme.name, mode.mode.id(), family.name),
                        message: "lightness reverses along the ramp".to_owned(),
                        margin: None,
                    });
                }
            }

            report.absorb(cross_family(palette, theme, mode, &off_ramp));
        }
    }

    report.absorb(temperatures(palette));
    report
}

/// The dense neutral ramps must be tellable apart from one another.
///
/// Once per palette, not once per theme: the ramps are emitted a single time and
/// shared by every theme and both modes, so measuring them thirty-six times
/// would report one finding thirty-six ways.
fn temperatures(palette: &Palette) -> Report {
    let mut report = Report::default();

    let ramps: Vec<(&String, &Vec<ResolvedStep>)> = palette.neutral_ramps.iter().collect();
    for (i, (left, left_steps)) in ramps.iter().enumerate() {
        for (right, right_steps) in ramps.iter().skip(i + 1) {
            report.checked += 1;

            // The widest the two ever get. See the module documentation: the
            // ends of a ramp cannot hold a tint, so a per-step check would be
            // asking the gamut for room it does not have.
            let peak = left_steps
                .iter()
                .zip(right_steps.iter())
                .map(|(a, b)| {
                    delta_e_ok(a.primary().oklch.to_oklab(), b.primary().oklch.to_oklab())
                })
                .fold(0.0, f64::max);

            if peak >= TEMPERATURE_TARGET {
                continue;
            }

            // Under a just-noticeable difference they are not a subtle variant,
            // they are the same colour emitted twice.
            let severity = if peak < JND {
                Severity::Fail
            } else {
                Severity::Warn
            };
            let message = if peak < JND {
                format!(
                    "never more than {peak:.4} apart, under the {JND} just-noticeable \
                     difference — these are the same ramp under two names. Raise the \
                     tint strengths in `[neutral]`, or drop one of the hues"
                )
            } else {
                format!(
                    "never more than {peak:.4} apart, wants {TEMPERATURE_TARGET:.2}. Raise \
                     the tint strengths in `[neutral]`; hue placement cannot fix this, \
                     because two nearly-achromatic ramps are close whatever their hue"
                )
            };

            report.findings.push(Finding {
                gate: GATE,
                severity,
                where_: format!("{left} vs {right}"),
                message,
                margin: Some(peak - TEMPERATURE_TARGET),
            });
        }
    }

    report
}

/// Families should sit at comparable lightness at the same step.
fn cross_family(
    palette: &Palette,
    theme: &noctua_engine::ResolvedTheme,
    mode: &noctua_engine::ResolvedMode,
    off_ramp: &dyn Fn(&str) -> bool,
) -> Report {
    let mut report = Report::default();
    {
        {
            for index in 0..palette.roles.len() {
                // Families are *meant* to diverge at the solid steps; that is
                // what keeps them apart for a dichromat. This gate looks for
                // accidental drift, so it skips the deliberate kind.
                if palette.roles.get(index).is_some_and(|r| off_ramp(r)) {
                    continue;
                }
                let lightnesses: Vec<(String, f64)> = mode
                    .families
                    .values()
                    .filter_map(|family| {
                        family
                            .steps
                            .get(index)
                            .map(|s| (family.name.clone(), s.primary().oklch.l))
                    })
                    .collect();
                if lightnesses.len() < 2 {
                    continue;
                }

                report.checked += 1;
                let highest = lightnesses.iter().map(|(_, l)| *l).fold(f64::MIN, f64::max);
                let lowest = lightnesses.iter().map(|(_, l)| *l).fold(f64::MAX, f64::min);
                let spread = highest - lowest;
                if spread > MAXIMUM_LIGHTNESS_SPREAD {
                    let widest = lightnesses
                        .iter()
                        .map(|(name, l)| format!("{name} {l:.3}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    report.findings.push(Finding {
                        gate: GATE,
                        severity: Severity::Warn,
                        where_: format!(
                            "{}/{} step {} ({})",
                            theme.name,
                            mode.mode.id(),
                            index + 1,
                            palette.roles.get(index).map_or("", String::as_str)
                        ),
                        message: format!("families span {spread:.3} in lightness: {widest}"),
                        margin: Some(MAXIMUM_LIGHTNESS_SPREAD - spread),
                    });
                }
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        noctua_engine::build(&noctua_spec::load(path).expect("valid")).expect("builds")
    }

    #[test]
    fn the_gate_runs_over_every_family_in_every_theme_and_mode() {
        let palette = shipped();
        let report = check(&palette);
        assert!(report.checked > 0, "the spacing gate performed no checks");

        // Adjacent pairs, minus those touching a solid step, plus one
        // monotonicity check per family, plus one cross-family check per
        // on-ramp role.
        let families = palette.themes[0].modes[0].families.len();
        let steps = palette.roles.len();
        let off_ramp = palette.shiftable_roles.len();
        let pairs_per_family = (steps - 1) - (off_ramp + 1);
        let per_mode = families * (pairs_per_family + 1) + (steps - off_ramp);

        // Plus the temperature pairs, which are counted once for the palette
        // rather than once per theme: the dense ramps are emitted once.
        let ramps = palette.neutral_ramps.len();
        let temperature_pairs = ramps * (ramps - 1) / 2;

        assert_eq!(
            report.checked,
            palette.themes.len() * 2 * per_mode + temperature_pairs
        );
    }

    /// The defect this check was added for: `gray-warm` shipped byte-identical
    /// to `gray`, 0.0015 apart at its furthest, because nothing looked.
    #[test]
    fn the_neutral_temperatures_are_tellable_apart() {
        let palette = shipped();
        assert!(
            palette.neutral_ramps.len() > 1,
            "the shipped spec is meant to emit temperatures for this to check"
        );

        for finding in check(&palette).findings {
            assert!(
                !finding.message.contains("apart, under the"),
                "two ramps are the same colour: {finding}"
            );
        }
    }

    /// Under a just-noticeable difference is a defect, not a judgement call:
    /// the variant is a duplicate, and no component can mitigate that.
    #[test]
    fn temperatures_within_a_jnd_fail_rather_than_warn() {
        let mut palette = shipped();

        // Collapse one temperature onto the base by copying it, which is what
        // too little tint strength amounts to.
        let base = palette
            .neutral_ramps
            .get(noctua_engine::BASE_NEUTRAL_RAMP)
            .expect("a base ramp")
            .clone();
        let variant = palette
            .neutral_ramps
            .keys()
            .find(|name| *name != noctua_engine::BASE_NEUTRAL_RAMP)
            .expect("a variant")
            .clone();
        palette.neutral_ramps.insert(variant.clone(), base);

        let findings = check(&palette).findings;
        let collapsed: Vec<_> = findings
            .iter()
            .filter(|f| f.where_.contains(&variant) && f.message.contains("same ramp"))
            .collect();
        assert!(!collapsed.is_empty(), "the collapse went unreported");
        for finding in collapsed {
            assert_eq!(finding.severity, Severity::Fail, "{finding}");
            assert!(finding.margin.is_some(), "{finding}");
        }
    }

    /// Solid steps are meant to leave the ramp; the gate must not call that
    /// drift. Real twelve-step scales all do it.
    #[test]
    fn deliberate_divergence_at_the_solid_steps_is_not_reported() {
        let palette = shipped();
        for finding in check(&palette).findings {
            for role in &palette.shiftable_roles {
                assert!(
                    !finding.where_.contains(role.as_str()),
                    "the gate flagged a deliberate shift: {finding}"
                );
            }
        }
    }

    #[test]
    fn indistinguishable_neighbours_are_a_failure_and_wide_gaps_only_a_warning() {
        // A step nobody can see is a defect; a wide gap is a judgement call.
        for finding in check(&shipped()).findings {
            if finding.message.contains("indistinguishable") {
                assert_eq!(finding.severity, Severity::Fail);
            }
        }
    }
}
