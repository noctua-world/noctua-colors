//! Where the neutral ramp puts its steps.
//!
//! Interfaces do not use grays evenly. A light theme spends almost all its
//! surfaces in a narrow band just below white — page, card, input, hover, each
//! a hair apart — and a dark theme does the same just above black. The middle
//! of the ramp is mostly borders and disabled text, and needs far less
//! resolution.
//!
//! A ramp spread evenly over lightness therefore wastes most of its steps
//! where nothing needs them and runs out exactly where fine discrimination
//! matters. This module places steps according to a **density** declared in
//! the spec: integrate the density into a cumulative measure, then place steps
//! at equal intervals of *that* rather than of lightness.

use noctua_spec::DensityBand;

/// Places `count` lightness values across `[low, high]`, weighted by `bands`.
///
/// The result is sorted ascending, always starts at `low` and ends at `high`,
/// and is strictly increasing. Density outside every band is `1.0`, so a band
/// of weight 3 receives roughly three times the steps of an equal stretch of
/// unbanded ramp.
#[must_use]
pub fn place(count: usize, low: f64, high: f64, bands: &[DensityBand]) -> Vec<f64> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 || high <= low {
        return vec![low];
    }

    let segments = segments(low, high, bands);

    // Cumulative "step demand" at each segment boundary.
    let mut cumulative = Vec::with_capacity(segments.len() + 1);
    cumulative.push(0.0);
    let mut total = 0.0;
    for &(start, end, density) in &segments {
        total += (end - start) * density;
        cumulative.push(total);
    }

    if total <= 0.0 {
        return (0..count)
            .map(|i| low + (high - low) * i as f64 / (count - 1) as f64)
            .collect();
    }

    (0..count)
        .map(|i| {
            let target = total * i as f64 / (count - 1) as f64;
            invert(&segments, &cumulative, target).clamp(low, high)
        })
        .collect()
}

/// Splits `[low, high]` at every band edge, tagging each piece with its
/// density.
fn segments(low: f64, high: f64, bands: &[DensityBand]) -> Vec<(f64, f64, f64)> {
    let mut edges = vec![low, high];
    for band in bands {
        for edge in band.range {
            if edge > low && edge < high {
                edges.push(edge);
            }
        }
    }
    edges.sort_by(f64::total_cmp);
    edges.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

    edges
        .windows(2)
        .map(|pair| {
            let middle = f64::midpoint(pair[0], pair[1]);
            let density = bands
                .iter()
                .find(|b| middle >= b.range[0] && middle <= b.range[1])
                .map_or(1.0, |b| b.weight);
            (pair[0], pair[1], density)
        })
        .collect()
}

/// Finds the lightness whose cumulative demand equals `target`.
fn invert(segments: &[(f64, f64, f64)], cumulative: &[f64], target: f64) -> f64 {
    for (i, &(start, end, density)) in segments.iter().enumerate() {
        if target <= cumulative[i + 1] || i == segments.len() - 1 {
            if density <= 0.0 {
                return start;
            }
            return (start + (target - cumulative[i]) / density).min(end);
        }
    }
    segments.last().map_or(0.0, |s| s.1)
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn band(low: f64, high: f64, weight: f64) -> DensityBand {
        DensityBand {
            range: [low, high],
            weight,
        }
    }

    #[test]
    fn no_bands_gives_an_even_spread() {
        let steps = place(5, 0.0, 1.0, &[]);
        for (i, value) in steps.iter().enumerate() {
            assert!(
                (value - f64::from(i as u32) / 4.0).abs() < 1e-9,
                "{steps:?}"
            );
        }
    }

    #[test]
    fn the_ends_are_always_hit_exactly() {
        let bands = [band(0.1, 0.25, 3.0), band(0.85, 0.99, 3.0)];
        for count in [2, 3, 12, 32, 64] {
            let steps = place(count, 0.04, 0.99, &bands);
            assert_eq!(steps.len(), count);
            assert!(
                (steps[0] - 0.04).abs() < 1e-9,
                "count {count}: {:?}",
                steps[0]
            );
            assert!((steps[count - 1] - 0.99).abs() < 1e-9, "count {count}");
        }
    }

    #[test]
    fn steps_are_strictly_increasing() {
        let bands = [band(0.1, 0.25, 3.0), band(0.85, 0.99, 3.0)];
        let steps = place(32, 0.04, 0.99, &bands);
        for pair in steps.windows(2) {
            assert!(pair[1] > pair[0], "not increasing: {pair:?}");
        }
    }

    /// The whole point: a weighted band gets proportionally more steps.
    #[test]
    fn a_weighted_band_receives_more_steps_than_an_equal_unweighted_stretch() {
        let bands = [band(0.80, 0.95, 4.0)];
        let steps = place(40, 0.0, 1.0, &bands);

        let inside = steps
            .iter()
            .filter(|&&s| (0.80..=0.95).contains(&s))
            .count();
        // An equally wide stretch elsewhere, at baseline density.
        let elsewhere = steps
            .iter()
            .filter(|&&s| (0.20..=0.35).contains(&s))
            .count();

        assert!(
            inside > elsewhere * 2,
            "weighted band got {inside} steps, comparable plain stretch got {elsewhere}"
        );
    }

    #[test]
    fn both_ends_can_be_weighted_at_once() {
        let bands = [band(0.10, 0.25, 3.0), band(0.85, 0.99, 3.0)];
        let steps = place(32, 0.04, 0.99, &bands);

        let dark = steps
            .iter()
            .filter(|&&s| (0.10..=0.25).contains(&s))
            .count();
        let light = steps
            .iter()
            .filter(|&&s| (0.85..=0.99).contains(&s))
            .count();
        let middle = steps
            .iter()
            .filter(|&&s| (0.40..=0.55).contains(&s))
            .count();

        assert!(dark >= 5, "dark surfaces got {dark} steps");
        assert!(light >= 5, "light surfaces got {light} steps");
        assert!(
            middle <= dark && middle <= light,
            "the middle should be sparsest"
        );
    }

    #[test]
    fn degenerate_requests_do_not_panic() {
        assert!(place(0, 0.0, 1.0, &[]).is_empty());
        assert_eq!(place(1, 0.3, 0.9, &[]), vec![0.3]);
        assert_eq!(place(5, 0.5, 0.5, &[]), vec![0.5]);
    }

    #[test]
    fn a_very_dense_ramp_still_behaves() {
        // The brief calls for target apps needing a great many gray steps.
        let bands = [band(0.10, 0.25, 3.0), band(0.85, 0.99, 3.0)];
        let steps = place(256, 0.04, 0.99, &bands);
        assert_eq!(steps.len(), 256);
        for pair in steps.windows(2) {
            assert!(pair[1] > pair[0]);
        }
    }
}
