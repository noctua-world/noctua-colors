//! Curves over the normalized step position.
//!
//! A family is described by curves over `t ∈ [0, 1]`, where `t = 0` is the
//! first step of a scale and `t = 1` the last. This module holds the
//! *authoring* forms and how they desugar; evaluation lives in
//! `noctua-engine`.

use serde::Deserialize;

/// A curve, as authored.
///
/// Three forms, in increasing order of control. All desugar to knots:
///
/// ```toml
/// cr = 0.8                                              # constant
/// cr = { ends = [0.15, 0.35], peak = 0.92 }             # shorthand
/// cr = { knots = [[0.0, 0.15], [0.55, 0.92], [1.0, 0.35]] }
/// ```
///
/// The shorthand covers the shape almost every relative-chroma curve wants —
/// low at both ends, peaking in the middle — without making the author think
/// about knot placement.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum CurveSpec {
    /// The same value at every step.
    Constant(f64),
    /// A peaked shape: value at `t = 0`, at the peak, and at `t = 1`.
    Shorthand {
        /// Values at `t = 0` and `t = 1`.
        ends: [f64; 2],
        /// Value at the peak.
        peak: f64,
        /// Where the peak sits. Defaults to slightly past the middle, which
        /// is where chroma naturally peaks on a lightness ramp.
        #[serde(default = "default_peak_at")]
        peak_at: f64,
    },
    /// Explicit `(t, value)` knots, which must be sorted by `t`.
    Explicit {
        /// The knots.
        knots: Vec<[f64; 2]>,
    },
}

fn default_peak_at() -> f64 {
    0.55
}

impl CurveSpec {
    /// Desugars to a sorted knot list.
    #[must_use]
    pub fn knots(&self) -> Vec<[f64; 2]> {
        match self {
            Self::Constant(v) => vec![[0.0, *v], [1.0, *v]],
            Self::Shorthand {
                ends,
                peak,
                peak_at,
            } => {
                vec![
                    [0.0, ends[0]],
                    [peak_at.clamp(0.01, 0.99), *peak],
                    [1.0, ends[1]],
                ]
            }
            Self::Explicit { knots } => knots.clone(),
        }
    }

    /// A curve that is the same value everywhere.
    #[must_use]
    pub const fn constant(value: f64) -> Self {
        Self::Constant(value)
    }
}

/// Hue over the ramp, as authored.
///
/// ```toml
/// hue = 264                                  # constant
/// hue = { base = 264, torsion = -14 }        # shifts by -14 degrees over the ramp
/// hue = { knots = [[0.0, 270.0], [1.0, 255.0]] }
/// ```
///
/// **Torsion is a design decision** — shadows cooler, highlights warmer, or
/// the reverse — and is what makes a ramp look authored rather than computed.
/// It is deliberately a separate field from `hue_correction`, which
/// compensates a known defect of the color space. Merging the two would make
/// "was this intentional?" unanswerable a year from now.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum HueSpec {
    /// One hue for the whole ramp.
    Constant(f64),
    /// A base hue with an optional linear shift across the ramp.
    Torsion {
        /// Hue at the middle of the ramp, in degrees.
        base: f64,
        /// Total shift in degrees from `t = 0` to `t = 1`.
        #[serde(default)]
        torsion: f64,
    },
    /// Explicit `(t, hue)` knots.
    Explicit {
        /// The knots, hue in degrees.
        knots: Vec<[f64; 2]>,
    },
}

impl HueSpec {
    /// Desugars to a sorted knot list of `(t, hue in degrees)`.
    ///
    /// Torsion is applied symmetrically about the middle of the ramp, so
    /// changing `torsion` rotates the ends without moving the family's
    /// nominal hue.
    #[must_use]
    pub fn knots(&self) -> Vec<[f64; 2]> {
        match self {
            Self::Constant(h) => vec![[0.0, *h], [1.0, *h]],
            Self::Torsion { base, torsion } => {
                vec![[0.0, base - torsion / 2.0], [1.0, base + torsion / 2.0]]
            }
            Self::Explicit { knots } => knots.clone(),
        }
    }

    /// The family's nominal hue, used for reporting and as the neutral tint
    /// default.
    #[must_use]
    pub fn base(&self) -> f64 {
        match self {
            Self::Constant(h) => *h,
            Self::Torsion { base, .. } => *base,
            Self::Explicit { knots } => knots.first().map_or(0.0, |k| k[1]),
        }
    }
}

#[cfg(test)]
// Comparisons here are against literal values the code returns verbatim.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn a_constant_curve_desugars_to_two_equal_knots() {
        let knots = CurveSpec::Constant(0.8).knots();
        assert_eq!(knots, vec![[0.0, 0.8], [1.0, 0.8]]);
    }

    #[test]
    fn the_shorthand_places_a_peak_between_the_ends() {
        let curve = CurveSpec::Shorthand {
            ends: [0.15, 0.35],
            peak: 0.92,
            peak_at: 0.55,
        };
        let knots = curve.knots();
        assert_eq!(knots.len(), 3);
        assert_eq!(knots[0], [0.0, 0.15]);
        assert_eq!(knots[1], [0.55, 0.92]);
        assert_eq!(knots[2], [1.0, 0.35]);
    }

    #[test]
    fn torsion_rotates_the_ends_without_moving_the_base() {
        let hue = HueSpec::Torsion {
            base: 264.0,
            torsion: -14.0,
        };
        let knots = hue.knots();
        assert!((knots[0][1] - 271.0).abs() < 1e-12, "start: {:?}", knots[0]);
        assert!((knots[1][1] - 257.0).abs() < 1e-12, "end: {:?}", knots[1]);

        // The midpoint of the ramp keeps the nominal hue.
        assert!((f64::midpoint(knots[0][1], knots[1][1]) - 264.0).abs() < 1e-12);
        assert!((hue.base() - 264.0).abs() < 1e-12);
    }

    #[test]
    fn zero_torsion_is_a_constant_hue() {
        let knots = HueSpec::Torsion {
            base: 100.0,
            torsion: 0.0,
        }
        .knots();
        assert!((knots[0][1] - knots[1][1]).abs() < 1e-12);
    }

    #[test]
    fn every_authoring_form_parses() {
        #[derive(Deserialize)]
        struct Holder {
            cr: CurveSpec,
            hue: HueSpec,
        }

        let cases = [
            "cr = 0.8\nhue = 264",
            "cr = { ends = [0.1, 0.3], peak = 0.9 }\nhue = { base = 264, torsion = -14 }",
            "cr = { knots = [[0.0, 0.1], [1.0, 0.4]] }\nhue = { knots = [[0.0, 270.0], [1.0, 255.0]] }",
        ];
        for case in cases {
            let parsed: Holder = toml::from_str(case).expect(case);
            assert!(parsed.cr.knots().len() >= 2);
            assert!(parsed.hue.knots().len() >= 2);
        }
    }

    #[test]
    fn the_peak_position_is_optional_and_clamped_inside_the_ramp() {
        #[derive(Deserialize)]
        struct Holder {
            cr: CurveSpec,
        }
        let parsed: Holder =
            toml::from_str("cr = { ends = [0.1, 0.3], peak = 0.9, peak_at = 5.0 }")
                .expect("parses");
        let knots = parsed.cr.knots();
        assert!(
            knots[1][0] < 1.0,
            "peak must stay inside the ramp: {knots:?}"
        );
    }
}
