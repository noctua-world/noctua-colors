//! Color space types and the conversions between them.
//!
//! The pipeline is `Oklch <-> Oklab <-> Xyz <-> LinearRgb <-> Rgb`. Oklab is
//! the working space for everything the compiler reasons about; the RGB spaces
//! are output encodings, never the source of truth.
//!
//! Routing through XYZ is what makes Display P3 and Rec.2020 fall out of the
//! same code as sRGB: a gamut is just a different pair of matrices on the
//! `Xyz <-> LinearRgb` edge.

use crate::matrix::{self, Mat3, Vec3};

/// CIE 1931 XYZ tristimulus values, normalized so that the D65 white point has
/// `y == 1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xyz {
    /// X tristimulus value.
    pub x: f64,
    /// Y tristimulus value, equal to relative luminance.
    pub y: f64,
    /// Z tristimulus value.
    pub z: f64,
}

/// A color in Oklab: perceptual lightness plus two opponent axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklab {
    /// Perceptual lightness, nominally `0.0` (black) to `1.0` (white).
    pub l: f64,
    /// Green-to-red opponent axis.
    pub a: f64,
    /// Blue-to-yellow opponent axis.
    pub b: f64,
}

/// A color in OKLCH: the cylindrical form of [`Oklab`].
///
/// This is the authoring space. Hue is in degrees, canonically in `[0, 360)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklch {
    /// Perceptual lightness, nominally `0.0` (black) to `1.0` (white).
    pub l: f64,
    /// Absolute chroma. Note that the spec format stores *relative* chroma;
    /// see `noctua-engine` for the conversion.
    pub c: f64,
    /// Hue angle in degrees.
    pub h: f64,
}

/// Linear-light RGB, relative to some [`Gamut`](crate::Gamut).
///
/// A color is inside its gamut exactly when all three components lie in
/// `[0, 1]`. Values outside that range are meaningful and are what the gamut
/// mapper exists to resolve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearRgb {
    /// Linear red component.
    pub r: f64,
    /// Linear green component.
    pub g: f64,
    /// Linear blue component.
    pub b: f64,
}

/// Transfer-encoded RGB in `[0, 1]`, ready to be written as hex or a CSS
/// `color()` function.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    /// Encoded red component.
    pub r: f64,
    /// Encoded green component.
    pub g: f64,
    /// Encoded blue component.
    pub b: f64,
}

/// CIE standard illuminant D65, as `(x, y)` chromaticity.
///
/// Shared by the Oklab white-point conditioning below and by every gamut this
/// crate supports.
pub const D65_XY: (f64, f64) = (0.3127, 0.3290);

/// D65 as XYZ, normalized to unit luminance.
pub const D65_XYZ: [f64; 3] = [
    D65_XY.0 / D65_XY.1,
    1.0,
    (1.0 - D65_XY.0 - D65_XY.1) / D65_XY.1,
];

// --- Oklab matrices -------------------------------------------------------
//
// Björn Ottosson's Oklab, defined as XYZ(D65) -> LMS -> cube root -> Lab.
// Only the two forward matrices are written down; the inverses are derived,
// so a round trip is exact by construction rather than by transcription luck.

/// XYZ (D65) to the Oklab cone-response space, as published.
const XYZ_TO_LMS_PUBLISHED: Mat3 = [
    [0.818_933_010_1, 0.361_866_742_4, -0.128_859_713_7],
    [0.032_984_543_6, 0.929_311_871_5, 0.036_145_638_7],
    [0.048_200_301_8, 0.264_366_269_1, 0.633_851_707_0],
];

/// Nonlinear (cube-rooted) cone responses to Oklab coordinates, as published.
const LMS_TO_OKLAB_PUBLISHED: Mat3 = [
    [0.210_454_255_3, 0.793_617_785_0, -0.004_072_046_8],
    [1.977_998_495_1, -2.428_592_205_0, 0.450_593_709_9],
    [0.025_904_037_1, 0.782_771_766_2, -0.808_675_766_0],
];

/// Rescales each row of `m` so that `white` maps to unit response.
///
/// The published matrices carry a rounding residual: fed D65, they return a
/// lightness of `0.999_998_8` rather than 1. That is invisible to the eye and
/// fatal to this compiler, which anchors scale steps on lightness and needs
/// `l = 1.0` to mean exactly white and nothing else. Conditioning the matrix
/// against its own reference white is the standard fix and costs nothing at
/// runtime.
const fn condition_white(m: Mat3, white: [f64; 3]) -> Mat3 {
    let w = matrix::mul_vec(m, white);
    [
        [m[0][0] / w[0], m[0][1] / w[0], m[0][2] / w[0]],
        [m[1][0] / w[1], m[1][1] / w[1], m[1][2] / w[1]],
        [m[2][0] / w[2], m[2][1] / w[2], m[2][2] / w[2]],
    ]
}

/// Conditions the Lab matrix so the achromatic axis is exact.
///
/// Given unit cone response, lightness is the sum of row 0 and the two
/// opponent axes are the sums of rows 1 and 2. Forcing those sums to exactly
/// 1, 0 and 0 makes "white is `l = 1`" and "gray has zero chroma" true by
/// construction instead of true to eight decimal places. The corrections are
/// on the order of 1e-8 — far below any perceptual or numerical significance,
/// and far above the cost of an invariant that silently does not hold.
const fn condition_achromatic_axis(m: Mat3) -> Mat3 {
    let scale = 1.0 / (m[0][0] + m[0][1] + m[0][2]);
    let shift_a = (m[1][0] + m[1][1] + m[1][2]) / 3.0;
    let shift_b = (m[2][0] + m[2][1] + m[2][2]) / 3.0;
    [
        [m[0][0] * scale, m[0][1] * scale, m[0][2] * scale],
        [m[1][0] - shift_a, m[1][1] - shift_a, m[1][2] - shift_a],
        [m[2][0] - shift_b, m[2][1] - shift_b, m[2][2] - shift_b],
    ]
}

const XYZ_TO_LMS: Mat3 = condition_white(XYZ_TO_LMS_PUBLISHED, D65_XYZ);
const LMS_TO_OKLAB: Mat3 = condition_achromatic_axis(LMS_TO_OKLAB_PUBLISHED);

const LMS_TO_XYZ: Mat3 = matrix::inverse(XYZ_TO_LMS);
const OKLAB_TO_LMS: Mat3 = matrix::inverse(LMS_TO_OKLAB);

/// Oklab coordinates to nonlinear cone responses.
///
/// Exposed for the analytic gamut-boundary solver, which needs the transform
/// as coefficients rather than as a conversion.
pub(crate) const fn oklab_to_lms_matrix() -> Mat3 {
    OKLAB_TO_LMS
}

/// Cube-rooted cone responses to CIE XYZ (D65). Companion to
/// [`oklab_to_lms_matrix`].
pub(crate) const fn lms_to_xyz_matrix() -> Mat3 {
    LMS_TO_XYZ
}

impl Oklab {
    /// Converts from CIE XYZ (D65).
    #[must_use]
    pub fn from_xyz(xyz: Xyz) -> Self {
        let lms = matrix::mul_vec(XYZ_TO_LMS, [xyz.x, xyz.y, xyz.z]);
        let nonlinear = [lms[0].cbrt(), lms[1].cbrt(), lms[2].cbrt()];
        let lab = matrix::mul_vec(LMS_TO_OKLAB, nonlinear);
        Self {
            l: lab[0],
            a: lab[1],
            b: lab[2],
        }
    }

    /// Converts to CIE XYZ (D65).
    #[must_use]
    pub fn to_xyz(self) -> Xyz {
        let nonlinear = matrix::mul_vec(OKLAB_TO_LMS, [self.l, self.a, self.b]);
        let lms: Vec3 = [
            nonlinear[0] * nonlinear[0] * nonlinear[0],
            nonlinear[1] * nonlinear[1] * nonlinear[1],
            nonlinear[2] * nonlinear[2] * nonlinear[2],
        ];
        let xyz = matrix::mul_vec(LMS_TO_XYZ, lms);
        Xyz {
            x: xyz[0],
            y: xyz[1],
            z: xyz[2],
        }
    }

    /// Converts to the cylindrical [`Oklch`] form.
    #[must_use]
    pub fn to_oklch(self) -> Oklch {
        let c = self.a.hypot(self.b);
        // Hue is undefined for achromatic colors. Reporting 0 rather than a
        // NaN keeps downstream arithmetic total; callers that care use
        // `is_achromatic`.
        let h = if c < ACHROMATIC_CHROMA {
            0.0
        } else {
            normalize_hue(self.b.atan2(self.a).to_degrees())
        };
        Oklch { l: self.l, c, h }
    }
}

/// Chroma below which a color is treated as achromatic and its hue as
/// undefined.
///
/// Well under the ~0.0005 chroma of any color distinguishable from gray, so
/// this only ever catches true neutrals and floating-point noise around them.
pub const ACHROMATIC_CHROMA: f64 = 1e-7;

impl Oklch {
    /// Converts to the rectangular [`Oklab`] form.
    #[must_use]
    pub fn to_oklab(self) -> Oklab {
        let rad = self.h.to_radians();
        Oklab {
            l: self.l,
            a: self.c * rad.cos(),
            b: self.c * rad.sin(),
        }
    }

    /// Returns `true` when chroma is low enough that hue carries no meaning.
    #[must_use]
    pub fn is_achromatic(self) -> bool {
        self.c < ACHROMATIC_CHROMA
    }

    /// Returns a copy with the hue wrapped into `[0, 360)`.
    #[must_use]
    pub fn normalized(self) -> Self {
        Self {
            h: normalize_hue(self.h),
            ..self
        }
    }
}

/// Wraps a hue angle in degrees into `[0, 360)`.
#[must_use]
pub fn normalize_hue(degrees: f64) -> f64 {
    let wrapped = degrees % 360.0;
    if wrapped < 0.0 {
        wrapped + 360.0
    } else {
        wrapped
    }
}

/// Returns the signed shortest angular distance from `from` to `to`, in
/// degrees, in `(-180, 180]`.
///
/// Used wherever hue differences are compared against a tolerance: a naive
/// subtraction reports 359 degrees for what is really a 1 degree shift across
/// the wrap point.
#[must_use]
pub fn hue_difference(from: f64, to: f64) -> f64 {
    let d = (normalize_hue(to) - normalize_hue(from) + 540.0) % 360.0 - 180.0;
    // Exact antipodes land on -180; report the positive representative so the
    // result stays in (-180, 180].
    if d <= -180.0 { d + 360.0 } else { d }
}

impl Xyz {
    /// Relative luminance, identical to the `y` component.
    #[must_use]
    pub fn luminance(self) -> f64 {
        self.y
    }
}

/// A uniform color appearance space usable as the compiler's working space.
///
/// Only [`Oklab`] implements this. The trait exists so that a second space
/// could be introduced later if measurements ever justify it, without the
/// engine having to name a concrete type.
///
/// Deliberately *not* implemented: CAM16-UCS. It needs viewing-condition
/// parameters this compiler does not have, and its advantage over Oklab for
/// UI work is marginal. See `AGENTS.md`.
pub trait UniformSpace: Copy {
    /// Converts from CIE XYZ (D65).
    fn from_xyz(xyz: Xyz) -> Self;

    /// Converts to CIE XYZ (D65).
    fn to_xyz(self) -> Xyz;

    /// Perceptual difference between two colors in this space.
    fn difference(a: Self, b: Self) -> f64;
}

impl UniformSpace for Oklab {
    fn from_xyz(xyz: Xyz) -> Self {
        Self::from_xyz(xyz)
    }

    fn to_xyz(self) -> Xyz {
        self.to_xyz()
    }

    fn difference(a: Self, b: Self) -> f64 {
        crate::diff::delta_e_ok(a, b)
    }
}

#[cfg(test)]
// These assertions compare against literal sentinels the functions return
// verbatim (exactly 0.0, exactly 1.0). Exact comparison is the assertion.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn oklab_round_trips_through_xyz() {
        let samples = [
            Oklab {
                l: 0.0,
                a: 0.0,
                b: 0.0,
            },
            Oklab {
                l: 1.0,
                a: 0.0,
                b: 0.0,
            },
            Oklab {
                l: 0.5,
                a: 0.1,
                b: -0.15,
            },
            Oklab {
                l: 0.62,
                a: -0.08,
                b: 0.12,
            },
        ];
        for c in samples {
            let back = Oklab::from_xyz(c.to_xyz());
            assert!((back.l - c.l).abs() < 1e-12, "l: {back:?} vs {c:?}");
            assert!((back.a - c.a).abs() < 1e-12, "a: {back:?} vs {c:?}");
            assert!((back.b - c.b).abs() < 1e-12, "b: {back:?} vs {c:?}");
        }
    }

    #[test]
    fn oklch_round_trips_through_oklab() {
        let c = Oklch {
            l: 0.7,
            c: 0.13,
            h: 265.0,
        };
        let back = c.to_oklab().to_oklch();
        assert!((back.l - c.l).abs() < 1e-12);
        assert!((back.c - c.c).abs() < 1e-12);
        assert!((back.h - c.h).abs() < 1e-10);
    }

    #[test]
    fn achromatic_colors_report_zero_hue_rather_than_nan() {
        let gray = Oklab {
            l: 0.5,
            a: 0.0,
            b: 0.0,
        }
        .to_oklch();
        assert!(gray.is_achromatic());
        assert!(gray.h.is_finite());
        assert_eq!(gray.h, 0.0);
    }

    #[test]
    fn the_d65_white_point_is_exactly_lightness_one() {
        let white = Xyz {
            x: D65_XYZ[0],
            y: D65_XYZ[1],
            z: D65_XYZ[2],
        };
        let lab = Oklab::from_xyz(white);
        assert!((lab.l - 1.0).abs() < 1e-14, "l = {}", lab.l);
        assert!(
            lab.a.abs() < 1e-14 && lab.b.abs() < 1e-14,
            "not neutral: {lab:?}"
        );
    }

    #[test]
    fn every_gray_has_exactly_zero_chroma() {
        // Anything on the D65 achromatic axis must land on a == b == 0, or
        // neutral ramps acquire a phantom hue that the tint machinery would
        // then fight.
        for step in 0..=20 {
            let k = f64::from(step) / 20.0;
            let gray = Xyz {
                x: D65_XYZ[0] * k,
                y: k,
                z: D65_XYZ[2] * k,
            };
            let lab = Oklab::from_xyz(gray);
            assert!(lab.a.abs() < 1e-14 && lab.b.abs() < 1e-14, "k={k}: {lab:?}");
            assert!(lab.to_oklch().is_achromatic(), "k={k} not achromatic");
        }
    }

    #[test]
    fn normalize_hue_wraps_both_directions() {
        assert!((normalize_hue(370.0) - 10.0).abs() < 1e-12);
        assert!((normalize_hue(-10.0) - 350.0).abs() < 1e-12);
        assert!((normalize_hue(0.0) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn hue_difference_takes_the_short_way_around() {
        assert!((hue_difference(359.0, 1.0) - 2.0).abs() < 1e-12);
        assert!((hue_difference(1.0, 359.0) - -2.0).abs() < 1e-12);
        assert!((hue_difference(10.0, 20.0) - 10.0).abs() < 1e-12);
    }
}
