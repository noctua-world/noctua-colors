//! Turning a role's target into a lightness.
//!
//! This is where "the repository versions the curves, not the colors" actually
//! happens. A step is never authored as a value; it is *solved* — find the
//! lightness at which this family, with its own chroma and hue curves, hits
//! the contrast the role asked for.
//!
//! # The fixed point
//!
//! The obvious approach — pick a lightness, look up the color — does not work,
//! because the pipeline is circular. Chroma is a fraction of what the gamut
//! allows at a given lightness, so chroma depends on lightness; and chroma
//! changes luminance, so contrast depends on chroma. Solving analytically
//! would mean inverting the whole chain including a gamut boundary.
//!
//! So the search runs the *entire* pipeline at each candidate lightness —
//! curves, hue correction, relative chroma, gamut mapping, encoding, metric —
//! and bisects. Contrast is monotone in lightness once the reference is fixed,
//! so bisection is well defined and terminates in a bounded number of steps.

use noctua_core::map::Mapped;
use noctua_core::{Gamut, Oklch, apca, map_into_gamut};

use crate::curve::Curve;

/// Which mode a step is being resolved for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mode {
    /// Light interfaces: a near-white background, everything else darker.
    Light,
    /// Dark interfaces: a near-black background, everything else lighter.
    Dark,
}

impl Mode {
    /// The stable identifier used in emitted output.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Both modes, in a fixed order.
    #[must_use]
    pub const fn all() -> [Self; 2] {
        [Self::Light, Self::Dark]
    }

    /// Which way a role moves away from its reference.
    ///
    /// This is what lets a spec write `lc = 90` once instead of `90` for light
    /// and `-90` for dark. In a light interface every role is darker than the
    /// app background; in a dark one, lighter.
    #[must_use]
    pub const fn direction(self) -> f64 {
        match self {
            Self::Light => -1.0,
            Self::Dark => 1.0,
        }
    }
}

/// Everything needed to produce one family's color at a given step.
#[derive(Debug, Clone)]
pub struct FamilyCurves {
    /// Hue in degrees over the ramp.
    pub hue: Curve,
    /// Relative chroma over the ramp.
    pub chroma: Curve,
    /// Corrective hue offsets, indexed by lightness.
    pub correction: Curve,
    /// Multiplier applied to relative chroma, from the theme and any
    /// per-family override.
    pub multiplier: f64,
}

impl FamilyCurves {
    /// The color this family produces at ramp position `t` and lightness `l`.
    ///
    /// The whole pipeline in one place: hue with torsion, then the corrective
    /// offset, then relative chroma resolved against this gamut's boundary at
    /// this exact lightness and hue, then gamut mapping.
    #[must_use]
    pub fn color_at(&self, t: f64, lightness: f64, gamut: Gamut) -> Mapped {
        let lightness = lightness.clamp(0.0, 1.0);
        let hue = self.hue.at(t) + self.correction.at(lightness);
        let relative = (self.chroma.at(t) * self.multiplier).clamp(0.0, 1.0);
        let chroma = relative * gamut.max_chroma(lightness, hue);
        map_into_gamut(
            Oklch {
                l: lightness,
                c: chroma,
                h: hue,
            },
            gamut,
        )
    }

    /// The relative chroma actually requested at `t`, before the gamut has a
    /// say. Reported so the docs site can show intent alongside outcome.
    #[must_use]
    pub fn relative_chroma_at(&self, t: f64) -> f64 {
        (self.chroma.at(t) * self.multiplier).clamp(0.0, 1.0)
    }
}

/// What a role is anchored to, once the spec's typed target is resolved.
#[derive(Debug, Clone, Copy)]
pub enum Anchor<'a> {
    /// A fixed lightness.
    Fixed(f64),
    /// APCA contrast against an already-resolved color.
    Apca {
        /// The color to measure against.
        reference: &'a Mapped,
        /// Target contrast magnitude in Lc.
        lc: f64,
    },
    /// Oklab lightness separation from an already-resolved color.
    DeltaL {
        /// The color to measure against.
        reference: &'a Mapped,
        /// Separation magnitude.
        amount: f64,
    },
}

/// The outcome of a solve.
#[derive(Debug, Clone, Copy)]
pub struct Solved {
    /// The lightness found.
    pub lightness: f64,
    /// The best the family could do, when the target was out of reach.
    pub shortfall: Option<f64>,
}

/// Bisection steps. Halving `[0, 1]` forty times resolves lightness far below
/// the fourth decimal at which values are quantized.
const STEPS: u32 = 40;

/// Solves for the lightness at which `anchor` is satisfied.
///
/// `shortfall` is set when the target is unreachable, carrying the best value
/// the family could actually achieve so the caller can say so.
#[must_use]
pub fn solve(
    curves: &FamilyCurves,
    t: f64,
    anchor: Anchor<'_>,
    mode: Mode,
    gamut: Gamut,
) -> Solved {
    match anchor {
        Anchor::Fixed(lightness) => Solved {
            lightness: lightness.clamp(0.0, 1.0),
            shortfall: None,
        },

        Anchor::DeltaL { reference, amount } => {
            // Lightness separation needs no search: it *is* a lightness.
            let target = reference.oklch.l + mode.direction() * amount;
            if (0.0..=1.0).contains(&target) {
                Solved {
                    lightness: target,
                    shortfall: None,
                }
            } else {
                let clamped = target.clamp(0.0, 1.0);
                let achievable = (clamped - reference.oklch.l).abs();
                Solved {
                    lightness: clamped,
                    shortfall: Some(achievable),
                }
            }
        }

        Anchor::Apca { reference, lc } => {
            let measure = |lightness: f64| {
                let color = curves.color_at(t, lightness, gamut);
                apca(color.rgb, reference.rgb).abs()
            };

            // Search away from the reference, in the direction the mode
            // implies: toward black in a light interface, toward white in a
            // dark one.
            let limit = if mode.direction() < 0.0 { 0.0 } else { 1.0 };
            let reachable = measure(limit);
            if reachable < lc {
                return Solved {
                    lightness: limit,
                    shortfall: Some(reachable),
                };
            }

            // Contrast rises monotonically from the reference toward the
            // limit, so the crossing is unique.
            let mut near = reference.oklch.l;
            let mut far = limit;
            for _ in 0..STEPS {
                let middle = f64::midpoint(near, far);
                if measure(middle) < lc {
                    near = middle;
                } else {
                    far = middle;
                }
            }
            Solved {
                lightness: far,
                shortfall: None,
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn curves() -> FamilyCurves {
        FamilyCurves {
            hue: Curve::hue([[0.0, 264.0], [1.0, 264.0]]),
            chroma: Curve::constant(0.6),
            correction: Curve::constant(0.0),
            multiplier: 1.0,
        }
    }

    fn app_background(mode: Mode) -> Mapped {
        let lightness = if mode == Mode::Light { 0.994 } else { 0.178 };
        curves().color_at(0.0, lightness, Gamut::Srgb)
    }

    #[test]
    fn a_fixed_anchor_is_returned_as_given() {
        let solved = solve(
            &curves(),
            0.0,
            Anchor::Fixed(0.42),
            Mode::Light,
            Gamut::Srgb,
        );
        assert_eq!(solved.lightness, 0.42);
        assert!(solved.shortfall.is_none());
    }

    #[test]
    fn separation_moves_away_from_the_reference_in_both_modes() {
        for mode in Mode::all() {
            let reference = app_background(mode);
            let solved = solve(
                &curves(),
                0.2,
                Anchor::DeltaL {
                    reference: &reference,
                    amount: 0.05,
                },
                mode,
                Gamut::Srgb,
            );
            let moved = solved.lightness - reference.oklch.l;
            assert!(
                (moved.abs() - 0.05).abs() < 1e-12,
                "{}: moved {moved}",
                mode.id()
            );
            assert!(
                moved.signum() == mode.direction(),
                "{}: moved the wrong way ({moved})",
                mode.id()
            );
        }
    }

    #[test]
    fn an_apca_target_is_hit_in_both_modes_from_one_positive_number() {
        // The point of magnitude-plus-mode: the same `lc` reads correctly in
        // light and dark without the author tracking a sign.
        for mode in Mode::all() {
            let reference = app_background(mode);
            for target in [45.0, 60.0, 75.0, 90.0] {
                let solved = solve(
                    &curves(),
                    0.9,
                    Anchor::Apca {
                        reference: &reference,
                        lc: target,
                    },
                    mode,
                    Gamut::Srgb,
                );
                assert!(
                    solved.shortfall.is_none(),
                    "{} Lc {target} unreachable",
                    mode.id()
                );

                let color = curves().color_at(0.9, solved.lightness, Gamut::Srgb);
                let achieved = apca(color.rgb, reference.rgb).abs();
                assert!(
                    (achieved - target).abs() < 0.5,
                    "{} wanted Lc {target}, got {achieved}",
                    mode.id()
                );
            }
        }
    }

    #[test]
    fn apca_polarity_follows_the_mode() {
        for mode in Mode::all() {
            let reference = app_background(mode);
            let solved = solve(
                &curves(),
                0.9,
                Anchor::Apca {
                    reference: &reference,
                    lc: 75.0,
                },
                mode,
                Gamut::Srgb,
            );
            let color = curves().color_at(0.9, solved.lightness, Gamut::Srgb);
            let signed = apca(color.rgb, reference.rgb);
            match mode {
                Mode::Light => assert!(signed > 0.0, "light mode should be dark-on-light"),
                Mode::Dark => assert!(signed < 0.0, "dark mode should be light-on-dark"),
            }
        }
    }

    #[test]
    fn an_unreachable_target_reports_the_best_it_could_do() {
        let reference = app_background(Mode::Light);
        let solved = solve(
            &curves(),
            0.9,
            Anchor::Apca {
                reference: &reference,
                lc: 200.0,
            },
            Mode::Light,
            Gamut::Srgb,
        );
        let shortfall = solved.shortfall.expect("200 Lc is impossible");
        assert!(
            shortfall > 90.0 && shortfall < 110.0,
            "reported {shortfall}"
        );
    }

    #[test]
    fn separation_beyond_the_ramp_reports_the_best_it_could_do() {
        let reference = app_background(Mode::Light);
        let solved = solve(
            &curves(),
            0.5,
            Anchor::DeltaL {
                reference: &reference,
                amount: 2.0,
            },
            Mode::Light,
            Gamut::Srgb,
        );
        assert_eq!(solved.lightness, 0.0);
        let shortfall = solved.shortfall.expect("cannot move 2.0 in lightness");
        assert!((shortfall - 0.994).abs() < 1e-9, "reported {shortfall}");
    }

    #[test]
    fn higher_targets_land_further_from_the_reference() {
        let reference = app_background(Mode::Light);
        let mut previous = reference.oklch.l;
        for target in [30.0, 45.0, 60.0, 75.0, 90.0] {
            let solved = solve(
                &curves(),
                0.9,
                Anchor::Apca {
                    reference: &reference,
                    lc: target,
                },
                Mode::Light,
                Gamut::Srgb,
            );
            assert!(
                solved.lightness < previous,
                "Lc {target} landed at {} which is not darker than {previous}",
                solved.lightness
            );
            previous = solved.lightness;
        }
    }

    #[test]
    fn the_solved_color_respects_its_relative_chroma() {
        // A step must end up at the requested fraction of the boundary, or
        // relative chroma is not doing its job.
        let curves = curves();
        let color = curves.color_at(0.5, 0.6, Gamut::Srgb);
        let boundary = Gamut::Srgb.max_chroma(color.oklch.l, color.oklch.h);
        let achieved = color.oklch.c / boundary;
        assert!(
            (achieved - 0.6).abs() < 0.01,
            "asked 0.6 of the boundary, got {achieved}"
        );
    }
}
