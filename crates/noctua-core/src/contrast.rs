//! Contrast metrics.
//!
//! [`apca`] is the design criterion. [`wcag21`] exists because compliance
//! paperwork asks for it, and for no other reason — it is reported, never
//! solved against.
//!
//! # Why not WCAG 2.x
//!
//! WCAG 2.x models contrast as a ratio of relative luminances with a `+0.05`
//! flare term. That model is polarity-blind and badly calibrated at the dark
//! end: it passes light-gray-on-black pairs that are genuinely hard to read
//! and fails dark-gray-on-mid-gray pairs that are perfectly comfortable.
//! Designing against it produces dark themes that satisfy an audit and hurt to
//! use. APCA models the actual perceptual response, and its asymmetry —
//! `Lc(a, b) != -Lc(b, a)` — is the point, not a defect.

use crate::space::Rgb;

/// Perceptual contrast in APCA lightness-contrast units (Lc).
///
/// The sign carries the polarity, which is APCA's own convention and is
/// load-bearing:
///
/// - **Positive** — dark text on a light background (normal polarity).
/// - **Negative** — light text on a dark background (reverse polarity).
///
/// Magnitudes run to roughly 106 for black on white and 108 for white on
/// black. A rough reading of the scale: 90 for body text, 75 for larger body
/// text, 60 for headlines, 45 for large or bold text, 30 for disabled text or
/// non-text elements, and 15 as the floor at which anything is discernible.
///
/// Both arguments must be the **sRGB** rendition of the color. APCA is
/// defined only for sRGB; for wide-gamut output, pass the sRGB-mapped
/// fallback, since that is what governs legibility across the widest range of
/// displays.
///
/// Implements APCA-W3 revision 0.1.9.
#[must_use]
pub fn apca(text: Rgb, background: Rgb) -> f64 {
    let text_y = soft_clamp_black(screen_luminance(text));
    let bg_y = soft_clamp_black(screen_luminance(background));

    // Luminances this close together produce noise, not contrast.
    if (bg_y - text_y).abs() < DELTA_Y_MIN {
        return 0.0;
    }

    let raw = if bg_y > text_y {
        // Normal polarity: dark text on a light background.
        let sapc = (bg_y.powf(NORM_BG) - text_y.powf(NORM_TEXT)) * SCALE_BOW;
        if sapc < LO_CLIP {
            0.0
        } else {
            sapc - LO_BOW_OFFSET
        }
    } else {
        // Reverse polarity: light text on a dark background.
        let sapc = (bg_y.powf(REV_BG) - text_y.powf(REV_TEXT)) * SCALE_WOB;
        if sapc > -LO_CLIP {
            0.0
        } else {
            sapc + LO_WOB_OFFSET
        }
    };

    raw * 100.0
}

/// The magnitude of [`apca`], discarding polarity.
///
/// Use this only when comparing against a threshold that is genuinely
/// polarity-independent. Most scale roles are not: a role knows whether it is
/// text on a surface or a surface behind text.
#[must_use]
pub fn apca_magnitude(text: Rgb, background: Rgb) -> f64 {
    apca(text, background).abs()
}

// --- APCA-W3 0.1.9 constants ----------------------------------------------

/// Coefficients for APCA's estimate of screen luminance.
const R_CO: f64 = 0.212_672_9;
const G_CO: f64 = 0.715_152_2;
const B_CO: f64 = 0.072_175_0;

/// APCA's tone response curve exponent.
const MAIN_TRC: f64 = 2.4;

/// Below this luminance, APCA soft-clamps to model black-level flare.
const BLACK_THRESHOLD: f64 = 0.022;
const BLACK_CLAMP: f64 = 1.414;

const NORM_BG: f64 = 0.56;
const NORM_TEXT: f64 = 0.57;
const REV_TEXT: f64 = 0.62;
const REV_BG: f64 = 0.65;

const SCALE_BOW: f64 = 1.14;
const SCALE_WOB: f64 = 1.14;
const LO_BOW_OFFSET: f64 = 0.027;
const LO_WOB_OFFSET: f64 = 0.027;

const DELTA_Y_MIN: f64 = 0.000_5;
const LO_CLIP: f64 = 0.1;

/// APCA's estimate of the luminance a screen emits.
///
/// This deliberately uses a plain 2.4 power rather than the true sRGB transfer
/// function with its linear toe. It is not a colorimetric luminance and must
/// not be "corrected" into one: APCA's constants were fitted against *this*
/// curve, and substituting the piecewise sRGB decode shifts every Lc value.
fn screen_luminance(rgb: Rgb) -> f64 {
    let ch = |v: f64| v.clamp(0.0, 1.0).powf(MAIN_TRC);
    R_CO * ch(rgb.r) + G_CO * ch(rgb.g) + B_CO * ch(rgb.b)
}

/// Models the flare that keeps very dark screen pixels from being truly black.
fn soft_clamp_black(y: f64) -> f64 {
    if y > BLACK_THRESHOLD {
        y
    } else {
        y + (BLACK_THRESHOLD - y).powf(BLACK_CLAMP)
    }
}

// --- WCAG 2.x -------------------------------------------------------------

/// The WCAG 2.x contrast ratio between two colors, from 1.0 to 21.0.
///
/// Symmetric, unlike [`apca`]. **Reporting only.** No gate in this project
/// solves against this number; see the module documentation for why.
#[must_use]
pub fn wcag21(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (wcag_luminance(a), wcag_luminance(b));
    let (lighter, darker) = if la > lb { (la, lb) } else { (lb, la) };
    (lighter + 0.05) / (darker + 0.05)
}

/// WCAG 2.x relative luminance, which — unlike APCA's estimate — does use the
/// true sRGB transfer function.
fn wcag_luminance(rgb: Rgb) -> f64 {
    let ch = |v: f64| {
        let v = v.clamp(0.0, 1.0);
        if v <= 0.040_45 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * ch(rgb.r) + 0.7152 * ch(rgb.g) + 0.0722 * ch(rgb.b)
}

#[cfg(test)]
// These assertions compare against literal sentinels the functions return
// verbatim (exactly 0.0, exactly 1.0). Exact comparison is the assertion.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::map::from_hex;

    /// Parses a hex string. Used only where the value is a *published
    /// reference constant* of APCA or WCAG, never to pick a color.
    fn hex(s: &str) -> Rgb {
        from_hex(s).expect("test hex is valid")
    }

    /// A neutral at the given lightness, built rather than chosen.
    fn gray(lightness: f64) -> Rgb {
        crate::map::map_into_gamut(
            crate::space::Oklch {
                l: lightness,
                c: 0.0,
                h: 0.0,
            },
            crate::gamut::Gamut::Srgb,
        )
        .rgb
    }

    /// The two anchor values published with the APCA specification. If these
    /// drift, a constant above is wrong.
    #[test]
    fn apca_matches_the_published_anchor_values() {
        let black_on_white = apca(hex("#000000"), hex("#ffffff"));
        assert!(
            (black_on_white - 106.04).abs() < 0.01,
            "black on white: {black_on_white}"
        );

        let white_on_black = apca(hex("#ffffff"), hex("#000000"));
        assert!(
            (white_on_black - -107.88).abs() < 0.01,
            "white on black: {white_on_black}"
        );
    }

    #[test]
    fn apca_polarity_is_signed_and_asymmetric() {
        let (dark, light) = (gray(0.3), gray(0.92));
        let a = apca(dark, light);
        let b = apca(light, dark);
        assert!(a > 0.0, "dark on light should be positive, got {a}");
        assert!(b < 0.0, "light on dark should be negative, got {b}");
        // The asymmetry is the entire reason APCA exists; if these ever match
        // in magnitude, the reverse-polarity branch stopped being used.
        assert!(
            (a.abs() - b.abs()).abs() > 0.5,
            "expected asymmetry, got {a} and {b}"
        );
    }

    #[test]
    fn identical_colors_have_no_contrast() {
        for step in 0..=10 {
            let c = gray(f64::from(step) / 10.0);
            assert_eq!(apca(c, c), 0.0, "gray at l={}", f64::from(step) / 10.0);
        }
    }

    #[test]
    fn contrast_falls_monotonically_as_text_lightens_toward_the_background() {
        let bg = gray(1.0);
        let mut previous = f64::INFINITY;
        for step in (0..=255).step_by(15) {
            let v = f64::from(step) / 255.0;
            let lc = apca(Rgb { r: v, g: v, b: v }, bg);
            assert!(
                lc <= previous + 1e-9,
                "not monotonic at {step}: {lc} > {previous}"
            );
            previous = lc;
        }
        // ...all the way down to nothing when text and background coincide.
        assert_eq!(apca(bg, bg), 0.0);
    }

    #[test]
    fn near_identical_luminances_clip_to_zero_rather_than_reporting_noise() {
        let a = Rgb {
            r: 0.5,
            g: 0.5,
            b: 0.5,
        };
        let b = Rgb {
            r: 0.5001,
            g: 0.5001,
            b: 0.5001,
        };
        assert_eq!(apca(a, b), 0.0);
    }

    #[test]
    fn wcag_matches_its_published_extremes() {
        assert!((wcag21(hex("#000000"), hex("#ffffff")) - 21.0).abs() < 0.01);
        assert!((wcag21(hex("#ffffff"), hex("#ffffff")) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn wcag_is_symmetric_where_apca_is_not() {
        let (a, b) = (gray(0.3), gray(0.92));
        assert!((wcag21(a, b) - wcag21(b, a)).abs() < 1e-12);
        assert!((apca(a, b).abs() - apca(b, a).abs()).abs() > 0.5);
    }

    /// The concrete reason this project does not design against WCAG 2.x.
    ///
    /// Both pairs below clear the WCAG 4.5:1 bar for body text, yet APCA rates
    /// them very differently — the dark-mode pair is markedly weaker. A system
    /// tuned to WCAG would ship both as equivalent.
    ///
    /// These four hex values are the one place in this crate where a color is
    /// written down rather than computed. They are reference points, not
    /// design choices: each sits just above a WCAG threshold, which is exactly
    /// what makes the divergence visible.
    #[test]
    fn wcag_rates_pairs_as_equivalent_that_apca_separates() {
        let light_mode = (hex("#767676"), hex("#ffffff")); // allow-literal: published reference pair, near the WCAG AA threshold
        let dark_mode = (hex("#9a9a9a"), hex("#000000")); // allow-literal: published reference pair, near the WCAG AAA threshold

        for (fg, bg) in [light_mode, dark_mode] {
            assert!(wcag21(fg, bg) >= 4.5, "premise: both pairs pass WCAG AA");
        }

        let light_lc = apca(light_mode.0, light_mode.1).abs();
        let dark_lc = apca(dark_mode.0, dark_mode.1).abs();
        assert!(
            light_lc - dark_lc > 5.0,
            "expected APCA to separate these; got {light_lc} and {dark_lc}"
        );
    }
}
