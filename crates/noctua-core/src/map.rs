//! Gamut mapping: fitting a color into a gamut without wrecking it.
//!
//! The naive approach — clamp each RGB channel into `[0, 1]` — is wrong in a
//! way that is easy to miss. Clipping one channel and not the others moves the
//! color sideways in hue, so a saturated blue quietly becomes purple and a
//! saturated red becomes orange. Worse, it does this *inconsistently* across a
//! ramp, so the steps that happened to be in gamut keep their hue and the ones
//! that did not do not.
//!
//! This module implements the CSS Color 4 algorithm instead: hold lightness
//! and hue fixed, bisect chroma downward until the result is
//! indistinguishable from its clipped form, and only then clip to absorb
//! floating-point error.

use crate::diff::{JND, delta_e_ok};
use crate::gamut::Gamut;
use crate::space::{LinearRgb, Oklab, Oklch, Rgb};

/// The result of fitting a color into a gamut.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mapped {
    /// The mapped color in OKLCH. Lightness and hue match the input; chroma is
    /// less than or equal to the input's.
    pub oklch: Oklch,
    /// Linear-light RGB, guaranteed within `[0, 1]`.
    pub linear: LinearRgb,
    /// Transfer-encoded RGB, guaranteed within `[0, 1]`.
    pub rgb: Rgb,
    /// How much chroma the mapping had to give up. Zero when the color was
    /// already inside the gamut.
    pub chroma_reduction: f64,
}

/// Chroma resolution at which the bisection stops, matching the CSS Color 4
/// algorithm's epsilon.
const EPSILON: f64 = 0.000_1;

/// Fits `color` into `gamut`, preserving lightness and hue.
///
/// Colors already inside the gamut pass through with chroma untouched.
#[must_use]
pub fn map_into_gamut(color: Oklch, gamut: Gamut) -> Mapped {
    let color = color.normalized();

    // The lightness extremes are single points; no amount of chroma survives.
    if color.l >= 1.0 {
        return at_chroma(
            Oklch {
                l: 1.0,
                c: 0.0,
                h: color.h,
            },
            gamut,
            color.c,
        );
    }
    if color.l <= 0.0 {
        return at_chroma(
            Oklch {
                l: 0.0,
                c: 0.0,
                h: color.h,
            },
            gamut,
            color.c,
        );
    }
    if gamut.contains(color.to_oklab()) {
        return at_chroma(color, gamut, 0.0);
    }

    let mut low = 0.0;
    let mut high = color.c;
    // Tracks whether the lower bound is still known to be inside the gamut.
    // Once the algorithm accepts a clipped result as close enough, the lower
    // bound is no longer a guaranteed-inside chroma, and the cheap membership
    // test must give way to the perceptual one.
    let mut low_in_gamut = true;

    while high - low > EPSILON {
        let chroma = f64::midpoint(low, high);
        let candidate = Oklch { c: chroma, ..color };
        let lab = candidate.to_oklab();

        if low_in_gamut && gamut.contains(lab) {
            low = chroma;
            continue;
        }

        let clipped_linear = clip(gamut.oklab_to_linear(lab));
        let clipped_lab = gamut.linear_to_oklab(clipped_linear);
        let error = delta_e_ok(clipped_lab, lab);

        if error < JND {
            // Close enough that a viewer could not tell the clipped version
            // apart. If it is *only just* close enough, stop here.
            if JND - error < EPSILON {
                return finish(color, candidate, clipped_linear, gamut);
            }
            low_in_gamut = false;
            low = chroma;
        } else {
            high = chroma;
        }
    }

    let final_color = Oklch { c: low, ..color };
    let clipped_linear = clip(gamut.oklab_to_linear(final_color.to_oklab()));
    finish(color, final_color, clipped_linear, gamut)
}

/// Builds a [`Mapped`] for a color already known to be in gamut.
fn at_chroma(color: Oklch, gamut: Gamut, chroma_reduction: f64) -> Mapped {
    let linear = clip(gamut.oklab_to_linear(color.to_oklab()));
    Mapped {
        oklch: color,
        linear,
        rgb: gamut.encode(linear),
        chroma_reduction,
    }
}

/// Assembles the final result.
///
/// The reported OKLCH is derived from the **clipped** linear RGB, not from the
/// bisection's last candidate. Those are not the same color: the algorithm
/// stops once the two are perceptually indistinguishable, which leaves them
/// numerically a little apart.
///
/// Reporting the candidate instead was a real bug. Coordinates and channels
/// would describe slightly different colors, so feeding the coordinates back
/// in mapped them *again* — repeated mapping eroded chroma by about 7e-5 a
/// pass, forever, instead of settling. It also meant an emitter could write an
/// `oklch()` value and a hex fallback that were not the same color.
fn finish(original: Oklch, mapped: Oklch, _linear: LinearRgb, gamut: Gamut) -> Mapped {
    // Two obvious answers are both wrong, and the tests caught each in turn.
    //
    // Reporting the bisection's candidate leaves coordinates that are *not*
    // in gamut, because the algorithm stops as soon as the candidate and its
    // clipped form are perceptually indistinguishable rather than identical.
    // Feeding those coordinates back in maps them again, so repeated mapping
    // eroded chroma by about 7e-5 a pass, forever.
    //
    // Reporting the clipped color's own coordinates fixes that and breaks
    // something worse: clipping moves hue, by up to 15 degrees in the worst
    // case measured. Hue preservation is the entire point of this module.
    //
    // So: keep lightness and hue exactly, and reduce chroma to the most the
    // gamut allows there. `max_chroma` guarantees everything below it is in
    // gamut, which makes the result both faithful and a fixed point.
    let limit = gamut.max_chroma(mapped.l, mapped.h);
    let oklch = Oklch {
        l: mapped.l,
        c: mapped.c.min(limit),
        h: mapped.h,
    };

    let linear = clip(gamut.oklab_to_linear(oklch.to_oklab()));
    Mapped {
        oklch,
        linear,
        rgb: gamut.encode(linear),
        chroma_reduction: (original.c - oklch.c).max(0.0),
    }
}

/// Clamps each linear component into `[0, 1]`.
///
/// Only ever called on a color the bisection has already established is
/// perceptually indistinguishable from its clipped form, which is why this is
/// safe here and catastrophic as a mapping strategy on its own.
fn clip(rgb: LinearRgb) -> LinearRgb {
    LinearRgb {
        r: rgb.r.clamp(0.0, 1.0),
        g: rgb.g.clamp(0.0, 1.0),
        b: rgb.b.clamp(0.0, 1.0),
    }
}

/// Formats encoded RGB as a lowercase `#rrggbb` string.
///
/// Components are rounded to the nearest 8-bit value; inputs outside `[0, 1]`
/// are clamped, so this is total even if handed an unmapped color.
#[must_use]
pub fn to_hex(rgb: Rgb) -> String {
    let channel = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(rgb.r),
        channel(rgb.g),
        channel(rgb.b)
    )
}

/// Parses a `#rgb`, `#rrggbb`, or `#rrggbbaa` string into encoded RGB,
/// discarding any alpha component.
///
/// Used by the palette importer to read existing palettes. Returns `None` for
/// anything it does not recognize.
#[must_use]
pub fn from_hex(s: &str) -> Option<Rgb> {
    let h = s.trim().strip_prefix('#')?;
    let expand = |c: char| c.to_digit(16).map(|d| f64::from(d * 17) / 255.0);
    let pair = |i: usize| {
        u8::from_str_radix(h.get(i..i + 2)?, 16)
            .ok()
            .map(|v| f64::from(v) / 255.0)
    };

    match h.len() {
        3 => {
            let mut cs = h.chars();
            Some(Rgb {
                r: expand(cs.next()?)?,
                g: expand(cs.next()?)?,
                b: expand(cs.next()?)?,
            })
        }
        6 | 8 => Some(Rgb {
            r: pair(0)?,
            g: pair(2)?,
            b: pair(4)?,
        }),
        _ => None,
    }
}

/// Converts encoded RGB in `gamut` back to OKLCH.
#[must_use]
pub fn rgb_to_oklch(rgb: Rgb, gamut: Gamut) -> Oklch {
    gamut.linear_to_oklab(gamut.decode(rgb)).to_oklch()
}

/// Converts an Oklab color to encoded RGB without any gamut mapping.
///
/// Exposed for the CVD simulator and for tests; production paths go through
/// [`map_into_gamut`].
#[must_use]
pub fn oklab_to_rgb_unmapped(lab: Oklab, gamut: Gamut) -> Rgb {
    gamut.encode(gamut.oklab_to_linear(lab))
}

#[cfg(test)]
// These assertions compare against literal sentinels the functions return
// verbatim (exactly 0.0, exactly 1.0). Exact comparison is the assertion.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::space::hue_difference;

    #[test]
    fn colors_already_in_gamut_pass_through_untouched() {
        let color = Oklch {
            l: 0.6,
            c: 0.05,
            h: 240.0,
        };
        let mapped = map_into_gamut(color, Gamut::Srgb);
        assert!((mapped.oklch.c - color.c).abs() < 1e-12);
        assert_eq!(mapped.chroma_reduction, 0.0);
    }

    #[test]
    fn out_of_gamut_colors_lose_chroma_and_keep_hue() {
        // Far outside every gamut at this lightness.
        let color = Oklch {
            l: 0.5,
            c: 0.4,
            h: 264.0,
        };
        let mapped = map_into_gamut(color, Gamut::Srgb);

        assert!(mapped.oklch.c < color.c, "chroma should have been reduced");
        assert!(mapped.chroma_reduction > 0.0);
        assert!(
            hue_difference(color.h, mapped.oklch.h).abs() < 1.0,
            "hue drifted by {} degrees",
            hue_difference(color.h, mapped.oklch.h)
        );
        assert!((mapped.oklch.l - color.l).abs() < 0.02, "lightness drifted");
    }

    #[test]
    fn mapped_output_is_always_inside_the_unit_cube() {
        for hue in (0..360).step_by(7) {
            for l_step in 0..=20 {
                let color = Oklch {
                    l: f64::from(l_step) / 20.0,
                    c: 0.45,
                    h: f64::from(hue),
                };
                for gamut in Gamut::all() {
                    let m = map_into_gamut(color, gamut);
                    for (name, v) in [("r", m.rgb.r), ("g", m.rgb.g), ("b", m.rgb.b)] {
                        assert!(
                            (0.0..=1.0).contains(&v),
                            "{} {name}={v} for {color:?}",
                            gamut.id()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn mapping_never_increases_chroma() {
        for hue in (0..360).step_by(13) {
            for c_step in 0..=10 {
                let color = Oklch {
                    l: 0.55,
                    c: f64::from(c_step) / 20.0,
                    h: f64::from(hue),
                };
                for gamut in Gamut::all() {
                    let m = map_into_gamut(color, gamut);
                    assert!(
                        m.oklch.c <= color.c + 1e-9,
                        "{}: {} > {}",
                        gamut.id(),
                        m.oklch.c,
                        color.c
                    );
                }
            }
        }
    }

    #[test]
    fn a_wider_gamut_keeps_at_least_as_much_chroma() {
        // The whole premise of relative chroma: the same definition should be
        // no less saturated on a wider display.
        for hue in (0..360).step_by(9) {
            let color = Oklch {
                l: 0.6,
                c: 0.3,
                h: f64::from(hue),
            };
            let srgb = map_into_gamut(color, Gamut::Srgb).oklch.c;
            let p3 = map_into_gamut(color, Gamut::DisplayP3).oklch.c;
            assert!(p3 >= srgb - EPSILON, "hue {hue}: p3 {p3} < srgb {srgb}");
        }
    }

    #[test]
    fn pure_white_and_black_survive_mapping() {
        for gamut in Gamut::all() {
            let white = map_into_gamut(
                Oklch {
                    l: 1.0,
                    c: 0.0,
                    h: 0.0,
                },
                gamut,
            );
            assert_eq!(to_hex(white.rgb), "#ffffff", "{}", gamut.id());
            let black = map_into_gamut(
                Oklch {
                    l: 0.0,
                    c: 0.0,
                    h: 0.0,
                },
                gamut,
            );
            assert_eq!(to_hex(black.rgb), "#000000", "{}", gamut.id());
        }
    }

    /// A spread of colors covering the space, built rather than chosen.
    ///
    /// The literals below are hue angles and fractions, not colors: no hex
    /// value is written down anywhere in this crate except where the hex
    /// *format* is what is under test.
    fn sample_colors() -> impl Iterator<Item = Oklch> {
        [0.12, 0.35, 0.55, 0.78, 0.95].into_iter().flat_map(|l| {
            [15.0, 95.0, 150.0, 210.0, 264.0, 320.0]
                .into_iter()
                .map(move |h| {
                    let c = Gamut::Srgb.max_chroma(l, h) * 0.7;
                    Oklch { l, c, h }
                })
        })
    }

    #[test]
    fn hex_round_trips() {
        for color in sample_colors() {
            let rgb = map_into_gamut(color, Gamut::Srgb).rgb;
            let text = to_hex(rgb);
            let parsed = from_hex(&text).expect("output of to_hex must parse");
            assert_eq!(to_hex(parsed), text);
        }
    }

    #[test]
    fn short_hex_expands_by_digit_duplication() {
        assert_eq!(to_hex(from_hex("#abc").expect("valid")), "#aabbcc"); // allow-literal: hex format fixtures: the parser is what is under test
        assert_eq!(to_hex(from_hex("#fff").expect("valid")), "#ffffff");
    }

    #[test]
    fn eight_digit_hex_drops_alpha() {
        assert_eq!(to_hex(from_hex("#abcdef80").expect("valid")), "#abcdef"); // allow-literal: hex format fixture: eight-digit alpha form
    }

    #[test]
    fn malformed_hex_is_rejected_rather_than_guessed() {
        for bad in ["", "#", "abcdef", "#gg0000", "#12345", "#1234567"] {
            assert!(from_hex(bad).is_none(), "{bad} should not parse");
        }
    }

    /// A displayable color survives the whole pipeline unchanged once
    /// quantized. Every emitted artifact rests on this.
    #[test]
    fn colors_survive_a_trip_through_oklch() {
        for color in sample_colors() {
            let start = to_hex(map_into_gamut(color, Gamut::Srgb).rgb);
            let rgb = from_hex(&start).expect("valid hex");
            let back = map_into_gamut(rgb_to_oklch(rgb, Gamut::Srgb), Gamut::Srgb);
            assert_eq!(to_hex(back.rgb), start, "round trip changed {start}");
        }
    }
}
