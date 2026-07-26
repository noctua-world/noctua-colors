//! The categorical scale.
//!
//! A ramp is ordered; a categorical scale is not. Its colors stand for things
//! with no natural sequence — series in a chart, tags, participants — and the
//! only thing that matters is that no two are mistaken for each other.
//!
//! # Why equal hue angles are the wrong default
//!
//! Rotating hue by a fixed number of degrees does not change appearance by a
//! fixed amount. The stretch of the wheel between yellow and green is
//! perceptually cramped, while the blues are spacious. Eight colors at 45
//! degree intervals therefore come out visibly lumpy: two near-twins in the
//! greens, a lonely blue.
//!
//! The default instead places hues at equal *perceptual* intervals, by
//! measuring the wheel in delta-E and stepping evenly through that.

use noctua_core::{Gamut, Oklch, delta_e_ok};
use noctua_spec::Spread;

/// Resolution of the perceptual arc-length measurement.
///
/// Half a degree, which is finer than the hue differences any categorical set
/// cares about.
const SAMPLES: usize = 720;

/// The golden angle, which spreads consecutive entries far apart.
const GOLDEN_ANGLE: f64 = 137.507_764_050_037_85;

/// Chooses `count` hues starting from `start`.
///
/// `lightness` and `relative_chroma` describe the colors the hues will be used
/// at, because perceptual spacing depends on both — the wheel is a different
/// shape at different lightness.
#[must_use]
pub fn hues(
    count: usize,
    start: f64,
    spread: Spread,
    lightness: f64,
    relative_chroma: f64,
    gamut: Gamut,
) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }

    match spread {
        Spread::EvenHue => (0..count)
            .map(|i| start + 360.0 * i as f64 / count as f64)
            .collect(),
        Spread::Golden => (0..count)
            .map(|i| start + GOLDEN_ANGLE * i as f64)
            .collect(),
        Spread::EvenDeltaE => even_delta_e(count, start, lightness, relative_chroma, gamut),
    }
}

/// Places hues at equal perceptual intervals around the wheel.
///
/// Walks the wheel once measuring cumulative delta-E between neighbouring
/// samples, then inverts that to find the hues sitting at equal fractions of
/// the total. Same cumulative-measure-and-invert shape as the neutral ramp's
/// density placement, applied to a circle rather than a line.
fn even_delta_e(
    count: usize,
    start: f64,
    lightness: f64,
    relative_chroma: f64,
    gamut: Gamut,
) -> Vec<f64> {
    let color_at = |hue: f64| {
        let chroma = relative_chroma * gamut.max_chroma(lightness, hue);
        Oklch {
            l: lightness,
            c: chroma,
            h: hue,
        }
        .to_oklab()
    };

    // Cumulative perceptual distance travelled, sample by sample.
    let mut cumulative = Vec::with_capacity(SAMPLES + 1);
    cumulative.push(0.0);
    let mut total = 0.0;
    let mut previous = color_at(start);
    for i in 1..=SAMPLES {
        let hue = start + 360.0 * i as f64 / SAMPLES as f64;
        let current = color_at(hue);
        total += delta_e_ok(previous, current);
        cumulative.push(total);
        previous = current;
    }

    if total <= 0.0 {
        // Achromatic: every hue looks the same, so spacing is meaningless and
        // equal angles are as good an answer as any.
        return (0..count)
            .map(|i| start + 360.0 * i as f64 / count as f64)
            .collect();
    }

    (0..count)
        .map(|i| {
            let target = total * i as f64 / count as f64;
            let upper = cumulative.partition_point(|&c| c < target).max(1);
            let (low, high) = (cumulative[upper - 1], cumulative[upper]);
            let within = if (high - low).abs() < f64::EPSILON {
                0.0
            } else {
                (target - low) / (high - low)
            };
            start + 360.0 * (upper as f64 - 1.0 + within) / SAMPLES as f64
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use noctua_core::space::hue_difference;

    fn spacing(hues: &[f64], lightness: f64, cr: f64) -> Vec<f64> {
        let color = |hue: f64| {
            let chroma = cr * Gamut::Srgb.max_chroma(lightness, hue);
            Oklch {
                l: lightness,
                c: chroma,
                h: hue,
            }
            .to_oklab()
        };
        hues.windows(2)
            .map(|p| delta_e_ok(color(p[0]), color(p[1])))
            .collect()
    }

    #[test]
    fn the_requested_number_of_hues_comes_back() {
        for count in [1, 3, 8, 12, 24] {
            for spread in [Spread::EvenHue, Spread::Golden, Spread::EvenDeltaE] {
                let hues = hues(count, 264.0, spread, 0.62, 0.85, Gamut::Srgb);
                assert_eq!(hues.len(), count, "{spread:?} with {count}");
            }
        }
    }

    #[test]
    fn every_scale_starts_where_it_was_told_to() {
        for spread in [Spread::EvenHue, Spread::Golden, Spread::EvenDeltaE] {
            let hues = hues(8, 264.0, spread, 0.62, 0.85, Gamut::Srgb);
            assert!(
                (hues[0] - 264.0).abs() < 1e-9,
                "{spread:?} started at {}",
                hues[0]
            );
        }
    }

    #[test]
    fn equal_angles_really_are_equal_angles() {
        let hues = hues(8, 0.0, Spread::EvenHue, 0.62, 0.85, Gamut::Srgb);
        for (i, hue) in hues.iter().enumerate() {
            assert!((hue - 45.0 * i as f64).abs() < 1e-9);
        }
    }

    /// The claim this module is built on, measured — and its limit.
    ///
    /// Placement equalizes distance travelled *along* the hue circle, but
    /// evenness is judged by the straight-line difference between the chosen
    /// colors. Those two agree only where the path is straight, and the path
    /// bends sharply near a gamut primary, so some residual unevenness is
    /// inherent rather than a tuning failure.
    ///
    /// Measured across start angles: equal angles spread 1.93x to 2.48x
    /// between the widest and narrowest gap; perceptual placement spreads
    /// 1.18x to 1.84x. Better everywhere, perfect nowhere.
    #[test]
    fn perceptual_spacing_is_more_even_than_angular_spacing() {
        let (lightness, cr) = (0.62, 0.85);

        let spread_of = |spread: Spread, start: f64| {
            let hues = hues(8, start, spread, lightness, cr, Gamut::Srgb);
            // The set is a circle, so the wrap-around gap counts too.
            let mut gaps = spacing(&hues, lightness, cr);
            gaps.extend(spacing(&[hues[7], hues[0]], lightness, cr));
            let max = gaps.iter().copied().fold(f64::MIN, f64::max);
            let min = gaps.iter().copied().fold(f64::MAX, f64::min);
            max / min
        };

        for start in [0.0, 20.0, 137.0, 264.0, 330.0] {
            let angular = spread_of(Spread::EvenHue, start);
            let perceptual = spread_of(Spread::EvenDeltaE, start);
            assert!(
                perceptual < angular,
                "from {start} degrees: perceptual {perceptual:.2}x should beat angular {angular:.2}x"
            );
            assert!(
                perceptual < 2.0,
                "from {start} degrees: {perceptual:.2}x is worse than the measured worst case"
            );
        }
    }

    #[test]
    fn no_two_entries_land_on_the_same_hue() {
        for spread in [Spread::EvenHue, Spread::Golden, Spread::EvenDeltaE] {
            let hues = hues(12, 264.0, spread, 0.62, 0.85, Gamut::Srgb);
            for (i, a) in hues.iter().enumerate() {
                for b in hues.iter().skip(i + 1) {
                    assert!(
                        hue_difference(*a, *b).abs() > 1.0,
                        "{spread:?}: {a} and {b} are the same hue"
                    );
                }
            }
        }
    }

    #[test]
    fn the_golden_angle_separates_the_first_few_entries_well() {
        // Its reason for existing: a chart using only three of eight series
        // still gets three clearly different colors.
        let hues = hues(8, 0.0, Spread::Golden, 0.62, 0.85, Gamut::Srgb);
        for pair in hues.windows(2) {
            assert!(
                hue_difference(pair[0], pair[1]).abs() > 80.0,
                "consecutive entries too close: {pair:?}"
            );
        }
    }

    #[test]
    fn an_achromatic_scale_does_not_divide_by_zero() {
        let hues = hues(8, 0.0, Spread::EvenDeltaE, 0.62, 0.0, Gamut::Srgb);
        assert_eq!(hues.len(), 8);
        assert!(hues.iter().all(|h| h.is_finite()));
    }

    #[test]
    fn placement_is_deterministic() {
        let once = hues(8, 264.0, Spread::EvenDeltaE, 0.62, 0.85, Gamut::Srgb);
        let twice = hues(8, 264.0, Spread::EvenDeltaE, 0.62, 0.85, Gamut::Srgb);
        assert_eq!(once, twice);
    }
}
