//! Perceptual color difference.

use crate::space::Oklab;

/// A just-noticeable difference in Oklab, as used by the CSS Color 4 gamut
/// mapping algorithm.
///
/// This is the threshold below which the algorithm accepts a channel-clipped
/// result as indistinguishable from the chroma-reduced one.
pub const JND: f64 = 0.02;

/// Euclidean distance in Oklab.
///
/// Oklab is designed so that Euclidean distance approximates perceptual
/// difference, which is the entire reason the compiler works in it. There is
/// no weighting term and no parametric factor: if this function ever grows
/// one, the space stopped being uniform and the choice of space should be
/// revisited instead.
#[must_use]
pub fn delta_e_ok(a: Oklab, b: Oklab) -> f64 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    dl.mul_add(dl, da.mul_add(da, db * db)).sqrt()
}

#[cfg(test)]
// These assertions compare against literal sentinels the functions return
// verbatim (exactly 0.0, exactly 1.0). Exact comparison is the assertion.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn distance_to_self_is_zero() {
        let c = Oklab {
            l: 0.5,
            a: 0.1,
            b: -0.05,
        };
        assert!(delta_e_ok(c, c) < f64::EPSILON);
    }

    #[test]
    fn distance_is_symmetric() {
        let a = Oklab {
            l: 0.5,
            a: 0.1,
            b: -0.05,
        };
        let b = Oklab {
            l: 0.7,
            a: -0.02,
            b: 0.09,
        };
        assert!((delta_e_ok(a, b) - delta_e_ok(b, a)).abs() < 1e-15);
    }

    #[test]
    fn distance_matches_hand_computed_euclidean() {
        let a = Oklab {
            l: 0.0,
            a: 0.0,
            b: 0.0,
        };
        let b = Oklab {
            l: 3.0,
            a: 4.0,
            b: 0.0,
        };
        assert!((delta_e_ok(a, b) - 5.0).abs() < 1e-15);
    }

    #[test]
    fn black_to_white_is_one_unit_of_lightness() {
        let black = Oklab {
            l: 0.0,
            a: 0.0,
            b: 0.0,
        };
        let white = Oklab {
            l: 1.0,
            a: 0.0,
            b: 0.0,
        };
        assert!((delta_e_ok(black, white) - 1.0).abs() < 1e-15);
    }
}
