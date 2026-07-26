//! Real roots of cubic and lower-degree polynomials.
//!
//! Exists for one caller: [`Gamut::max_chroma`](crate::Gamut::max_chroma).
//! At fixed lightness and hue, each linear RGB channel turns out to be an
//! exact cubic in chroma, so the gamut boundary can be *solved* rather than
//! searched. That matters more than speed — see the note there on why a
//! bisection gets the wrong answer near the blue primary.

/// Coefficients of `a x^3 + b x^2 + c x + d`, highest power first.
pub(crate) type Cubic = [f64; 4];

/// Below this magnitude a leading coefficient is treated as absent and the
/// polynomial degenerates to the next lower degree.
const DEGENERATE: f64 = 1e-12;

/// Evaluates the polynomial at `x`.
fn evaluate(p: Cubic, x: f64) -> f64 {
    ((p[0] * x + p[1]) * x + p[2]) * x + p[3]
}

/// Refines a root with a couple of Newton steps.
///
/// Cardano's formula and the trigonometric form both lose precision for
/// nearly-degenerate cubics, and this caller cares about the fourth decimal
/// of the result. Two steps are enough to recover it and cannot wander,
/// because they start from an already-good root.
fn polish(p: Cubic, mut x: f64) -> f64 {
    for _ in 0..2 {
        let value = evaluate(p, x);
        let slope = (3.0 * p[0] * x + 2.0 * p[1]) * x + p[2];
        if slope.abs() < f64::MIN_POSITIVE {
            break;
        }
        x -= value / slope;
    }
    x
}

/// Every real root of the polynomial, unordered and unpolished.
fn real_roots(p: Cubic) -> Vec<f64> {
    let [a, b, c, d] = p;

    if a.abs() < DEGENERATE {
        // Quadratic b x^2 + c x + d.
        if b.abs() < DEGENERATE {
            // Linear c x + d.
            return if c.abs() < DEGENERATE {
                Vec::new()
            } else {
                vec![-d / c]
            };
        }
        let disc = c * c - 4.0 * b * d;
        if disc < 0.0 {
            return Vec::new();
        }
        let root = disc.sqrt();
        return vec![(-c + root) / (2.0 * b), (-c - root) / (2.0 * b)];
    }

    // Normalize to x^3 + px^2 + qx + r, then depress with x = t - p/3.
    let (p2, q, r) = (b / a, c / a, d / a);
    let shift = p2 / 3.0;
    let big_p = q - p2 * p2 / 3.0;
    let big_q = 2.0 * p2 * p2 * p2 / 27.0 - p2 * q / 3.0 + r;

    let half_q = big_q / 2.0;
    let third_p = big_p / 3.0;
    let discriminant = half_q * half_q + third_p * third_p * third_p;

    if discriminant > 0.0 {
        // One real root.
        let root = discriminant.sqrt();
        let t = (-half_q + root).cbrt() + (-half_q - root).cbrt();
        vec![t - shift]
    } else if big_p.abs() < DEGENERATE {
        // Triple root at the inflection.
        vec![-shift]
    } else {
        // Three real roots. `big_p` is necessarily negative here.
        let m = 2.0 * (-third_p).sqrt();
        let inner = (3.0 * big_q) / (big_p * m);
        let phi = inner.clamp(-1.0, 1.0).acos() / 3.0;
        (0..3)
            .map(|k| {
                let angle = f64::from(k) * 2.0 * std::f64::consts::PI / 3.0;
                m * (phi - angle).cos() - shift
            })
            .collect()
    }
}

/// The smallest real root strictly greater than `lower`, if any.
///
/// Roots are polished before comparison, so a root sitting a rounding error
/// below `lower` is not silently promoted past it.
pub(crate) fn smallest_root_above(p: Cubic, lower: f64) -> Option<f64> {
    real_roots(p)
        .into_iter()
        .map(|x| polish(p, x))
        .filter(|x| x.is_finite() && *x > lower)
        .fold(None, |best: Option<f64>, x| {
            Some(best.map_or(x, |b| b.min(x)))
        })
}

#[cfg(test)]
// These assertions compare against literal sentinels the functions return
// verbatim (exactly 0.0, exactly 1.0). Exact comparison is the assertion.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn assert_has_root(p: Cubic, expected: f64) {
        let roots = real_roots(p);
        assert!(
            roots
                .iter()
                .any(|r| (polish(p, *r) - expected).abs() < 1e-9),
            "expected a root at {expected}, found {roots:?}"
        );
    }

    #[test]
    fn finds_three_distinct_real_roots() {
        // (x - 1)(x - 2)(x - 3) = x^3 - 6x^2 + 11x - 6
        let p = [1.0, -6.0, 11.0, -6.0];
        assert_eq!(real_roots(p).len(), 3);
        for expected in [1.0, 2.0, 3.0] {
            assert_has_root(p, expected);
        }
    }

    #[test]
    fn finds_the_single_real_root_of_a_cubic_with_complex_pair() {
        // (x - 2)(x^2 + 1) = x^3 - 2x^2 + x - 2
        let p = [1.0, -2.0, 1.0, -2.0];
        assert_eq!(real_roots(p).len(), 1);
        assert_has_root(p, 2.0);
    }

    #[test]
    fn handles_a_triple_root() {
        // (x - 4)^3
        assert_has_root([1.0, -12.0, 48.0, -64.0], 4.0);
    }

    #[test]
    fn degenerates_to_quadratic_and_linear() {
        assert_has_root([0.0, 1.0, -3.0, 2.0], 1.0); // x^2 - 3x + 2
        assert_has_root([0.0, 1.0, -3.0, 2.0], 2.0);
        assert_has_root([0.0, 0.0, 2.0, -6.0], 3.0); // 2x - 6
        assert!(real_roots([0.0, 0.0, 0.0, 1.0]).is_empty());
    }

    #[test]
    fn smallest_root_above_respects_the_lower_bound() {
        let p = [1.0, -6.0, 11.0, -6.0]; // roots 1, 2, 3
        assert!((smallest_root_above(p, 0.0).expect("root") - 1.0).abs() < 1e-9);
        assert!((smallest_root_above(p, 1.5).expect("root") - 2.0).abs() < 1e-9);
        assert!((smallest_root_above(p, 2.5).expect("root") - 3.0).abs() < 1e-9);
        assert!(smallest_root_above(p, 3.5).is_none());
    }

    #[test]
    fn every_reported_root_actually_evaluates_to_zero() {
        // A spread of awkward shapes, including near-degenerate leading terms.
        let cases: [Cubic; 6] = [
            [1.0, 0.0, -3.0, 0.0],
            [2.5, -1.0, -7.0, 3.0],
            [1e-10, 1.0, -3.0, 2.0],
            [1.0, 3.0, 3.0, 1.0],
            [-4.0, 12.0, -9.0, 1.0],
            [0.3, 0.0, 0.0, -0.024],
        ];
        for p in cases {
            for root in real_roots(p) {
                let x = polish(p, root);
                // Judged relative to the size of the terms being summed. An
                // absolute bound is meaningless here: `[1e-10, 1, -3, 2]` has
                // a genuine root near -1e10, where the individual terms are
                // around 1e20 and f64 cannot resolve their difference to
                // anything near zero.
                let scale = (p[0] * x * x * x)
                    .abs()
                    .max((p[1] * x * x).abs())
                    .max((p[2] * x).abs())
                    .max(p[3].abs())
                    .max(1.0);
                assert!(
                    evaluate(p, x).abs() <= 1e-9 * scale,
                    "{p:?} root {x} evaluates to {} (scale {scale})",
                    evaluate(p, x)
                );
            }
        }
    }
}
