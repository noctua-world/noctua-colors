//! Placing stops along a hue path.
//!
//! A scale is read in order — `level-0` through `level-10`, green to red — so
//! what matters is that consecutive stops look one step apart and that the ends
//! look far apart. Spacing them evenly in `t` does not give that: a hue path
//! crosses perceptual space at very uneven rates, and the yellow middle of a
//! traffic light compresses badly against the red end.
//!
//! # Why the walk is measured on one lightness slice
//!
//! A scale varies lightness along its path on purpose — under dichromacy hue is
//! gone and lightness is the only thing left carrying the order — so it is
//! tempting to fold lightness into the measurement and place stops by the total
//! distance each one travels.
//!
//! That is wrong, and measurably so. Folding the two together lets them trade
//! against each other: where the hue path moves quickly, less lightness is
//! needed to reach the same total, so stops bunch in `t` and their lightnesses
//! bunch with them. Measured on the shipped eleven-stop scale, the tightest
//! neighbouring pair came out **0.0192** apart under protanopia — inside the
//! just-noticeable difference, meaning two stops that a protanope reads as one.
//! The bunching lands exactly where hue does the most work, which is exactly
//! where a dichromat has the least.
//!
//! So the two axes are placed separately and each exactly:
//!
//! - **hue and chroma** by equal arc length along the path, measured on the
//!   scale's middle lightness — what normal vision reads;
//! - **lightness** by equal steps of stop *index*, in [`crate::palette`] — what
//!   is left when hue is not available.
//!
//! Their combination comes out even too, since a constant step of one plus an
//! even step of the other is an even step of the pair.
//!
//! # Why not [`crate::chart`]
//!
//! `chart::hues` solves the neighbouring problem — spreading hues *around the
//! wheel* so a categorical set is mutually distinguishable — and its shape is
//! the one borrowed here. It cannot be reused directly: it walks a **closed
//! ring**, with `360.0` hardcoded into both the sampling and the inversion, and
//! it divides the total by `count` because entry `count` wraps back onto entry
//! `0`. A scale is an **open segment** — the first and last stops are endpoints
//! that must land exactly on the ends of the path — so the total divides by
//! `count - 1`, as [`crate::neutral::place`] does over a line.

use noctua_core::{Gamut, delta_e_ok};

use crate::solve::FamilyCurves;

/// How finely the path is walked before inverting. Same resolution as the
/// categorical spread, for the same reason: the measure only has to be smooth
/// enough that interpolating between samples is invisible.
const SAMPLES: usize = 720;

/// Positions in `[0, 1]` for `count` stops, spaced by perceptual distance along
/// the path the curves describe at `lightness`.
///
/// One lightness, not a function of position — see the module documentation for
/// why placing hue and lightness separately is the point rather than an
/// approximation.
///
/// Falls back to even spacing in `t` when the path has no perceptual length —
/// a scale of one hue at one lightness, where every stop is the same color and
/// no placement can improve on any other.
#[must_use]
pub fn place(count: usize, curves: &FamilyCurves, lightness: f64, gamut: Gamut) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![0.0];
    }

    let even = |i: usize| i as f64 / (count - 1) as f64;

    // Cumulative perceptual distance along the path.
    let mut cumulative = Vec::with_capacity(SAMPLES);
    let mut total = 0.0;
    let mut previous: Option<noctua_core::Oklab> = None;
    for i in 0..SAMPLES {
        let t = i as f64 / (SAMPLES - 1) as f64;
        let current = curves.color_at(t, lightness, gamut).oklch.to_oklab();
        if let Some(last) = previous {
            total += delta_e_ok(last, current);
        }
        cumulative.push(total);
        previous = Some(current);
    }

    if total <= 0.0 {
        return (0..count).map(even).collect();
    }

    (0..count)
        .map(|i| {
            // `count - 1` intervals, not `count`: the ends are stops, not the
            // seam of a ring.
            let target = total * i as f64 / (count - 1) as f64;
            let upper = cumulative.partition_point(|&c| c < target);
            if upper == 0 {
                return 0.0;
            }
            if upper >= SAMPLES {
                return 1.0;
            }
            let below = cumulative[upper - 1];
            let above = cumulative[upper];
            let within = if above > below {
                (target - below) / (above - below)
            } else {
                0.0
            };
            (upper - 1) as f64 + within
        })
        .map(|sample| sample / (SAMPLES - 1) as f64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Curve;

    /// A traffic light: green through yellow to red.
    fn traffic_light() -> FamilyCurves {
        FamilyCurves {
            hue: Curve::hue([[0.0, 144.0], [0.5, 90.0], [1.0, 30.0]]),
            chroma: Curve::constant(0.8),
            correction: Curve::constant(0.0),
            multiplier: 1.0,
        }
    }

    /// The slice the walk is measured on.
    const L: f64 = 0.62;

    #[test]
    fn the_ends_are_stops_not_a_seam() {
        // The whole difference from the categorical spread: an ordinal scale's
        // first and last stops sit exactly on the ends of the path. Dividing
        // by `count` instead of `count - 1` would leave the last stop short.
        let positions = place(11, &traffic_light(), L, Gamut::Srgb);
        assert_eq!(positions.len(), 11);
        assert!(positions[0].abs() < 1e-9, "first stop at {}", positions[0]);
        assert!(
            (positions[10] - 1.0).abs() < 1e-9,
            "last stop at {}",
            positions[10]
        );
    }

    #[test]
    fn positions_increase() {
        let positions = place(11, &traffic_light(), L, Gamut::Srgb);
        assert!(
            positions.windows(2).all(|w| w[0] < w[1]),
            "not monotone: {positions:?}"
        );
    }

    /// The arc between consecutive stops, measured independently of `place`
    /// and at a different resolution, so agreement is not agreement with a
    /// shared bug.
    fn arc(curves: &FamilyCurves, from: f64, to: f64) -> f64 {
        const STEPS: usize = 200;
        let point = |t: f64| curves.color_at(t, L, Gamut::Srgb).oklch.to_oklab();
        (0..STEPS)
            .map(|i| {
                let a = from + (to - from) * i as f64 / STEPS as f64;
                let b = from + (to - from) * (i + 1) as f64 / STEPS as f64;
                delta_e_ok(point(a), point(b))
            })
            .sum()
    }

    /// The point of measuring rather than dividing: equal steps of perceptual
    /// distance *along the path*, which is not equal steps in `t`.
    #[test]
    fn stops_are_evenly_spaced_in_perception_not_in_t() {
        let curves = traffic_light();
        let positions = place(9, &curves, L, Gamut::Srgb);

        let arcs: Vec<f64> = positions
            .windows(2)
            .map(|w| arc(&curves, w[0], w[1]))
            .collect();
        let mean = arcs.iter().sum::<f64>() / arcs.len() as f64;
        for a in &arcs {
            assert!(
                (a - mean).abs() < mean * 0.02,
                "arc {a:.4} against mean {mean:.4}: {arcs:?}"
            );
        }

        // And confirm the test is not vacuous — even spacing in `t` would have
        // been visibly uneven here.
        let uneven: Vec<f64> = (0..8)
            .map(|i| arc(&curves, f64::from(i) / 8.0, f64::from(i + 1) / 8.0))
            .collect();
        let spread = |v: &[f64]| {
            let max = v.iter().copied().fold(0.0, f64::max);
            let min = v.iter().copied().fold(f64::INFINITY, f64::min);
            max / min
        };
        assert!(
            spread(&uneven) > 2.0 * spread(&arcs),
            "even-in-t was already even, so this proves nothing: {uneven:?}"
        );
    }

    /// What arc-length placement does *not* promise, recorded so nobody reads
    /// more into the previous test than it says.
    ///
    /// Consecutive stops are one step apart along the path; the straight-line
    /// distance between them is shorter wherever the path bends, and this one
    /// bends at the green end — the sRGB boundary's maximum chroma falls
    /// steeply there, which rotates the direction of travel. Equalizing chords
    /// instead is not the fix: a chord is a shortcut past colors the scale
    /// actually passes through, so equal chords would place stops unevenly in
    /// the thing a reader perceives.
    #[test]
    fn the_chord_between_stops_is_shorter_than_the_arc_where_the_path_bends() {
        let curves = traffic_light();
        let positions = place(9, &curves, L, Gamut::Srgb);
        let chords: Vec<f64> = positions
            .windows(2)
            .map(|w| {
                delta_e_ok(
                    curves.color_at(w[0], L, Gamut::Srgb).oklch.to_oklab(),
                    curves.color_at(w[1], L, Gamut::Srgb).oklch.to_oklab(),
                )
            })
            .collect();

        let arcs: Vec<f64> = positions
            .windows(2)
            .map(|w| arc(&curves, w[0], w[1]))
            .collect();
        for (chord, arc) in chords.iter().zip(&arcs) {
            assert!(chord <= &(arc * 1.000_001), "a chord cannot exceed its arc");
        }
        assert!(
            chords[0] < chords[4] * 0.85,
            "the bend is at the green end: {chords:?}"
        );
    }

    #[test]
    fn a_path_with_no_length_falls_back_to_even_spacing() {
        let flat_path = FamilyCurves {
            hue: Curve::hue([[0.0, 200.0], [1.0, 200.0]]),
            chroma: Curve::constant(0.0),
            correction: Curve::constant(0.0),
            multiplier: 1.0,
        };
        let positions = place(5, &flat_path, L, Gamut::Srgb);
        for (i, p) in positions.iter().enumerate() {
            assert!((p - i as f64 / 4.0).abs() < 1e-9, "{positions:?}");
        }
    }

    #[test]
    fn degenerate_counts_do_not_panic() {
        assert!(place(0, &traffic_light(), L, Gamut::Srgb).is_empty());
        assert_eq!(place(1, &traffic_light(), L, Gamut::Srgb), vec![0.0]);
    }
}
