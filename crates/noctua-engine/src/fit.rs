//! Fitting an existing palette back to spec parameters.
//!
//! Two jobs, and the second one matters more.
//!
//! It lets an existing product's palette become a first-class theme in this
//! system rather than a hardcoded exception — measure it, express it as
//! curves, regenerate it.
//!
//! And it is a **falsifiability test of the model**. If a reasonable
//! real-world palette cannot be expressed within tolerance, then the curve
//! model is too weak, and that is worth knowing. The honest response to a
//! large residual is to publish it, not to special-case the palette.
//!
//! # What is fitted, and what is not
//!
//! Hue and relative chroma. **Not lightness** — each target keeps its own,
//! because the engine solves lightness from contrast targets rather than
//! matching a source. Asking "can these hues and chromas be expressed as
//! curves?" is the question the model can actually be wrong about.
//!
//! # Why Nelder–Mead, hand-rolled
//!
//! It needs no derivatives, which is just as well: the objective runs through
//! a gamut boundary solver and is not differentiable in closed form. It is
//! about 150 lines, and writing it here makes determinism something the code
//! guarantees rather than something a dependency is trusted for.

use noctua_core::{Gamut, Oklch, delta_e_ok};

use crate::curve::Curve;

/// A fitted family, with the evidence for how well it fits.
#[derive(Debug, Clone)]
pub struct Fit {
    /// Hue in degrees at `t = 0`, `t = 0.5` and `t = 1`.
    ///
    /// Three knots rather than two because measurement said so. Fitting the
    /// Tailwind v4 palette with a straight hue line left eight of its
    /// twenty-six families outside a just-noticeable difference, every one of
    /// them in the orange-to-green arc where the authored hue path is a
    /// pronounced S-curve — amber sweeps 95 degrees to 46 with most of the
    /// movement in the middle third. A line through the endpoints misses the
    /// middle by around fifteen degrees.
    pub hue: [f64; 3],
    /// Relative chroma at `t = 0`, at the peak, and at `t = 1`.
    pub chroma: [f64; 3],
    /// Where along the ramp the chroma peak sits.
    ///
    /// Fitted rather than assumed. Pinning it at the spec's default of 0.55
    /// left Tailwind's amber and orange outside a just-noticeable difference
    /// even in Display P3: their chroma crests later than a blue or indigo
    /// ramp does, and a peak in the wrong place cannot be compensated for by
    /// moving the endpoints.
    pub peak_at: f64,
    /// Perceptual error per target color, in Oklab units.
    pub residuals: Vec<f64>,
}

impl Fit {
    /// The largest residual. This is the number that decides whether the model
    /// expressed the palette or merely approximated it.
    #[must_use]
    pub fn worst(&self) -> f64 {
        self.residuals.iter().copied().fold(0.0, f64::max)
    }

    /// The mean residual.
    #[must_use]
    pub fn mean(&self) -> f64 {
        if self.residuals.is_empty() {
            return 0.0;
        }
        self.residuals.iter().sum::<f64>() / self.residuals.len() as f64
    }

    /// Whether every color landed inside a just-noticeable difference.
    ///
    /// At `worst() < 0.02` the fitted palette is not merely close, it is
    /// indistinguishable from the original.
    #[must_use]
    pub fn is_imperceptible(&self) -> bool {
        self.worst() < noctua_core::JND
    }

    /// Whether the ramp had enough colors for the residual to mean much.
    ///
    /// Five parameters are fitted, and each color constrains two of them —
    /// hue and chroma, since lightness is taken from the target. Three colors
    /// therefore give six constraints against five unknowns: solvable, but
    /// only just, and a near-zero residual there is close to arithmetic
    /// rather than evidence. From five colors the system is twice
    /// over-determined and a good fit says something.
    #[must_use]
    pub fn is_well_constrained(&self) -> bool {
        self.residuals.len() >= PARAMETERS
    }

    /// The family's nominal hue: the middle of the ramp.
    #[must_use]
    pub fn base_hue(&self) -> f64 {
        noctua_core::space::normalize_hue(self.hue[1])
    }

    /// Total hue shift from one end of the ramp to the other.
    #[must_use]
    pub fn torsion(&self) -> f64 {
        self.hue[2] - self.hue[0]
    }

    /// How far the middle knot sits from a straight line between the ends.
    ///
    /// Zero means the hue path really is linear and `torsion` describes it
    /// exactly.
    #[must_use]
    pub fn hue_bow(&self) -> f64 {
        self.hue[1] - f64::midpoint(self.hue[0], self.hue[2])
    }

    /// The spec fragment this fit corresponds to, ready to paste.
    ///
    /// A family whose hue path is straight gets the readable `base`/`torsion`
    /// form; one that bends gets explicit knots, because that bend is the
    /// difference between fitting and not.
    #[must_use]
    pub fn to_spec_fragment(&self, name: &str) -> String {
        /// Degrees of bow below which the two forms are indistinguishable.
        const STRAIGHT: f64 = 0.5;

        let hue = if self.hue_bow().abs() < STRAIGHT {
            format!(
                "hue = {{ base = {:.1}, torsion = {:.1} }}",
                self.base_hue(),
                self.torsion()
            )
        } else {
            format!(
                "hue = {{ knots = [[0.0, {:.1}], [0.5, {:.1}], [1.0, {:.1}]] }}",
                noctua_core::space::normalize_hue(self.hue[0]),
                noctua_core::space::normalize_hue(self.hue[1]),
                noctua_core::space::normalize_hue(self.hue[2])
            )
        };

        format!(
            "[families.{name}]\n\
             {hue}\n\
             cr = {{ ends = [{:.2}, {:.2}], peak = {:.2}, peak_at = {:.2} }}\n\
             \n\
             # Fitted from {} colors. Worst residual {:.4}, mean {:.4}.{}\n\
             # A just-noticeable difference is about 0.02.\n",
            self.chroma[0],
            self.chroma[2],
            self.chroma[1],
            self.peak_at,
            self.residuals.len(),
            self.worst(),
            self.mean(),
            if self.is_well_constrained() {
                ""
            } else {
                "\n# Fewer colors than the model has parameters: weak evidence."
            }
        )
    }
}

/// Fits hue and chroma curves to a ramp of colors.
///
/// Targets are taken in ramp order; each keeps its own lightness.
///
/// Returns `None` for fewer than two targets, where a curve is meaningless.
#[must_use]
pub fn fit_family(targets: &[Oklch], gamut: Gamut) -> Option<Fit> {
    if targets.len() < 2 {
        return None;
    }

    // A sensible starting point beats a blind one: the targets' own hues, and
    // their own relative chroma.
    let start = seed(targets, gamut);
    let score = |p: &Params| objective(p, targets, gamut);

    // The objective is not convex, and one start finds one basin. Twice the
    // measurements said so out loud: adding a parameter made Tailwind's amber
    // *worse*, which is impossible for an optimiser that reached the global
    // minimum, since the previous model is a special case of the new one.
    //
    // So: several starts, deterministic ones, keep the best. Nelder-Mead also
    // stagnates when its simplex collapses along an axis, so each start is
    // then restarted from where it landed until it stops improving.
    let mut best = start;
    let mut best_score = f64::INFINITY;

    for offset in STARTS {
        let mut candidate = start;
        for (parameter, shift) in candidate.iter_mut().zip(offset) {
            *parameter += shift;
        }

        for _ in 0..RESTARTS {
            let next = nelder_mead(&candidate, score);
            let improved = score(&candidate) - score(&next);
            candidate = next;
            if improved < TOLERANCE {
                break;
            }
        }

        let candidate_score = score(&candidate);
        if candidate_score < best_score {
            best_score = candidate_score;
            best = candidate;
        }
    }

    Some(build(&best, targets, gamut))
}

/// How many parameters the fit has to play with.
pub const PARAMETERS: usize = 7;

/// Hue at three knots, relative chroma at three, then the position of the
/// chroma peak — in the order the optimiser sees them.
type Params = [f64; PARAMETERS];

fn seed(targets: &[Oklch], gamut: Gamut) -> Params {
    // The targets' own hues at the two ends and the middle. Unwrapped
    // relative to the first, so a ramp crossing 360 degrees seeds as a short
    // path rather than a sweep the long way round.
    let first = targets[0].h;
    let unwrap = |h: f64| first + noctua_core::space::hue_difference(first, h);

    let start = first;
    let middle = unwrap(targets[targets.len() / 2].h);
    let end = unwrap(targets[targets.len() - 1].h);

    let relative: Vec<f64> = targets
        .iter()
        .map(|t| {
            let max = gamut.max_chroma(t.l, t.h);
            if max > 0.0 {
                (t.c / max).clamp(0.0, 1.0)
            } else {
                0.0
            }
        })
        .collect();

    let low = relative.first().copied().unwrap_or(0.2);
    let high = relative.last().copied().unwrap_or(0.4);

    // Seed the peak where the targets actually crest, not at a default.
    let (crest, peak) = relative
        .iter()
        .copied()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or((0, 0.5));
    let peak_at = (crest as f64 / (relative.len() - 1) as f64).clamp(PEAK_RANGE.0, PEAK_RANGE.1);

    [start, middle, end, low, peak, high, peak_at]
}

/// Where the chroma peak may sit.
///
/// Bounded away from the ends so the three knots stay distinct and ordered;
/// a peak at exactly 0 or 1 is an end, and the curve would have two knots at
/// the same `t`.
const PEAK_RANGE: (f64, f64) = (0.05, 0.95);

/// Sum of squared perceptual error.
fn objective(params: &Params, targets: &[Oklch], gamut: Gamut) -> f64 {
    predicted(params, targets, gamut)
        .iter()
        .zip(targets)
        .map(|(p, t)| {
            let error = delta_e_ok(p.to_oklab(), t.to_oklab());
            error * error
        })
        .sum()
}

/// What the fitted curves produce at each target's lightness.
fn predicted(params: &Params, targets: &[Oklch], gamut: Gamut) -> Vec<Oklch> {
    let [h0, h_mid, h1, low, peak, high, peak_at] = *params;
    let hue = Curve::hue([[0.0, h0], [0.5, h_mid], [1.0, h1]]);
    // Knots must stay ordered and distinct, whatever the optimiser tries.
    let at = peak_at.clamp(PEAK_RANGE.0, PEAK_RANGE.1);
    let chroma = Curve::new([[0.0, low], [at, peak], [1.0, high]]);

    let last = (targets.len() - 1) as f64;
    targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let t = index as f64 / last;
            let h = hue.at(t);
            let relative = chroma.at(t).clamp(0.0, 1.0);
            let c = relative * gamut.max_chroma(target.l, h);
            // Lightness is the target's own: this fits hue and chroma, which
            // are what the curve model claims to express.
            Oklch { l: target.l, c, h }
        })
        .collect()
}

fn build(params: &Params, targets: &[Oklch], gamut: Gamut) -> Fit {
    let residuals = predicted(params, targets, gamut)
        .iter()
        .zip(targets)
        .map(|(p, t)| delta_e_ok(p.to_oklab(), t.to_oklab()))
        .collect();

    Fit {
        hue: [params[0], params[1], params[2]],
        chroma: [
            params[3].clamp(0.0, 1.0),
            params[4].clamp(0.0, 1.0),
            params[5].clamp(0.0, 1.0),
        ],
        peak_at: params[6].clamp(PEAK_RANGE.0, PEAK_RANGE.1),
        residuals,
    }
}

// --- Nelder–Mead ----------------------------------------------------------

/// Iterations per pass. Generous: the objective is cheap and a fit runs once.
const ITERATIONS: usize = 2_000;

/// How many times to rebuild the simplex around the current best.
///
/// Stops early once a pass stops improving, so the usual cost is two.
const RESTARTS: usize = 8;

/// Offsets applied to the seed to start from genuinely different places.
///
/// Fixed rather than random, so the fit stays reproducible — a fitted spec
/// fragment gets committed, and a fit that moved between runs would make the
/// spec move with it. The spread is over hue, since that is the parameter
/// the objective is most multi-modal in: a ramp can be fitted going either
/// way around a bend.
const STARTS: [Params; 5] = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, -20.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 20.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -0.30],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.30],
];

/// Stops when the simplex has collapsed to this width.
const TOLERANCE: f64 = 1e-9;

/// Minimises `f`, starting from `start`.
///
/// Deterministic by construction: the initial simplex is built from fixed
/// offsets and every step is arithmetic. The same input always produces the
/// same fit, which is what lets a fitted spec fragment be committed.
fn nelder_mead(start: &Params, f: impl Fn(&Params) -> f64) -> Params {
    const N: usize = PARAMETERS;
    // Per-parameter step sizes. Degrees and unit fractions are not the same
    // scale, and one shared step would make the optimiser crawl in hue and
    // thrash in chroma.
    const STEPS: Params = [10.0, 10.0, 10.0, 0.08, 0.08, 0.08, 0.10];

    let mut simplex: Vec<(Params, f64)> = Vec::with_capacity(N + 1);
    simplex.push((*start, f(start)));
    for i in 0..N {
        let mut point = *start;
        point[i] += STEPS[i];
        let value = f(&point);
        simplex.push((point, value));
    }

    for _ in 0..ITERATIONS {
        simplex.sort_by(|a, b| a.1.total_cmp(&b.1));

        if (simplex[N].1 - simplex[0].1).abs() < TOLERANCE {
            break;
        }

        // Centroid of everything but the worst point.
        let mut centroid = [0.0; N];
        for (point, _) in &simplex[..N] {
            for i in 0..N {
                centroid[i] += point[i] / N as f64;
            }
        }

        let worst = simplex[N].0;
        let combine = |a: &Params, b: &Params, t: f64| -> Params {
            let mut out = [0.0; N];
            for i in 0..N {
                out[i] = a[i] + t * (a[i] - b[i]);
            }
            out
        };

        let reflected = combine(&centroid, &worst, 1.0);
        let reflected_value = f(&reflected);

        if reflected_value < simplex[0].1 {
            // Better than the best: try going further.
            let expanded = combine(&centroid, &worst, 2.0);
            let expanded_value = f(&expanded);
            simplex[N] = if expanded_value < reflected_value {
                (expanded, expanded_value)
            } else {
                (reflected, reflected_value)
            };
        } else if reflected_value < simplex[N - 1].1 {
            simplex[N] = (reflected, reflected_value);
        } else {
            let contracted = combine(&centroid, &worst, -0.5);
            let contracted_value = f(&contracted);
            if contracted_value < simplex[N].1 {
                simplex[N] = (contracted, contracted_value);
            } else {
                // Nothing worked: shrink everything toward the best point.
                let best = simplex[0].0;
                for entry in simplex.iter_mut().skip(1) {
                    let mut point = [0.0; N];
                    for i in 0..N {
                        point[i] = best[i] + 0.5 * (entry.0[i] - best[i]);
                    }
                    *entry = (point, f(&point));
                }
            }
        }
    }

    simplex.sort_by(|a, b| a.1.total_cmp(&b.1));
    simplex[0].0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a ramp from known curve parameters, so a fit can be checked
    /// against the answer rather than against a guess.
    fn synthetic(base: f64, torsion: f64, chroma: [f64; 3], steps: usize) -> Vec<Oklch> {
        let hue = Curve::hue([[0.0, base - torsion / 2.0], [1.0, base + torsion / 2.0]]);
        let cr = Curve::new([[0.0, chroma[0]], [0.55, chroma[1]], [1.0, chroma[2]]]);
        // The fitter must recover this position, not assume it.
        (0..steps)
            .map(|i| {
                let t = i as f64 / (steps - 1) as f64;
                let l = 0.95 - 0.7 * t;
                let h = hue.at(t);
                let c = cr.at(t) * Gamut::Srgb.max_chroma(l, h);
                Oklch { l, c, h }
            })
            .collect()
    }

    #[test]
    fn a_ramp_the_model_generated_is_recovered_exactly() {
        // The floor: if the fitter cannot recover its own output, nothing it
        // says about a real palette means anything.
        let targets = synthetic(264.0, -14.0, [0.15, 0.9, 0.4], 12);
        let fit = fit_family(&targets, Gamut::Srgb).expect("fits");

        assert!(
            fit.is_imperceptible(),
            "worst residual {:.5} on a ramp this model produced",
            fit.worst()
        );
        assert!(
            (fit.base_hue() - 264.0).abs() < 2.0,
            "hue {:.1}",
            fit.base_hue()
        );
    }

    #[test]
    fn a_short_ramp_admits_it_proves_little() {
        let short = synthetic(200.0, 0.0, [0.2, 0.6, 0.3], 3);
        let fit = fit_family(&short, Gamut::Srgb).expect("fits");
        assert!(!fit.is_well_constrained(), "3 colors against 5 parameters");
        assert!(
            fit.to_spec_fragment("short").contains("weak evidence"),
            "a fragment must not present an under-determined fit as a result"
        );

        let long = synthetic(200.0, 0.0, [0.2, 0.6, 0.3], 12);
        let fit = fit_family(&long, Gamut::Srgb).expect("fits");
        assert!(fit.is_well_constrained());
        assert!(!fit.to_spec_fragment("long").contains("weak evidence"));
    }

    #[test]
    fn fitting_is_deterministic() {
        let targets = synthetic(30.0, 6.0, [0.2, 0.8, 0.35], 12);
        let first = fit_family(&targets, Gamut::Srgb).expect("fits");
        let second = fit_family(&targets, Gamut::Srgb).expect("fits");
        assert_eq!(first.residuals, second.residuals);
        assert!((first.base_hue() - second.base_hue()).abs() < 1e-12);
    }

    #[test]
    fn a_ramp_across_the_seam_seeds_the_short_way_round() {
        // 350 to 10 degrees is a 20-degree step, not a 340-degree one. Seeded
        // the long way, the optimiser starts on the far side of the wheel.
        let targets = vec![
            Oklch {
                l: 0.7,
                c: 0.1,
                h: 350.0,
            },
            Oklch {
                l: 0.6,
                c: 0.1,
                h: 0.0,
            },
            Oklch {
                l: 0.5,
                c: 0.1,
                h: 10.0,
            },
        ];
        let seeded = seed(&targets, Gamut::Srgb);
        assert!(
            (seeded[2] - seeded[0]).abs() < 30.0,
            "seeded a {:.0}-degree sweep for a 20-degree ramp",
            (seeded[2] - seeded[0]).abs()
        );
    }

    /// The measurement that put the middle knot there in the first place.
    #[test]
    fn a_bowed_hue_path_is_fitted_and_reported_as_knots() {
        // Amber's shape: mostly flat, then a steep drop, then flat again.
        // A straight line between the ends misses the middle badly.
        let bowed = Curve::hue([[0.0, 95.3], [0.5, 84.4], [1.0, 45.6]]);
        let targets: Vec<Oklch> = (0..11)
            .map(|i| {
                let t = f64::from(i) / 10.0;
                let l = 0.97 - 0.72 * t;
                let h = bowed.at(t);
                Oklch {
                    l,
                    c: 0.55 * Gamut::Srgb.max_chroma(l, h),
                    h,
                }
            })
            .collect();

        let fit = fit_family(&targets, Gamut::Srgb).expect("fits");
        assert!(
            fit.is_imperceptible(),
            "a bowed hue path must be expressible; worst {:.4}",
            fit.worst()
        );
        assert!(
            fit.hue_bow().abs() > 0.5,
            "the bow is real: {:.1}",
            fit.hue_bow()
        );
        assert!(
            fit.to_spec_fragment("amber").contains("knots"),
            "a bowed path must be emitted as knots, not as torsion"
        );
    }

    /// The hard shape, built from parameters rather than borrowed colors: a
    /// bowed hue path *and* a chroma peak far from the default position.
    ///
    /// Each of these alone was enough to push a real family outside a
    /// just-noticeable difference. Together they are what the fitter has to
    /// handle, and a regression in either the middle hue knot, the fitted
    /// peak position, or the multi-start will fail here.
    #[test]
    fn a_bowed_hue_and_a_late_chroma_peak_are_both_recovered() {
        let hue = Curve::hue([[0.0, 95.0], [0.5, 84.0], [1.0, 46.0]]);
        let cr = Curve::new([[0.0, 0.30], [0.82, 0.95], [1.0, 0.55]]);

        let targets: Vec<Oklch> = (0..11)
            .map(|i| {
                let t = f64::from(i) / 10.0;
                let l = 0.97 - 0.70 * t;
                let h = hue.at(t);
                Oklch {
                    l,
                    c: cr.at(t) * Gamut::Srgb.max_chroma(l, h),
                    h,
                }
            })
            .collect();

        let fit = fit_family(&targets, Gamut::Srgb).expect("fits");
        assert!(
            fit.is_imperceptible(),
            "worst {:.4} on a shape the model can express exactly",
            fit.worst()
        );
        assert!(
            fit.peak_at > 0.65,
            "the chroma peak was authored at 0.82 and fitted at {:.2}",
            fit.peak_at
        );
    }

    /// Relative chroma is a fraction of what the gamut can show, so it cannot
    /// exceed 1 — and a palette authored for a wider gamut therefore cannot
    /// be expressed against a narrower one. That is a property of the model,
    /// not a defect in the fitter, and the residual has to show it.
    #[test]
    fn a_palette_authored_beyond_the_gamut_cannot_be_expressed_within_it() {
        // Built in Display P3 at near-full chroma, then measured against sRGB.
        let targets: Vec<Oklch> = (0..9)
            .map(|i| {
                let t = f64::from(i) / 8.0;
                let l = 0.90 - 0.55 * t;
                let h = 85.0 - 40.0 * t;
                Oklch {
                    l,
                    c: 0.98 * Gamut::DisplayP3.max_chroma(l, h),
                    h,
                }
            })
            .collect();

        let in_p3 = fit_family(&targets, Gamut::DisplayP3).expect("fits");
        let in_srgb = fit_family(&targets, Gamut::Srgb).expect("fits");

        assert!(
            in_p3.is_imperceptible(),
            "worst {:.4} in its own gamut",
            in_p3.worst()
        );
        assert!(
            in_srgb.worst() > in_p3.worst(),
            "measuring a P3 palette against sRGB must cost something: {:.4} vs {:.4}",
            in_srgb.worst(),
            in_p3.worst()
        );
    }

    #[test]
    fn a_straight_hue_path_keeps_the_readable_form() {
        let straight = synthetic(264.0, -14.0, [0.15, 0.9, 0.4], 12);
        let fit = fit_family(&straight, Gamut::Srgb).expect("fits");
        let fragment = fit.to_spec_fragment("accent");
        assert!(fragment.contains("torsion"), "{fragment}");
        assert!(!fragment.contains("knots"), "{fragment}");
    }

    #[test]
    fn too_few_colors_is_not_a_fit() {
        assert!(fit_family(&[], Gamut::Srgb).is_none());
        assert!(
            fit_family(
                &[Oklch {
                    l: 0.5,
                    c: 0.1,
                    h: 200.0
                }],
                Gamut::Srgb
            )
            .is_none()
        );
    }

    #[test]
    fn the_fragment_is_pasteable_and_reports_its_own_error() {
        let targets = synthetic(120.0, 0.0, [0.2, 0.7, 0.3], 8);
        let fit = fit_family(&targets, Gamut::Srgb).expect("fits");
        let fragment = fit.to_spec_fragment("imported");

        assert!(fragment.contains("[families.imported]"));
        assert!(fragment.contains("hue = { base ="));
        assert!(fragment.contains("cr = { ends ="));
        assert!(
            fragment.contains("Worst residual"),
            "a fit must publish its error"
        );

        // And it must actually parse as spec.
        let spec = noctua_spec::parse("fitted.toml", &fragment).expect("valid spec fragment");
        assert!(spec.families.contains_key("imported"));
    }

    /// A ramp with no consistent hue cannot be expressed, and the fitter must
    /// say so rather than return a confident wrong answer.
    #[test]
    fn an_unexpressible_ramp_reports_a_large_residual() {
        let scattered: Vec<Oklch> = [10.0, 140.0, 260.0, 40.0, 200.0, 320.0]
            .into_iter()
            .enumerate()
            .map(|(i, h)| {
                let l = 0.8 - 0.08 * i as f64;
                Oklch {
                    l,
                    c: 0.7 * Gamut::Srgb.max_chroma(l, h),
                    h,
                }
            })
            .collect();

        let fit = fit_family(&scattered, Gamut::Srgb).expect("fits");
        assert!(
            !fit.is_imperceptible(),
            "six unrelated hues should not fit a single hue curve; got {:.4}",
            fit.worst()
        );
    }
}
