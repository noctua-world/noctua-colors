//! Curve evaluation.
//!
//! Curves are **monotone cubic Hermite splines** (Fritsch–Carlson) over
//! `(t, value)` knots.
//!
//! # Why this and not Bézier
//!
//! Both were on the table; the brief asked for one, consistently. Knots win on
//! three counts:
//!
//! - **Readable.** `[[0.0, 0.15], [0.55, 0.92], [1.0, 0.35]]` says what it
//!   does. Four Bézier control points do not, and nobody edits them by hand
//!   without a curve editor.
//! - **No overshoot.** A plain cubic spline through those same knots would
//!   bulge past 0.92 between them, which for relative chroma means asking for
//!   more saturation than the author wrote — and past 1.0 it would be asking
//!   for more than the gamut has.
//! - **Monotone where authored monotone.** The lightness ramp has a
//!   monotonicity gate. A curve family that can violate it by construction
//!   would make that gate a coin toss.
//!
//! Fritsch–Carlson flattens cleanly at an interior extremum, so the peaked
//! relative-chroma curve — the common case — is well behaved.

/// A curve over `t ∈ [0, 1]`, ready to evaluate.
#[derive(Debug, Clone, PartialEq)]
pub struct Curve {
    knots: Vec<[f64; 2]>,
    tangents: Vec<f64>,
}

impl Curve {
    /// Builds a curve from `(t, value)` knots.
    ///
    /// Knots are sorted by `t`, and knots sharing a `t` are collapsed to the
    /// first — a duplicated position has no meaningful interpolation and
    /// would divide by zero.
    ///
    /// An empty knot list yields the constant zero curve; a single knot yields
    /// that constant. Both are degenerate rather than invalid, and the spec
    /// layer rejects them before they arrive.
    #[must_use]
    pub fn new(knots: impl IntoIterator<Item = [f64; 2]>) -> Self {
        let mut knots: Vec<[f64; 2]> = knots.into_iter().collect();
        knots.sort_by(|a, b| a[0].total_cmp(&b[0]));
        knots.dedup_by(|a, b| (a[0] - b[0]).abs() < f64::EPSILON);

        if knots.is_empty() {
            knots.push([0.0, 0.0]);
        }

        let tangents = fritsch_carlson(&knots);
        Self { knots, tangents }
    }

    /// A curve with the same value everywhere.
    #[must_use]
    pub fn constant(value: f64) -> Self {
        Self::new([[0.0, value]])
    }

    /// Builds a curve whose values are hue angles in degrees.
    ///
    /// Hue is circular, so the knots are unwrapped first: each is shifted by
    /// whole turns to sit within half a turn of its predecessor. Without that,
    /// a ramp from 350 to 10 degrees would interpolate the long way round,
    /// sweeping through every hue on the wheel instead of the twenty degrees
    /// the author asked for.
    #[must_use]
    pub fn hue(knots: impl IntoIterator<Item = [f64; 2]>) -> Self {
        let mut knots: Vec<[f64; 2]> = knots.into_iter().collect();
        knots.sort_by(|a, b| a[0].total_cmp(&b[0]));

        for i in 1..knots.len() {
            let previous = knots[i - 1][1];
            let mut current = knots[i][1];
            while current - previous > 180.0 {
                current -= 360.0;
            }
            while current - previous < -180.0 {
                current += 360.0;
            }
            knots[i][1] = current;
        }

        Self::new(knots)
    }

    /// Evaluates the curve at `t`, clamped to the knot range.
    #[must_use]
    pub fn at(&self, t: f64) -> f64 {
        let knots = &self.knots;
        if knots.len() == 1 {
            return knots[0][1];
        }

        let first = knots[0][0];
        let last = knots[knots.len() - 1][0];
        if t <= first {
            return knots[0][1];
        }
        if t >= last {
            return knots[knots.len() - 1][1];
        }

        let i = knots.partition_point(|k| k[0] <= t).saturating_sub(1);
        let i = i.min(knots.len() - 2);

        let [x0, y0] = knots[i];
        let [x1, y1] = knots[i + 1];
        let h = x1 - x0;
        let s = (t - x0) / h;
        let (m0, m1) = (self.tangents[i], self.tangents[i + 1]);

        // Cubic Hermite basis, written to match the published form rather
        // than to please a lint about fused multiply-add.
        let s2 = s * s;
        let s3 = s2 * s;
        let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
        let h10 = s3 - 2.0 * s2 + s;
        let h01 = -2.0 * s3 + 3.0 * s2;
        let h11 = s3 - s2;

        y0 * h00 + h * m0 * h10 + y1 * h01 + h * m1 * h11
    }

    /// The knots this curve was built from.
    #[must_use]
    pub fn knots(&self) -> &[[f64; 2]] {
        &self.knots
    }
}

/// Fritsch–Carlson tangents: the step that makes the spline monotone.
///
/// Starting from the usual averaged secants, any tangent pair that would let
/// the segment overshoot is scaled back onto the circle of radius 3 in
/// `(alpha, beta)` space. That is the classical sufficient condition for
/// monotonicity.
fn fritsch_carlson(knots: &[[f64; 2]]) -> Vec<f64> {
    let n = knots.len();
    if n < 2 {
        return vec![0.0; n];
    }

    let secants: Vec<f64> = (0..n - 1)
        .map(|i| (knots[i + 1][1] - knots[i][1]) / (knots[i + 1][0] - knots[i][0]))
        .collect();

    let mut tangents = Vec::with_capacity(n);
    tangents.push(secants[0]);
    for i in 1..n - 1 {
        // A sign change means an interior extremum; flattening there is what
        // keeps a peaked curve from overshooting its own peak.
        if secants[i - 1] * secants[i] <= 0.0 {
            tangents.push(0.0);
        } else {
            tangents.push(f64::midpoint(secants[i - 1], secants[i]));
        }
    }
    tangents.push(secants[n - 2]);

    for i in 0..n - 1 {
        if secants[i].abs() < f64::EPSILON {
            tangents[i] = 0.0;
            tangents[i + 1] = 0.0;
            continue;
        }
        let alpha = tangents[i] / secants[i];
        let beta = tangents[i + 1] / secants[i];
        let magnitude = alpha.hypot(beta);
        if magnitude > 3.0 {
            let scale = 3.0 / magnitude;
            tangents[i] = scale * alpha * secants[i];
            tangents[i + 1] = scale * beta * secants[i];
        }
    }

    tangents
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn a_constant_curve_is_constant() {
        let curve = Curve::constant(0.8);
        for i in 0..=10 {
            assert!((curve.at(f64::from(i) / 10.0) - 0.8).abs() < 1e-12);
        }
    }

    #[test]
    fn the_curve_passes_exactly_through_every_knot() {
        let knots = [[0.0, 0.15], [0.55, 0.92], [1.0, 0.35]];
        let curve = Curve::new(knots);
        for [t, expected] in knots {
            assert!(
                (curve.at(t) - expected).abs() < 1e-12,
                "at t={t}: {} != {expected}",
                curve.at(t)
            );
        }
    }

    #[test]
    fn values_outside_the_range_clamp_to_the_ends() {
        let curve = Curve::new([[0.0, 0.2], [1.0, 0.8]]);
        assert!((curve.at(-5.0) - 0.2).abs() < 1e-12);
        assert!((curve.at(5.0) - 0.8).abs() < 1e-12);
    }

    /// The property Bézier control points would not give for free.
    #[test]
    fn a_peaked_curve_never_overshoots_its_peak() {
        let curve = Curve::new([[0.0, 0.15], [0.55, 0.92], [1.0, 0.35]]);
        for i in 0..=1000 {
            let value = curve.at(f64::from(i) / 1000.0);
            assert!(
                (0.15..=0.92).contains(&value),
                "overshot at t={}: {value}",
                f64::from(i) / 1000.0
            );
        }
    }

    #[test]
    fn a_monotone_knot_set_produces_a_monotone_curve() {
        let curve = Curve::new([[0.0, 0.0], [0.3, 0.1], [0.7, 0.75], [1.0, 1.0]]);
        let mut previous = f64::NEG_INFINITY;
        for i in 0..=1000 {
            let value = curve.at(f64::from(i) / 1000.0);
            assert!(
                value >= previous - 1e-12,
                "not monotonic at {i}: {value} < {previous}"
            );
            previous = value;
        }
    }

    #[test]
    fn a_descending_knot_set_stays_descending() {
        let curve = Curve::new([[0.0, 1.0], [0.4, 0.6], [1.0, 0.0]]);
        let mut previous = f64::INFINITY;
        for i in 0..=1000 {
            let value = curve.at(f64::from(i) / 1000.0);
            assert!(value <= previous + 1e-12, "not monotonic at {i}");
            previous = value;
        }
    }

    #[test]
    fn knots_are_sorted_and_deduplicated() {
        let curve = Curve::new([[1.0, 0.8], [0.0, 0.2], [0.5, 0.5], [0.5, 0.9]]);
        let positions: Vec<f64> = curve.knots().iter().map(|k| k[0]).collect();
        assert_eq!(positions, vec![0.0, 0.5, 1.0]);
        // The duplicate must not have produced a division by zero.
        assert!(curve.at(0.5).is_finite());
    }

    #[test]
    fn a_hue_curve_takes_the_short_way_across_the_seam() {
        // 350 to 10 degrees is a 20 degree move, not a 340 degree one.
        let curve = Curve::hue([[0.0, 350.0], [1.0, 10.0]]);
        let middle = curve.at(0.5);
        let normalized = noctua_core::space::normalize_hue(middle);
        assert!(
            !(1.0..359.0).contains(&normalized),
            "went the long way round: {normalized} degrees at the midpoint"
        );
    }

    #[test]
    fn a_hue_curve_without_a_seam_behaves_like_any_other() {
        let curve = Curve::hue([[0.0, 271.0], [1.0, 257.0]]);
        assert!((curve.at(0.0) - 271.0).abs() < 1e-12);
        assert!((curve.at(1.0) - 257.0).abs() < 1e-12);
        assert!((curve.at(0.5) - 264.0).abs() < 1e-9);
    }

    #[test]
    fn degenerate_knot_lists_do_not_panic() {
        assert_eq!(Curve::new([]).at(0.5), 0.0);
        assert_eq!(Curve::new([[0.4, 0.7]]).at(0.9), 0.7);
    }

    #[test]
    fn a_flat_segment_stays_flat() {
        // Equal neighbouring values must not bow, or a deliberately flat
        // stretch of a ramp would acquire a wobble.
        let curve = Curve::new([[0.0, 0.5], [0.5, 0.5], [1.0, 0.9]]);
        for i in 0..=500 {
            let t = f64::from(i) / 1000.0;
            assert!(
                (curve.at(t) - 0.5).abs() < 1e-12,
                "wobbled at t={t}: {}",
                curve.at(t)
            );
        }
    }
}
