//! Formatting resolved colors as text.
//!
//! Shared so that every target spells the same color the same way, and so the
//! precision decision is made once.

use noctua_engine::ResolvedColor;

/// A CSS `oklch()` function.
///
/// Printed at exactly the precision the palette was quantized to, so the text
/// is a faithful record of the value that was checked rather than a rounding
/// of it.
#[must_use]
pub fn oklch(color: &ResolvedColor) -> String {
    format!(
        "oklch({:.4} {:.4} {:.2})",
        color.oklch.l, color.oklch.c, color.oklch.h
    )
}

/// A CSS `color()` function in the given predefined space.
///
/// Used for wide-gamut layers where a consumer wants explicit channels rather
/// than letting the browser map an `oklch()` value.
#[must_use]
pub fn color_function(color: &ResolvedColor) -> String {
    format!(
        "color({} {:.5} {:.5} {:.5})",
        color.gamut.id(),
        color.rgb.r,
        color.rgb.g,
        color.rgb.b
    )
}

/// `#rrggbb`, always lowercase.
#[must_use]
pub fn hex(color: &ResolvedColor) -> String {
    color.hex()
}

/// The eight-bit alpha byte for a percentage.
#[must_use]
pub fn alpha_byte(percentage: f64) -> u8 {
    (percentage.clamp(0.0, 100.0) / 100.0 * 255.0).round() as u8
}

/// `#rrggbbaa`, the CSS and web ordering.
#[must_use]
pub fn hex_rgba(color: &ResolvedColor, percentage: f64) -> String {
    format!("{}{:02x}", hex(color), alpha_byte(percentage))
}

/// `#aarrggbb`, the **Qt** ordering, in which alpha leads.
///
/// A separate function rather than a flag, because the two forms are
/// indistinguishable by inspection and picking the wrong one produces a
/// plausible colour rather than an error: the same eight digits are a mostly
/// transparent black in one reading and an opaque near-black in the other.
#[must_use]
pub fn hex_argb(color: &ResolvedColor, percentage: f64) -> String {
    let (r, g, b) = rgb_bytes(color);
    format!("#{:02x}{r:02x}{g:02x}{b:02x}", alpha_byte(percentage))
}

/// A CSS `color-mix()` that is the referenced token at `percentage` opacity.
///
/// Mixing with `transparent` in `oklab` is premultiplied, so this *is* the token
/// at that alpha — not a blend toward some other colour. It composites correctly
/// over any backdrop, needs no alpha field anywhere in the colour model, and
/// follows whichever mode and gamut layer defined the token it references.
#[must_use]
pub fn color_mix(token: &str, percentage: f64) -> String {
    format!("color-mix(in oklab, var({token}) {percentage}%, transparent)")
}

/// The three eight-bit channel values.
#[must_use]
pub fn rgb_bytes(color: &ResolvedColor) -> (u8, u8, u8) {
    let channel = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        channel(color.rgb.r),
        channel(color.rgb.g),
        channel(color.rgb.b),
    )
}

#[cfg(test)]
mod tests {
    use noctua_core::{Gamut, Oklch};
    use noctua_engine::ResolvedColor;

    use super::*;

    fn sample() -> ResolvedColor {
        let oklch = Oklch {
            l: 0.6584,
            c: 0.0985,
            h: 57.71,
        };
        let mapped = noctua_core::map_into_gamut(oklch, Gamut::Srgb);
        ResolvedColor {
            gamut: Gamut::Srgb,
            oklch,
            rgb: mapped.rgb,
            requested_relative_chroma: 0.62,
            achieved_relative_chroma: 0.62,
            chroma_headroom: 0.05,
        }
    }

    #[test]
    fn oklch_prints_at_the_quantized_precision() {
        assert_eq!(oklch(&sample()), "oklch(0.6584 0.0985 57.71)");
    }

    #[test]
    fn hex_is_six_lowercase_digits() {
        let text = hex(&sample());
        assert_eq!(text.len(), 7);
        assert!(text.starts_with('#'));
        assert_eq!(text, text.to_lowercase());
    }

    /// Qt reads the eight-digit form as ARGB and the web reads it as RGBA. The
    /// same string means two different colours, so the two forms have to be
    /// produced by two different functions.
    #[test]
    fn the_two_eight_digit_orderings_are_not_the_same_string() {
        let color = sample();
        let rgba = hex_rgba(&color, 60.0);
        let argb = hex_argb(&color, 60.0);
        assert_eq!(rgba.len(), 9);
        assert_eq!(argb.len(), 9);
        assert_ne!(rgba, argb);
        assert!(rgba.ends_with("99"), "alpha trails in RGBA: {rgba}");
        assert!(argb.starts_with("#99"), "alpha leads in ARGB: {argb}");
        assert_eq!(&rgba[1..7], &argb[3..9], "same channels, moved");
    }

    #[test]
    fn an_alpha_byte_spans_the_whole_range() {
        assert_eq!(alpha_byte(0.0), 0);
        assert_eq!(alpha_byte(100.0), 255);
        assert_eq!(alpha_byte(50.0), 128);
        // Out of range is clamped rather than wrapped, because wrapping turns
        // a typo into an invisible token.
        assert_eq!(alpha_byte(-5.0), 0);
        assert_eq!(alpha_byte(140.0), 255);
    }

    /// Mixing with `transparent` in `oklab` is premultiplied, which is the
    /// whole reason this is a correct way to express alpha.
    #[test]
    fn the_mix_names_oklab_and_transparent() {
        let mix = color_mix("--nc-neutral-text-strong", 6.0);
        assert!(mix.contains("in oklab"), "{mix}");
        assert!(mix.ends_with("6%, transparent)"), "{mix}");
    }

    #[test]
    fn the_color_function_names_its_space() {
        assert!(color_function(&sample()).starts_with("color(srgb "));
    }

    #[test]
    fn channel_bytes_match_the_hex() {
        let color = sample();
        let (r, g, b) = rgb_bytes(&color);
        assert_eq!(format!("#{r:02x}{g:02x}{b:02x}"), hex(&color));
    }
}
