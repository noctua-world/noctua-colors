//! Color vision deficiency simulation, after Brettel, Viénot & Mollon (1997).
//!
//! Roughly one man in twelve has some form of red-green deficiency. A palette
//! that encodes meaning in hue alone — success green against danger red — is
//! unreadable for them, and no amount of contrast fixes it. This module exists
//! so the compiler can *measure* that rather than hope.
//!
//! # The model
//!
//! A dichromat's perceptual space is a surface, not a volume: two half-planes
//! in LMS cone space hinged along the neutral axis. Brettel's algorithm
//! projects each stimulus onto whichever half-plane it belongs to, along the
//! axis of the missing cone.
//!
//! Brettel is used rather than the more common Viénot (1999) simplification
//! because Viénot collapses the two half-planes into one. That is a fine
//! approximation for protanopia and deuteranopia and a poor one for
//! tritanopia — which is precisely where this project needs the answer, since
//! tritan deficiency is what separates a blue-versus-yellow signalling scheme.
//!
//! # Provenance
//!
//! The parameters below are the Brettel 1997 projections pre-composed into
//! linear sRGB (`rgbFromLms . projection . lmsFromRgb`), as published in
//! the `DaltonLens` reference implementation `libDaltonLens`. Composing them
//! removes the separate LMS round trip and the separation test can be done
//! directly in RGB.
//!
//! Reference: Brettel, H., Viénot, F., & Mollon, J. D. (1997). "Computerized
//! simulation of color appearance for dichromats." JOSA A, 14(10), 2647-2655.
//! <https://doi.org/10.1364/josaa.14.002647>

use crate::diff::delta_e_ok;
use crate::gamut::Gamut;
use crate::matrix::{Mat3, mul_vec};
use crate::space::{LinearRgb, Oklab};

/// A form of dichromacy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cvd {
    /// Missing long-wavelength (red) cones. Roughly 1% of men.
    Protanopia,
    /// Missing medium-wavelength (green) cones. Roughly 1% of men, and the
    /// most common dichromacy.
    Deuteranopia,
    /// Missing short-wavelength (blue) cones. Rare, and affects all genders
    /// about equally.
    Tritanopia,
}

impl Cvd {
    /// The stable identifier used in the spec and in emitted reports.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Protanopia => "protanopia",
            Self::Deuteranopia => "deuteranopia",
            Self::Tritanopia => "tritanopia",
        }
    }

    /// Every deficiency, in a fixed order so reports are deterministic.
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::Protanopia, Self::Deuteranopia, Self::Tritanopia]
    }

    const fn params(self) -> &'static BrettelParams {
        match self {
            Self::Protanopia => &PROTAN,
            Self::Deuteranopia => &DEUTAN,
            Self::Tritanopia => &TRITAN,
        }
    }
}

/// One deficiency's projection onto the dichromatic surface.
struct BrettelParams {
    /// Projection for stimuli on the first half-plane.
    plane_1: Mat3,
    /// Projection for stimuli on the second half-plane.
    plane_2: Mat3,
    /// Normal of the plane separating the two, already expressed in linear
    /// RGB. A non-negative dot product selects `plane_1`.
    separation_normal: [f64; 3],
}

const PROTAN: BrettelParams = BrettelParams {
    plane_1: [
        [0.149_80, 1.195_48, -0.345_28],
        [0.107_64, 0.848_64, 0.043_72],
        [0.003_84, -0.005_40, 1.001_56],
    ],
    plane_2: [
        [0.145_70, 1.161_72, -0.307_42],
        [0.108_16, 0.852_91, 0.038_92],
        [0.003_86, -0.005_24, 1.001_39],
    ],
    separation_normal: [0.000_48, 0.003_93, -0.004_41],
};

const DEUTAN: BrettelParams = BrettelParams {
    plane_1: [
        [0.364_77, 0.863_81, -0.228_58],
        [0.262_94, 0.642_45, 0.094_62],
        [-0.020_06, 0.027_28, 0.992_78],
    ],
    plane_2: [
        [0.372_98, 0.881_66, -0.254_64],
        [0.259_54, 0.635_06, 0.105_40],
        [-0.019_80, 0.027_84, 0.991_96],
    ],
    separation_normal: [-0.002_81, -0.006_11, 0.008_92],
};

const TRITAN: BrettelParams = BrettelParams {
    plane_1: [
        [1.012_77, 0.135_48, -0.148_26],
        [-0.012_43, 0.868_12, 0.144_31],
        [0.075_89, 0.805_00, 0.119_11],
    ],
    plane_2: [
        [0.936_78, 0.189_79, -0.126_57],
        [0.061_54, 0.815_26, 0.123_20],
        [-0.375_62, 1.127_67, 0.247_96],
    ],
    separation_normal: [0.039_01, -0.027_88, -0.011_13],
};

/// Simulates how `color` appears to someone with `deficiency`.
///
/// `severity` runs from `0.0` (normal vision, an identity transform) to `1.0`
/// (full dichromacy) and is applied as a linear interpolation in linear RGB,
/// which is where the projection is linear and the interpolation therefore
/// meaningful.
///
/// The Brettel parameters are defined against sRGB, so the color is taken
/// through sRGB. Pass a color that is already inside sRGB — the sRGB-mapped
/// fallback for a wide-gamut token — since that is what governs whether the
/// distinction survives on a real screen.
///
/// # Why this clamps channels, when nothing else in this crate may
///
/// The dichromatic surface is not contained in the sRGB cube: project a
/// saturated blue for a protanope and the red channel comes back around
/// `-0.32`. Something has to bring that back before it can be shown or
/// measured, and here — uniquely — that something is a per-channel clamp.
///
/// Gamut mapping, the rule everywhere else, is *wrong* for this. It reduces
/// chroma at fixed lightness and hue, but a color with a strongly negative
/// channel has no meaningful lightness or hue to preserve: Oklab is defined
/// there only because the cube root accepts negatives, and the coordinates it
/// reports are numerology. Mapping from them turns a simulated dark blue into
/// a desaturated cyan.
///
/// Clamping is also what the published algorithm specifies — reference
/// implementations write 8-bit sRGB and clamp implicitly — so this is the
/// behavior the model was validated against.
///
/// The cost is that simulation is only *near*-idempotent: a clamped result no
/// longer lies exactly on the dichromatic surface, so a second application
/// moves it slightly. Colors that project inside the gamut are exactly
/// idempotent, and the tests check that case specifically.
#[must_use]
pub fn simulate(color: Oklab, deficiency: Cvd, severity: f64) -> Oklab {
    let severity = severity.clamp(0.0, 1.0);
    let original = Gamut::Srgb.oklab_to_linear(color);

    let params = deficiency.params();
    let v = [original.r, original.g, original.b];
    let n = params.separation_normal;
    let side = v[0] * n[0] + v[1] * n[1] + v[2] * n[2];

    let projected = mul_vec(
        if side >= 0.0 {
            params.plane_1
        } else {
            params.plane_2
        },
        v,
    );

    let blend = |sim: f64, orig: f64| sim.mul_add(severity, orig * (1.0 - severity));
    let simulated = LinearRgb {
        r: blend(projected[0], original.r).clamp(0.0, 1.0),
        g: blend(projected[1], original.g).clamp(0.0, 1.0),
        b: blend(projected[2], original.b).clamp(0.0, 1.0),
    };

    Gamut::Srgb.linear_to_oklab(simulated)
}

/// How far apart two colors remain, in Oklab units, once `deficiency` is
/// applied to both.
///
/// This is the number the quality gate reports. A margin, not a verdict:
/// knowing that success and danger sit 0.31 apart under deuteranopia is
/// actionable in a way that "pass" is not.
#[must_use]
pub fn separation(a: Oklab, b: Oklab, deficiency: Cvd, severity: f64) -> f64 {
    delta_e_ok(
        simulate(a, deficiency, severity),
        simulate(b, deficiency, severity),
    )
}

/// The worst-case separation between two colors across every deficiency at
/// full severity, paired with the deficiency responsible.
///
/// The weakest link is what decides whether a pair is safe, so this is the
/// number a gate should threshold on.
#[must_use]
pub fn worst_separation(a: Oklab, b: Oklab) -> (Cvd, f64) {
    let mut worst = (Cvd::Protanopia, f64::INFINITY);
    for deficiency in Cvd::all() {
        let d = separation(a, b, deficiency, 1.0);
        if d < worst.1 {
            worst = (deficiency, d);
        }
    }
    worst
}

#[cfg(test)]
// These assertions compare against literal sentinels the functions return
// verbatim (exactly 0.0, exactly 1.0). Exact comparison is the assertion.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::space::Oklch;

    /// A saturated color at the given hue, built rather than chosen: 80% of
    /// the chroma sRGB allows at a mid lightness. No hex value is written
    /// down anywhere in this module.
    fn at_hue(hue: f64) -> Oklab {
        let l = 0.6;
        Oklch {
            l,
            c: Gamut::Srgb.max_chroma(l, hue) * 0.8,
            h: hue,
        }
        .to_oklab()
    }

    /// A spread of hues around the wheel, for laws that should hold for all
    /// of them.
    fn sample_hues() -> [f64; 6] {
        [15.0, 70.0, 145.0, 200.0, 264.0, 320.0]
    }

    /// Every projection row must sum to 1, which is what makes the neutral
    /// axis a fixed point. A single mistyped digit breaks this.
    #[test]
    fn every_projection_preserves_the_neutral_axis() {
        for deficiency in Cvd::all() {
            let p = deficiency.params();
            for (plane, m) in [("1", p.plane_1), ("2", p.plane_2)] {
                for (i, row) in m.iter().enumerate() {
                    let sum: f64 = row.iter().sum();
                    assert!(
                        (sum - 1.0).abs() < 1e-4,
                        "{} plane {plane} row {i} sums to {sum}",
                        deficiency.id()
                    );
                }
            }
        }
    }

    #[test]
    fn grays_are_unchanged_by_every_deficiency() {
        for deficiency in Cvd::all() {
            for step in 0..=10 {
                let gray = Oklch {
                    l: f64::from(step) / 10.0,
                    c: 0.0,
                    h: 0.0,
                }
                .to_oklab();
                let simulated = simulate(gray, deficiency, 1.0);
                assert!(
                    delta_e_ok(gray, simulated) < 2e-3,
                    "{} shifted gray at l={}: {:?} -> {:?}",
                    deficiency.id(),
                    f64::from(step) / 10.0,
                    gray,
                    simulated
                );
            }
        }
    }

    /// Projection is idempotent by definition: a point already on the surface
    /// stays where it is.
    ///
    /// Scoped to projections that land inside sRGB. Saturated colors project
    /// outside it and are clamped back, and a clamped result is no longer on
    /// the surface — see `simulate` for why clamping is right anyway.
    #[test]
    fn simulating_twice_matches_simulating_once() {
        let mut checked = 0;
        for deficiency in Cvd::all() {
            for hue in sample_hues() {
                let once = simulate(at_hue(hue), deficiency, 1.0);

                // A channel resting on a limit is the fingerprint of a clamp.
                let linear = Gamut::Srgb.oklab_to_linear(once);
                if ![linear.r, linear.g, linear.b]
                    .iter()
                    .all(|v| (0.002..0.998).contains(v))
                {
                    continue;
                }

                checked += 1;
                let twice = simulate(once, deficiency, 1.0);
                assert!(
                    delta_e_ok(once, twice) < 5e-3,
                    "{} at hue {hue}: not idempotent ({once:?} vs {twice:?})",
                    deficiency.id()
                );
            }
        }
        assert!(
            checked >= 10,
            "only {checked} unclamped cases; sample is too weak"
        );
    }

    #[test]
    fn zero_severity_is_the_identity() {
        for deficiency in Cvd::all() {
            for hue in sample_hues() {
                let color = at_hue(hue);
                assert!(delta_e_ok(color, simulate(color, deficiency, 0.0)) < 1e-9);
            }
        }
    }

    /// The failure this module exists to catch: red and green that differ
    /// *only* in hue are all but identical to a red-green dichromat.
    ///
    /// Constructed rather than quoted, so the pair is exactly equiluminant and
    /// the collapse is attributable to hue alone.
    #[test]
    fn equiluminant_red_and_green_collapse_under_red_green_deficiency() {
        let red = Oklch {
            l: 0.55,
            c: 0.15,
            h: 25.0,
        }
        .to_oklab();
        let green = Oklch {
            l: 0.55,
            c: 0.15,
            h: 145.0,
        }
        .to_oklab();

        let normal = delta_e_ok(red, green);
        assert!(
            normal > 0.2,
            "premise: clearly distinct to normal vision ({normal})"
        );

        // For a deuteranope the pair all but vanishes: what is left is under
        // the 0.02 just-noticeable difference.
        let deutan = separation(red, green, Cvd::Deuteranopia, 1.0);
        assert!(deutan < 0.02, "deuteranopia left {deutan} of {normal}");

        // A protanope keeps more of it — around half — but not because the
        // hues survive. Protan luminous efficiency is not Oklab lightness:
        // long wavelengths look dimmer, so a pair that is equiluminant on
        // paper acquires a real brightness difference. Worth knowing, because
        // it means "equal lightness" is not by itself a safe assumption.
        let protan = separation(red, green, Cvd::Protanopia, 1.0);
        assert!(
            protan < normal * 0.6,
            "protanopia should still lose most of it: {protan} of {normal}"
        );
        assert!(
            protan > deutan * 4.0,
            "expected protanopia to retain much more than deuteranopia here"
        );
    }

    /// The mitigation, stated as a test: give the same two hues a lightness
    /// difference and the distinction survives, because lightness is
    /// orthogonal to the missing cone.
    ///
    /// This is the rule the palette's semantic pairs are built on.
    #[test]
    fn adding_a_lightness_difference_rescues_the_same_two_hues() {
        let red = Oklch {
            l: 0.45,
            c: 0.15,
            h: 25.0,
        }
        .to_oklab();
        let green = Oklch {
            l: 0.72,
            c: 0.15,
            h: 145.0,
        }
        .to_oklab();

        for deficiency in [Cvd::Protanopia, Cvd::Deuteranopia] {
            let simulated = separation(red, green, deficiency, 1.0);
            assert!(
                simulated > 0.2,
                "{}: lightness difference should have survived, got {simulated}",
                deficiency.id()
            );
        }
    }

    /// The corresponding success case: separating by lightness as well as hue
    /// survives every deficiency. This is the property the palette must have.
    #[test]
    fn pairs_separated_in_lightness_survive_every_deficiency() {
        let dark = Oklch {
            l: 0.35,
            c: 0.13,
            h: 25.0,
        }
        .to_oklab();
        let light = Oklch {
            l: 0.78,
            c: 0.13,
            h: 150.0,
        }
        .to_oklab();
        let (deficiency, margin) = worst_separation(dark, light);
        assert!(
            margin > 0.2,
            "worst case {} left only {margin} of separation",
            deficiency.id()
        );
    }

    #[test]
    fn severity_interpolates_monotonically() {
        let (red, green) = (at_hue(25.0), at_hue(145.0));
        let mut previous = f64::INFINITY;
        for step in 0..=10 {
            let d = separation(red, green, Cvd::Deuteranopia, f64::from(step) / 10.0);
            assert!(
                d <= previous + 1e-6,
                "not monotonic at severity {step}: {d} > {previous}"
            );
            previous = d;
        }
    }

    #[test]
    fn worst_separation_reports_the_actual_minimum() {
        let red = Oklch {
            l: 0.55,
            c: 0.15,
            h: 25.0,
        }
        .to_oklab();
        let green = Oklch {
            l: 0.55,
            c: 0.15,
            h: 145.0,
        }
        .to_oklab();
        let (worst_kind, worst_value) = worst_separation(red, green);
        for deficiency in Cvd::all() {
            assert!(separation(red, green, deficiency, 1.0) >= worst_value - 1e-12);
        }
        assert!(
            matches!(worst_kind, Cvd::Protanopia | Cvd::Deuteranopia),
            "red vs green should be worst for a red-green deficiency, got {}",
            worst_kind.id()
        );
    }
}
