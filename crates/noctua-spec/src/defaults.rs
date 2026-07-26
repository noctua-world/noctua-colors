//! What a spec means when it does not say.
//!
//! The developer edits colors, not configuration. Everything here exists so
//! that a three-line spec produces a complete, checked, usable system — and so
//! that the numbers a designer would want to argue about live in one readable
//! place instead of scattered through the engine.

use toml::Spanned;

use crate::curve::CurveSpec;
use crate::model::{ApcaTarget, DeltaLTarget, DensityBand, Role, TargetSpec};

/// Wraps a value with a zero-width span.
///
/// Defaults have no position in the source, so a diagnostic about one points
/// at the spec as a whole rather than at a line the author never wrote.
fn unspanned<T>(value: T) -> Spanned<T> {
    Spanned::new(0..0, value)
}

fn lightness(value: f64) -> TargetSpec {
    TargetSpec {
        lightness: Some(unspanned(value)),
        ..TargetSpec::default()
    }
}

fn delta_l(against: &str, amount: f64) -> TargetSpec {
    TargetSpec {
        delta_l: Some(DeltaLTarget {
            against: unspanned(against.to_owned()),
            amount: unspanned(amount),
        }),
        ..TargetSpec::default()
    }
}

fn apca(against: &str, lc: f64) -> TargetSpec {
    TargetSpec {
        apca: Some(ApcaTarget {
            against: unspanned(against.to_owned()),
            lc: unspanned(lc),
        }),
        ..TargetSpec::default()
    }
}

fn role(name: &str, light: TargetSpec, dark: TargetSpec) -> Role {
    Role {
        name: unspanned(name.to_owned()),
        light,
        dark,
        shift: false,
    }
}

/// A role that a family's colour-vision shift applies to.
fn shiftable(name: &str, light: TargetSpec, dark: TargetSpec) -> Role {
    Role {
        shift: true,
        ..role(name, light, dark)
    }
}

/// The default twelve-step functional scale.
///
/// The roles follow Radix Colors' functional split, which has held up well:
/// each step exists for a job, and a component reaches for the job rather than
/// for a number. Numeric aliases are still emitted for interop, but the names
/// are canonical.
///
/// Surfaces and borders are anchored by lightness separation, solids and text
/// by APCA. See [`TargetSpec`] for why the metric is not uniform.
///
/// Both `lc` and `amount` are magnitudes; the solver takes the direction from
/// the mode, so light and dark read the same way.
///
/// # Why light and dark are not mirror images
///
/// APCA is asymmetric, and the asymmetry is large. Measured on the default
/// accent family: a lightness separation of 0.265 from the app background
/// reads as **Lc 46 in light mode but only Lc 15 in dark mode**. Reverse
/// polarity — light on dark — is far less sensitive, so a dark interface needs
/// roughly **1.45x** the lightness separation for the same felt contrast.
///
/// Two consequences are baked into the numbers below.
///
/// The dark separations are scaled up accordingly. Copying the light values
/// across, which is the obvious thing to do, leaves the surface steps bunched
/// against the background and then jumping to the solid: the eight ramp steps
/// would cover a quarter of the lightness range and step nine would leap over
/// half of it in one go.
///
/// The dark *solid* targets are lower, not equal. Solids are brand colors, and
/// a brand should look like itself in both modes. Demanding the same Lc in
/// dark mode does not produce the same color — it produces a much paler one,
/// because reaching Lc 58 against a dark ground means climbing to lightness
/// 0.75. Text targets are held near-equal, because text contrast is an
/// accessibility floor rather than an aesthetic choice.
#[must_use]
pub fn roles() -> Vec<Role> {
    const APP: &str = "bg-app";
    vec![
        // The page itself. Anchored outright: everything else is relative to
        // it, so it needs a fixed point. Not pure white or pure black —
        // both are harsh, and both leave the ramp no room to go further.
        role("bg-app", lightness(0.9940), lightness(0.1780)),
        // Striped rows, code blocks, anything that should read as "the page,
        // but marked".
        role("bg-subtle", delta_l(APP, 0.0190), delta_l(APP, 0.0280)),
        // Resting state of a control.
        role("bg-element", delta_l(APP, 0.0430), delta_l(APP, 0.0620)),
        role(
            "bg-element-hover",
            delta_l(APP, 0.0680),
            delta_l(APP, 0.0990),
        ),
        role(
            "bg-element-active",
            delta_l(APP, 0.0950),
            delta_l(APP, 0.1380),
        ),
        // Separators that should be felt more than seen.
        role("border-subtle", delta_l(APP, 0.1350), delta_l(APP, 0.1960)),
        // The workhorse border, and focus rings.
        role("border-element", delta_l(APP, 0.1850), delta_l(APP, 0.2680)),
        role("border-strong", delta_l(APP, 0.2650), delta_l(APP, 0.4150)),
        // The step a brand is recognized by: a filled button, a selected tab.
        // Above the Lc 45 floor for large or bold text, which is what sits on
        // a solid, and above the Lc 30 floor for it to read as an element.
        //
        // Dark aims at 45.25, and that quarter of an Lc is an allowance rather
        // than slack. A solid is *solved* against its own family's `bg-app` but
        // *shipped* against the page, which is the neutral's — two steps at the
        // same lightness differing only in the chroma of their tint. That hair
        // of chroma moves the measured contrast by up to **0.016 Lc**, so a
        // target of exactly 45 left six accent hues a ten-thousandth under it
        // and the contrast gate reporting eighteen findings whose real margin
        // was nothing.
        //
        // The size is deliberate in both directions. It has to exceed 0.016, and
        // 0.25 clears that fifteen times over. It must also stay small: a whole
        // Lc was tried first and walked `progress`'s dark solid up into its own
        // `border-strong` — that family carries the largest negative shift, so
        // its solid sits *below* the border rather than above it, and a rising
        // target closes the gap instead of opening it. At 46 they were 0.0117
        // apart, under the 0.012 floor at which two steps are one colour.
        shiftable("solid", apca(APP, 58.0), apca(APP, 45.5)),
        shiftable("solid-hover", apca(APP, 66.0), apca(APP, 53.5)),
        // Secondary text. Clears the Lc 75 guideline for larger body text in
        // light mode; dark mode sits a little under it, which is why
        // `text-muted` is for secondary text and never for body copy.
        //
        // Light aims at 78 rather than 76 to close the ramp's one structural
        // gap. `text-muted` to `text-strong` is a 24 Lc jump, and light polarity
        // compresses that into a quarter of the lightness range — measured, the
        // step was 0.2684 in `violet-vivid` against a 0.26 ceiling while every
        // other adjacent pair in the same ramp sat between 0.02 and 0.15. Two Lc
        // takes the worst case to 0.2510. The alternative was to pull
        // `text-strong` down instead, which would have weakened the primary text
        // to fix a gap; raising the muted end *improves* secondary-text contrast
        // and still clears the 75 guideline.
        role("text-muted", apca(APP, 78.0), apca(APP, 72.0)),
        // Primary text. Targeted above the Lc 90 body-text guideline rather
        // than at it: the target is measured against the *page*, but body text
        // is often set on a card, which sits a little closer in lightness and
        // costs a few Lc. Aiming at 90 exactly shipped 87.5 on cards.
        role("text-strong", apca(APP, 100.0), apca(APP, 96.0)),
    ]
}

/// The default translucency ladder.
///
/// Twelve stops, spaced geometrically rather than evenly. The useful range for
/// a wash is crowded at the bottom — the difference between 2% and 4% is a
/// hairline and a hover state, while the difference between 80% and 92% is
/// nothing anyone can see — so even spacing would spend half the ladder on
/// values no interface distinguishes.
#[must_use]
pub fn alpha_percentages() -> Vec<f64> {
    vec![
        2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0, 80.0, 92.0,
    ]
}

/// The default relative-chroma curve.
///
/// Muted at both ends and richest in the middle. Backgrounds carrying full
/// chroma look garish and text carrying it looks cheap, while the solid steps
/// in the middle are exactly where a brand wants saturation.
#[must_use]
pub fn chroma_curve() -> CurveSpec {
    CurveSpec::Shorthand {
        ends: [0.14, 0.42],
        peak: 0.92,
        peak_at: 0.55,
    }
}

/// The default neutral step distribution.
///
/// Interfaces do not use grays evenly. Light mode spends its surfaces in a
/// narrow band just below white, dark mode in a narrow band just above black,
/// and the middle is mostly borders and disabled text. Weighting the two ends
/// puts the resolution where the discrimination is actually needed.
#[must_use]
pub fn density() -> Vec<DensityBand> {
    vec![
        DensityBand {
            range: [0.10, 0.25],
            weight: 3.0,
        },
        DensityBand {
            range: [0.85, 0.99],
            weight: 3.0,
        },
    ]
}

#[cfg(test)]
// Comparisons here are against literal values the code returns verbatim.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn there_are_twelve_roles_with_unique_names() {
        let roles = roles();
        assert_eq!(roles.len(), 12);

        let mut names: Vec<&str> = roles.iter().map(|r| r.name.get_ref().as_str()).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "role names must be unique");
    }

    #[test]
    fn every_role_sets_exactly_one_kind_of_target_in_both_modes() {
        for role in roles() {
            for (mode, target) in [("light", &role.light), ("dark", &role.dark)] {
                let set = usize::from(target.lightness.is_some())
                    + usize::from(target.apca.is_some())
                    + usize::from(target.delta_l.is_some());
                assert_eq!(set, 1, "{} {mode} sets {set} targets", role.name.get_ref());
            }
        }
    }

    #[test]
    fn only_the_app_background_is_anchored_absolutely() {
        // Everything else must be relative, or the ramp cannot be retuned by
        // moving one number.
        let anchored: Vec<String> = roles()
            .iter()
            .filter(|r| r.light.lightness.is_some())
            .map(|r| r.name.get_ref().clone())
            .collect();
        assert_eq!(anchored, ["bg-app"]);
    }

    #[test]
    fn every_reference_names_a_role_that_exists() {
        let roles = roles();
        let names: Vec<&str> = roles.iter().map(|r| r.name.get_ref().as_str()).collect();

        for role in &roles {
            for target in [&role.light, &role.dark] {
                let reference = target
                    .apca
                    .as_ref()
                    .map(|t| t.against.get_ref())
                    .or_else(|| target.delta_l.as_ref().map(|t| t.against.get_ref()));
                if let Some(reference) = reference {
                    assert!(
                        names.contains(&reference.as_str()),
                        "{} references unknown role {reference}",
                        role.name.get_ref()
                    );
                }
            }
        }
    }

    #[test]
    fn separations_and_contrasts_increase_down_the_ramp() {
        // A ramp whose steps did not grow monotonically apart would produce
        // neighbours that swap places.
        let roles = roles();
        let mut previous_delta = 0.0;
        let mut previous_lc = 0.0;
        for role in &roles {
            if let Some(target) = &role.light.delta_l {
                let amount = *target.amount.get_ref();
                assert!(
                    amount > previous_delta,
                    "{}: separation {amount} does not exceed {previous_delta}",
                    role.name.get_ref()
                );
                previous_delta = amount;
            }
            if let Some(target) = &role.light.apca {
                let lc = *target.lc.get_ref();
                assert!(
                    lc > previous_lc,
                    "{}: contrast {lc} does not exceed {previous_lc}",
                    role.name.get_ref()
                );
                previous_lc = lc;
            }
        }
    }

    #[test]
    fn the_default_chroma_curve_is_muted_at_both_ends() {
        let knots = chroma_curve().knots();
        let peak = knots[1][1];
        assert!(peak > knots[0][1], "peak must exceed the light end");
        assert!(peak > knots[2][1], "peak must exceed the dark end");
        for knot in &knots {
            assert!(
                (0.0..=1.0).contains(&knot[1]),
                "relative chroma out of range: {knot:?}"
            );
        }
    }

    #[test]
    fn density_bands_are_ordered_valid_and_non_overlapping() {
        let bands = density();
        let mut previous_high = 0.0;
        for band in &bands {
            assert!(band.range[0] < band.range[1], "empty band {:?}", band.range);
            assert!(band.weight > 0.0, "non-positive weight");
            assert!(
                band.range[0] >= previous_high,
                "bands overlap or are unsorted: {:?}",
                band.range
            );
            previous_high = band.range[1];
        }
    }
}
