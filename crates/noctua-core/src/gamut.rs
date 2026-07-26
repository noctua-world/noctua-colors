//! Output gamuts: their primaries, transfer functions, and chroma boundary.

use crate::matrix::{self, Mat3};
use crate::space::{LinearRgb, Oklab, Rgb, Xyz};

/// CIE xy chromaticity coordinates of a gamut's primaries and white point.
///
/// These eight numbers are the *only* per-gamut constants written down in this
/// crate. Every matrix is derived from them.
#[derive(Debug, Clone, Copy)]
struct Primaries {
    red: (f64, f64),
    green: (f64, f64),
    blue: (f64, f64),
    white: (f64, f64),
}

/// CIE standard illuminant D65, as used by all three supported gamuts.
const D65: (f64, f64) = crate::space::D65_XY;

/// sRGB / Rec.709 primaries (IEC 61966-2-1).
const SRGB_PRIMARIES: Primaries = Primaries {
    red: (0.640, 0.330),
    green: (0.300, 0.600),
    blue: (0.150, 0.060),
    white: D65,
};

/// Display P3 primaries (SMPTE DCI-P3 gamut on a D65 white point).
const DISPLAY_P3_PRIMARIES: Primaries = Primaries {
    red: (0.680, 0.320),
    green: (0.265, 0.690),
    blue: (0.150, 0.060),
    white: D65,
};

/// Rec.2020 primaries (ITU-R BT.2020).
const REC2020_PRIMARIES: Primaries = Primaries {
    red: (0.708, 0.292),
    green: (0.170, 0.797),
    blue: (0.131, 0.046),
    white: D65,
};

/// Converts an xy chromaticity to XYZ at unit luminance.
const fn xy_to_xyz(xy: (f64, f64)) -> [f64; 3] {
    let (x, y) = xy;
    [x / y, 1.0, (1.0 - x - y) / y]
}

/// Builds the linear-RGB-to-XYZ matrix for a set of primaries.
///
/// This is the standard construction: place the three primaries as columns at
/// unit luminance, solve for the scale factors that map RGB white to the
/// declared white point, then scale the columns.
const fn rgb_to_xyz_matrix(p: Primaries) -> Mat3 {
    let r = xy_to_xyz(p.red);
    let g = xy_to_xyz(p.green);
    let b = xy_to_xyz(p.blue);

    let unscaled: Mat3 = [[r[0], g[0], b[0]], [r[1], g[1], b[1]], [r[2], g[2], b[2]]];
    let white = xy_to_xyz(p.white);
    let s = matrix::mul_vec(matrix::inverse(unscaled), white);

    [
        [s[0] * r[0], s[1] * g[0], s[2] * b[0]],
        [s[0] * r[1], s[1] * g[1], s[2] * b[1]],
        [s[0] * r[2], s[1] * g[2], s[2] * b[2]],
    ]
}

const SRGB_TO_XYZ: Mat3 = rgb_to_xyz_matrix(SRGB_PRIMARIES);
const XYZ_TO_SRGB: Mat3 = matrix::inverse(SRGB_TO_XYZ);
const P3_TO_XYZ: Mat3 = rgb_to_xyz_matrix(DISPLAY_P3_PRIMARIES);
const XYZ_TO_P3: Mat3 = matrix::inverse(P3_TO_XYZ);
const REC2020_TO_XYZ: Mat3 = rgb_to_xyz_matrix(REC2020_PRIMARIES);
const XYZ_TO_REC2020: Mat3 = matrix::inverse(REC2020_TO_XYZ);

// Cone responses straight to linear RGB, skipping the XYZ hop. Used by the
// analytic boundary solver, which needs each channel as a polynomial in
// chroma.
const LMS_TO_SRGB: Mat3 = matrix::mul(XYZ_TO_SRGB, crate::space::lms_to_xyz_matrix());
const LMS_TO_P3: Mat3 = matrix::mul(XYZ_TO_P3, crate::space::lms_to_xyz_matrix());
const LMS_TO_REC2020: Mat3 = matrix::mul(XYZ_TO_REC2020, crate::space::lms_to_xyz_matrix());

/// An output gamut.
///
/// The compiler is gamut-generic: relative chroma resolves against whichever
/// gamut is being emitted, which is what lets one token definition render
/// correctly on sRGB and more vividly on Display P3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Gamut {
    /// sRGB / Rec.709. The safe default and the hex fallback layer.
    Srgb,
    /// Display P3. Roughly 25% wider than sRGB, standard on modern displays.
    DisplayP3,
    /// Rec.2020. Wider still; few displays cover it fully.
    Rec2020,
}

impl Gamut {
    /// The stable identifier used in the spec and in emitted file names.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Srgb => "srgb",
            Self::DisplayP3 => "display-p3",
            Self::Rec2020 => "rec2020",
        }
    }

    /// Every gamut, in widening order. Iteration order is fixed so that
    /// emitted output is deterministic.
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::Srgb, Self::DisplayP3, Self::Rec2020]
    }

    /// Parses the identifier produced by [`Gamut::id`].
    ///
    /// Kept here rather than behind a `serde` implementation so that this
    /// crate stays free of the spec format's dependencies.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::all().into_iter().find(|g| g.id() == id)
    }

    const fn to_xyz_matrix(self) -> Mat3 {
        match self {
            Self::Srgb => SRGB_TO_XYZ,
            Self::DisplayP3 => P3_TO_XYZ,
            Self::Rec2020 => REC2020_TO_XYZ,
        }
    }

    /// Linear-light RGB from Oklab cone responses, for this gamut.
    const fn lms_to_rgb_matrix(self) -> Mat3 {
        match self {
            Self::Srgb => LMS_TO_SRGB,
            Self::DisplayP3 => LMS_TO_P3,
            Self::Rec2020 => LMS_TO_REC2020,
        }
    }

    const fn xyz_to_rgb_matrix(self) -> Mat3 {
        match self {
            Self::Srgb => XYZ_TO_SRGB,
            Self::DisplayP3 => XYZ_TO_P3,
            Self::Rec2020 => XYZ_TO_REC2020,
        }
    }

    /// Converts linear-light RGB in this gamut to CIE XYZ (D65).
    #[must_use]
    pub fn linear_to_xyz(self, rgb: LinearRgb) -> Xyz {
        let v = matrix::mul_vec(self.to_xyz_matrix(), [rgb.r, rgb.g, rgb.b]);
        Xyz {
            x: v[0],
            y: v[1],
            z: v[2],
        }
    }

    /// Converts CIE XYZ (D65) to linear-light RGB in this gamut.
    ///
    /// Components outside `[0, 1]` mean the color is outside the gamut; that
    /// is information, not an error, and is exactly what the boundary solver
    /// and the gamut mapper consume.
    #[must_use]
    pub fn xyz_to_linear(self, xyz: Xyz) -> LinearRgb {
        let v = matrix::mul_vec(self.xyz_to_rgb_matrix(), [xyz.x, xyz.y, xyz.z]);
        LinearRgb {
            r: v[0],
            g: v[1],
            b: v[2],
        }
    }

    /// Applies this gamut's transfer function, producing encoded RGB.
    #[must_use]
    pub fn encode(self, linear: LinearRgb) -> Rgb {
        let f = match self {
            // Display P3 shares the sRGB transfer function; only the
            // primaries differ.
            Self::Srgb | Self::DisplayP3 => srgb_encode,
            Self::Rec2020 => rec2020_encode,
        };
        Rgb {
            r: f(linear.r),
            g: f(linear.g),
            b: f(linear.b),
        }
    }

    /// Removes this gamut's transfer function, producing linear-light RGB.
    #[must_use]
    pub fn decode(self, rgb: Rgb) -> LinearRgb {
        let f = match self {
            Self::Srgb | Self::DisplayP3 => srgb_decode,
            Self::Rec2020 => rec2020_decode,
        };
        LinearRgb {
            r: f(rgb.r),
            g: f(rgb.g),
            b: f(rgb.b),
        }
    }

    /// Converts an Oklab color directly to linear-light RGB in this gamut.
    #[must_use]
    pub fn oklab_to_linear(self, lab: Oklab) -> LinearRgb {
        self.xyz_to_linear(lab.to_xyz())
    }

    /// Converts linear-light RGB in this gamut directly to Oklab.
    #[must_use]
    pub fn linear_to_oklab(self, rgb: LinearRgb) -> Oklab {
        Oklab::from_xyz(self.linear_to_xyz(rgb))
    }

    /// Returns `true` when every linear component lies within `[0, 1]`, with a
    /// small tolerance for floating-point error.
    #[must_use]
    pub fn contains(self, lab: Oklab) -> bool {
        let rgb = self.oklab_to_linear(lab);
        in_unit_range(rgb.r) && in_unit_range(rgb.g) && in_unit_range(rgb.b)
    }

    /// The largest chroma that stays inside this gamut at the given lightness
    /// and hue.
    ///
    /// Precisely: the largest `C` such that **every** chroma in `[0, C]` is in
    /// gamut. That distinction is not pedantic — see below.
    ///
    /// Returns `0.0` at lightness outside `(0, 1)`, where the gamut has
    /// collapsed to a point.
    ///
    /// # Why this is solved rather than searched
    ///
    /// The obvious implementation bisects on chroma, and it is wrong.
    /// Bisection assumes the in-gamut set along a ray is an interval, and in
    /// Oklab it is not: the cube-root nonlinearity makes the gamut slightly
    /// non-convex, and near the sRGB blue primary the red channel crosses zero
    /// **three times** along a single ray. A ray at hue 264.1 leaves the gamut
    /// at chroma 0.270, re-enters at 0.311, and leaves again at 0.313. A
    /// bisection converges to whichever crossing its midpoints happen to
    /// bracket, so maximum chroma jumped by 19% between hue 264.0 and 264.1 —
    /// a visible kink in any ramp built on relative chroma.
    ///
    /// Solving removes the ambiguity. At fixed lightness and hue, each cone
    /// response is *affine* in chroma, so each linear RGB channel is an exact
    /// **cubic** in chroma. The boundary is then the smallest positive root
    /// among six cubics — the three channels against 0 and against 1 — which
    /// is by construction the *first* crossing, the one relative chroma needs.
    /// It is also exact, continuous in hue, and faster than the bisection it
    /// replaces.
    #[must_use]
    pub fn max_chroma(self, lightness: f64, hue: f64) -> f64 {
        if lightness <= 0.0 || lightness >= 1.0 {
            return 0.0;
        }

        let (sin, cos) = hue.to_radians().sin_cos();
        let to_lms = crate::space::oklab_to_lms_matrix();
        let lms_to_rgb = self.lms_to_rgb_matrix();

        // Each cone response is affine in chroma: lms'[i] = base[i] + rate[i] * C.
        let mut base = [0.0; 3];
        let mut rate = [0.0; 3];
        for i in 0..3 {
            base[i] = to_lms[i][0] * lightness;
            rate[i] = to_lms[i][1] * cos + to_lms[i][2] * sin;
        }

        let mut limit = MAX_REPRESENTABLE_CHROMA;
        for row in lms_to_rgb {
            // channel(C) = sum_i row[i] * (base[i] + rate[i] * C)^3
            let mut poly = [0.0; 4];
            for i in 0..3 {
                let (a, b) = (base[i], rate[i]);
                poly[0] += row[i] * b * b * b;
                poly[1] += row[i] * 3.0 * a * b * b;
                poly[2] += row[i] * 3.0 * a * a * b;
                poly[3] += row[i] * a * a * a;
            }

            // Crossing zero, and crossing one.
            for target in [0.0, 1.0] {
                let mut shifted = poly;
                shifted[3] -= target;
                if let Some(root) = crate::cubic::smallest_root_above(shifted, 0.0) {
                    limit = limit.min(root);
                }
            }
        }

        limit.max(0.0)
    }

    /// The same boundary found by bisection.
    ///
    /// Kept only so the tests can cross-check [`Gamut::max_chroma`] against an
    /// independent method. Not for production use: see that function's note on
    /// why bisection is unreliable near a gamut's primaries.
    #[cfg(test)]
    fn max_chroma_by_bisection(self, lightness: f64, hue: f64) -> f64 {
        if lightness <= 0.0 || lightness >= 1.0 {
            return 0.0;
        }
        let at = |c: f64| {
            crate::space::Oklch {
                l: lightness,
                c,
                h: hue,
            }
            .to_oklab()
        };
        let mut high = MAX_REPRESENTABLE_CHROMA;
        if self.contains(at(high)) {
            return high;
        }
        let mut low = 0.0;
        for _ in 0..CHROMA_BISECTION_STEPS {
            let mid = f64::midpoint(low, high);
            if self.contains(at(mid)) {
                low = mid;
            } else {
                high = mid;
            }
        }
        low
    }
}

/// Tolerance on linear RGB components when testing gamut membership.
///
/// Sized to absorb the accumulated error of the XYZ round trip (which the
/// round-trip tests measure at well under `1e-12`) without admitting any color
/// a display could not actually show.
pub const GAMUT_EPSILON: f64 = 1e-9;

/// Iterations used by the reference chroma bisection.
#[cfg(test)]
///
/// 40 halvings of the `[0, 0.5]` starting interval resolve chroma to about
/// `5e-13`, far below the `1e-4` at which emitted values are quantized.
const CHROMA_BISECTION_STEPS: u32 = 40;

/// An upper bound on chroma for any color inside Rec.2020, and therefore for
/// any gamut this crate supports.
///
/// Rec.2020's most saturated colors reach roughly 0.32 chroma in Oklab; 0.5
/// leaves generous headroom while keeping the bisection interval tight.
pub const MAX_REPRESENTABLE_CHROMA: f64 = 0.5;

fn in_unit_range(v: f64) -> bool {
    (-GAMUT_EPSILON..=1.0 + GAMUT_EPSILON).contains(&v)
}

// --- Transfer functions ---------------------------------------------------

/// The sRGB transfer function, shared by sRGB and Display P3.
fn srgb_encode(c: f64) -> f64 {
    let sign = c.signum();
    let a = c.abs();
    sign * if a <= 0.003_130_8 {
        12.92 * a
    } else {
        1.055 * a.powf(1.0 / 2.4) - 0.055
    }
}

fn srgb_decode(c: f64) -> f64 {
    let sign = c.signum();
    let a = c.abs();
    sign * if a <= 0.040_449_936 {
        a / 12.92
    } else {
        ((a + 0.055) / 1.055).powf(2.4)
    }
}

/// Rec.2020 uses its own OETF with different constants and a 0.45 exponent.
const REC2020_ALPHA: f64 = 1.099_296_826_809_44;
const REC2020_BETA: f64 = 0.018_053_968_510_807;

fn rec2020_encode(c: f64) -> f64 {
    let sign = c.signum();
    let a = c.abs();
    sign * if a < REC2020_BETA {
        4.5 * a
    } else {
        REC2020_ALPHA * a.powf(0.45) - (REC2020_ALPHA - 1.0)
    }
}

fn rec2020_decode(c: f64) -> f64 {
    let sign = c.signum();
    let a = c.abs();
    sign * if a < 4.5 * REC2020_BETA {
        a / 4.5
    } else {
        ((a + (REC2020_ALPHA - 1.0)) / REC2020_ALPHA).powf(1.0 / 0.45)
    }
}

#[cfg(test)]
// These assertions compare against literal sentinels the functions return
// verbatim (exactly 0.0, exactly 1.0). Exact comparison is the assertion.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::space::Oklch;

    /// The derived matrices must reproduce the values published in the specs.
    /// This is the test that would catch a wrong chromaticity coordinate.
    #[test]
    fn srgb_matrix_matches_published_values() {
        // IEC 61966-2-1 / CSS Color 4, to 4 decimal places.
        let expected: Mat3 = [
            [0.4124, 0.3576, 0.1805],
            [0.2126, 0.7152, 0.0722],
            [0.0193, 0.1192, 0.9505],
        ];
        for r in 0..3 {
            for c in 0..3 {
                assert!(
                    (SRGB_TO_XYZ[r][c] - expected[r][c]).abs() < 5e-5,
                    "SRGB_TO_XYZ[{r}][{c}] = {}, expected ~{}",
                    SRGB_TO_XYZ[r][c],
                    expected[r][c]
                );
            }
        }
    }

    #[test]
    fn every_gamut_maps_rgb_white_to_the_d65_white_point() {
        let white = LinearRgb {
            r: 1.0,
            g: 1.0,
            b: 1.0,
        };
        let expected = xy_to_xyz(D65);
        for gamut in Gamut::all() {
            let xyz = gamut.linear_to_xyz(white);
            assert!(
                (xyz.x - expected[0]).abs() < 1e-12
                    && (xyz.y - 1.0).abs() < 1e-12
                    && (xyz.z - expected[2]).abs() < 1e-12,
                "{}: {xyz:?}",
                gamut.id()
            );
        }
    }

    #[test]
    fn rgb_white_is_oklab_lightness_one() {
        for gamut in Gamut::all() {
            let lab = gamut.linear_to_oklab(LinearRgb {
                r: 1.0,
                g: 1.0,
                b: 1.0,
            });
            assert!((lab.l - 1.0).abs() < 1e-6, "{}: l = {}", gamut.id(), lab.l);
            assert!(
                lab.a.abs() < 1e-6 && lab.b.abs() < 1e-6,
                "{}: not neutral",
                gamut.id()
            );
        }
    }

    #[test]
    fn transfer_functions_round_trip() {
        for gamut in Gamut::all() {
            for step in 0..=100 {
                let v = f64::from(step) / 100.0;
                let linear = LinearRgb { r: v, g: v, b: v };
                let back = gamut.decode(gamut.encode(linear));
                assert!(
                    (back.r - v).abs() < 1e-12,
                    "{} at {v}: got {}",
                    gamut.id(),
                    back.r
                );
            }
        }
    }

    #[test]
    fn srgb_is_contained_by_both_wider_gamuts() {
        for hue in (0..360).step_by(5) {
            for l_step in 1..20 {
                let (hue, l) = (f64::from(hue), f64::from(l_step) / 20.0);
                let srgb_max = Gamut::Srgb.max_chroma(l, hue);
                for wider in [Gamut::DisplayP3, Gamut::Rec2020] {
                    let wider_max = wider.max_chroma(l, hue);
                    assert!(
                        wider_max >= srgb_max,
                        "{} at l={l} h={hue}: {wider_max} < srgb {srgb_max}",
                        wider.id()
                    );
                }
            }
        }
    }

    /// Display P3 is **not** a subset of Rec.2020, despite being the narrower
    /// gamut nearly everywhere.
    ///
    /// P3's red primary sits at xy (0.680, 0.320); the Rec.2020 red-to-green
    /// edge passes through y = 0.31828 at that x. A thin sliver of saturated
    /// orange-red is therefore reachable in P3 and not in Rec.2020.
    ///
    /// This test exists to stop a future reader from "fixing" the apparent
    /// anomaly. If it ever starts failing, a primary was mistyped.
    #[test]
    fn display_p3_is_not_a_subset_of_rec2020_near_red() {
        let mut found_overshoot = false;
        for hue in (20..45).step_by(1) {
            for l_step in 1..20 {
                let (hue, l) = (f64::from(hue), f64::from(l_step) / 20.0);
                if Gamut::DisplayP3.max_chroma(l, hue) > Gamut::Rec2020.max_chroma(l, hue) {
                    found_overshoot = true;
                }
            }
        }
        assert!(
            found_overshoot,
            "expected P3 to exceed Rec.2020 somewhere in the orange-red band"
        );
    }

    /// The contract of [`Gamut::max_chroma`]: *everything* below the reported
    /// chroma is in gamut, and the boundary is genuinely nearby.
    ///
    /// Stated this way rather than as "C is in and C + epsilon is out",
    /// because the interesting failure is an *over*estimate — a reported
    /// boundary with out-of-gamut colors beneath it would silently break
    /// relative chroma for every value less than one.
    #[test]
    fn everything_below_max_chroma_is_in_gamut() {
        for hue in (0..360).step_by(7) {
            for l_step in 1..40 {
                let (hue, l) = (f64::from(hue), f64::from(l_step) / 40.0);
                for gamut in Gamut::all() {
                    let c = gamut.max_chroma(l, hue);

                    for i in 0..=200 {
                        let probe = c * f64::from(i) / 200.0;
                        assert!(
                            gamut.contains(
                                Oklch {
                                    l,
                                    c: probe,
                                    h: hue
                                }
                                .to_oklab()
                            ),
                            "{} l={l} h={hue}: {probe} below reported max {c} is out of gamut",
                            gamut.id()
                        );
                    }

                    // ...and the boundary is real: somewhere just above, the
                    // gamut is genuinely left behind.
                    if c < MAX_REPRESENTABLE_CHROMA {
                        let escapes = (1..=200).any(|i| {
                            let probe = c + 0.02 * f64::from(i) / 200.0;
                            !gamut.contains(
                                Oklch {
                                    l,
                                    c: probe,
                                    h: hue,
                                }
                                .to_oklab(),
                            )
                        });
                        assert!(
                            escapes,
                            "{} l={l} h={hue}: nothing above {c} leaves the gamut",
                            gamut.id()
                        );
                    }
                }
            }
        }
    }

    /// Cross-checks the analytic solver against an independent bisection,
    /// everywhere the two are entitled to agree.
    ///
    /// They disagree by design near a gamut's primaries, where the in-gamut
    /// set along a chroma ray is not an interval and bisection has no defined
    /// answer. Away from that, agreement should be near-exact — which is what
    /// makes this a real check on the cubic algebra.
    #[test]
    fn analytic_solver_agrees_with_bisection_where_the_gamut_is_well_behaved() {
        for gamut in Gamut::all() {
            let mut compared = 0;
            for hue in (0..360).step_by(3) {
                for l_step in 1..20 {
                    let (hue, l) = (f64::from(hue), f64::from(l_step) / 20.0);
                    let analytic = gamut.max_chroma(l, hue);

                    // Well-behaved means: the ray crosses the boundary exactly
                    // once, so both methods are answering the same question.
                    let crossings = (1..2000)
                        .filter(|i| {
                            let at = |c: f64| Oklch { l, c, h: hue }.to_oklab();
                            let step = 0.5 / 2000.0;
                            gamut.contains(at(f64::from(*i) * step))
                                != gamut.contains(at(f64::from(i - 1) * step))
                        })
                        .count();
                    if crossings != 1 {
                        continue;
                    }

                    compared += 1;
                    let bisected = gamut.max_chroma_by_bisection(l, hue);

                    // The two do not agree to the last bit, and should not.
                    // Bisection stops where `contains` does, which is
                    // GAMUT_EPSILON *past* the true boundary; the analytic
                    // solver finds the exact zero crossing. The gap is
                    // GAMUT_EPSILON divided by the channel's slope in chroma,
                    // which at very low lightness reaches a little over 1e-6.
                    assert!(
                        (analytic - bisected).abs() < 1e-5,
                        "{} l={l} h={hue}: analytic {analytic} vs bisection {bisected}",
                        gamut.id()
                    );
                    // The analytic answer must never be the looser of the two:
                    // erring high would put out-of-gamut colors under every
                    // relative chroma below one.
                    assert!(
                        analytic <= bisected + 1e-9,
                        "{} l={l} h={hue}: analytic {analytic} exceeds bisection {bisected}",
                        gamut.id()
                    );
                }
            }
            assert!(
                compared > 1000,
                "{}: only {compared} points compared",
                gamut.id()
            );
        }
    }

    /// The sRGB gamut is **not convex** in Oklab, and this is why
    /// [`Gamut::max_chroma`] solves rather than searches.
    ///
    /// Along the ray at lightness 0.4525 and hue 264.1, the red channel
    /// crosses zero three times: the ray leaves the gamut around chroma 0.27,
    /// re-enters near 0.311, and leaves for good just past 0.313. A bisection
    /// converges to whichever crossing its midpoints happen to bracket, which
    /// made maximum chroma jump 19% between hue 264.0 and 264.1.
    ///
    /// If this test ever fails, the non-convexity is gone and the analytic
    /// solver's justification should be revisited — not the other way round.
    #[test]
    fn the_srgb_gamut_is_non_convex_near_the_blue_primary() {
        let (l, hue) = (0.4525, 264.1);
        let inside = |c: f64| Gamut::Srgb.contains(Oklch { l, c, h: hue }.to_oklab());

        let mut transitions = 0;
        let mut previous = inside(0.0);
        for i in 1..=4000 {
            let current = inside(f64::from(i) * 0.32 / 4000.0);
            if current != previous {
                transitions += 1;
                previous = current;
            }
        }
        assert!(
            transitions >= 3,
            "expected the ray to leave, re-enter and leave again; saw {transitions} transitions"
        );

        // And the solver reports the *first* crossing, not the last.
        let reported = Gamut::Srgb.max_chroma(l, hue);
        assert!(
            (0.26..0.29).contains(&reported),
            "expected the first crossing near 0.27, got {reported}"
        );
    }

    #[test]
    fn max_chroma_collapses_to_zero_at_the_lightness_extremes() {
        for gamut in Gamut::all() {
            assert_eq!(gamut.max_chroma(0.0, 120.0), 0.0);
            assert_eq!(gamut.max_chroma(1.0, 120.0), 0.0);
            assert_eq!(gamut.max_chroma(-0.1, 120.0), 0.0);
            assert_eq!(gamut.max_chroma(1.1, 120.0), 0.0);
        }
    }
}
