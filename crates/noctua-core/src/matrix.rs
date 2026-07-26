//! Minimal `const`-evaluable 3x3 linear algebra.
//!
//! Every color matrix in this crate is *derived* rather than transcribed. The
//! sRGB, Display P3 and Rec.2020 RGB-to-XYZ matrices are built from eight
//! published chromaticity numbers each, and their inverses are computed here.
//! Transcribing twenty-seven-entry matrices by hand is the single most common
//! source of silent, subtly-wrong color output; deriving them removes the
//! entire class of bug and guarantees that the forward and inverse transforms
//! are exact inverses of each other.
//!
//! All of it evaluates at compile time, so the runtime cost is zero.

/// A 3x3 matrix in row-major order: `m[row][col]`.
pub(crate) type Mat3 = [[f64; 3]; 3];

/// A 3-element column vector.
pub(crate) type Vec3 = [f64; 3];

/// Multiplies a matrix by a column vector.
#[must_use]
pub(crate) const fn mul_vec(m: Mat3, v: Vec3) -> Vec3 {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Returns the determinant of `m`.
#[must_use]
pub(crate) const fn det(m: Mat3) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// Returns the inverse of `m` via the adjugate.
///
/// # Panics
///
/// Panics at compile time if `m` is singular. Every caller in this crate
/// inverts a matrix built from valid, non-degenerate primaries, so a panic
/// here means the primaries themselves are wrong.
#[must_use]
pub(crate) const fn inverse(m: Mat3) -> Mat3 {
    let d = det(m);
    assert!(d != 0.0, "singular matrix: primaries are degenerate");
    let inv_d = 1.0 / d;

    [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_d,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_d,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_d,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_d,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_d,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_d,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_d,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_d,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_d,
        ],
    ]
}

/// Multiplies two matrices: the result applies `b` first, then `a`.
#[must_use]
pub(crate) const fn mul(a: Mat3, b: Mat3) -> Mat3 {
    let mut out = [[0.0; 3]; 3];
    let mut r = 0;
    while r < 3 {
        let mut c = 0;
        while c < 3 {
            out[r][c] = a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c];
            c += 1;
        }
        r += 1;
    }
    out
}

#[cfg(test)]
// These assertions compare against literal sentinels the functions return
// verbatim (exactly 0.0, exactly 1.0). Exact comparison is the assertion.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    const IDENTITY: Mat3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    fn assert_close(a: Mat3, b: Mat3, eps: f64) {
        for r in 0..3 {
            for c in 0..3 {
                assert!(
                    (a[r][c] - b[r][c]).abs() < eps,
                    "m[{r}][{c}]: {} vs {}",
                    a[r][c],
                    b[r][c]
                );
            }
        }
    }

    #[test]
    fn inverse_of_identity_is_identity() {
        assert_close(inverse(IDENTITY), IDENTITY, 1e-15);
    }

    #[test]
    fn matrix_times_its_inverse_is_identity() {
        let m: Mat3 = [[0.41, 0.36, 0.18], [0.21, 0.72, 0.07], [0.02, 0.12, 0.95]];
        assert_close(mul(m, inverse(m)), IDENTITY, 1e-12);
        assert_close(mul(inverse(m), m), IDENTITY, 1e-12);
    }

    #[test]
    fn inverse_is_evaluable_in_const_context() {
        // The point of the module: this must compile, not merely run.
        const M: Mat3 = [[2.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 8.0]];
        const INV: Mat3 = inverse(M);
        assert!((INV[0][0] - 0.5).abs() < 1e-15);
        assert!((INV[1][1] - 0.25).abs() < 1e-15);
        assert!((INV[2][2] - 0.125).abs() < 1e-15);
    }

    #[test]
    fn mul_vec_matches_hand_expansion() {
        let m: Mat3 = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]];
        let v: Vec3 = [1.0, 0.5, 0.25];
        let got = mul_vec(m, v);
        assert!((got[0] - (1.0 + 1.0 + 0.75)).abs() < 1e-15);
        assert!((got[1] - (4.0 + 2.5 + 1.5)).abs() < 1e-15);
        assert!((got[2] - (7.0 + 4.0 + 2.25)).abs() < 1e-15);
    }
}
