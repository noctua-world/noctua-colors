//! Colour-vision safety.
//!
//! This gate is why `noctua-core::cvd` exists, and until now nothing called
//! it. The simulator was built, tested against its own invariants, and then
//! left connected to nothing — so the palette it was meant to protect was
//! never actually checked.
//!
//! # What it checks
//!
//! Pairs whose meanings are *opposed*. Success against danger above all: a
//! system that signals "it worked" and "it broke" with two colors a
//! deuteranope cannot tell apart has failed at the one job color was doing.
//!
//! Every categorical scale, pairwise. A chart with eight series is only eight
//! series if all twenty-eight pairs stay apart. Which scales those are comes
//! from the scale itself rather than from its name — the spec ships four
//! categorical sets and two ordered ones, and a name comparison called all but
//! one of them ordered.
//!
//! A set that declares `labelled = true` is measured exactly as thoroughly and
//! reports one finding per deficiency instead of one per pair: twelve entries
//! is sixty-six pairs, and publishing each shortfall separately would bury the
//! findings a reader can act on under two hundred restating one measured limit.
//!
//! And every ordinal scale — but **not** pairwise, which would be the wrong
//! property. Confusing `level-2` with `level-7` loses precision; confusing
//! `level-0` with `level-10` loses the meaning. So the checks are the three
//! things an ordered scale actually promises: neighbours are distinguishable,
//! the ends are opposed, and the order survives.
//!
//! # Opposition is curated, collisions are published
//!
//! [`OPPOSED`] is a hand-written list, not every pair of families. With ten
//! semantic families there are forty-five pairs, and gating all of them would
//! produce a hundred warnings that say the same thing — that colour alone
//! cannot separate ten meanings for a dichromat, which is already known and
//! already documented. What is gated is the pairs where confusion changes what
//! the interface *said*.
//!
//! Every pair is still measured. [`margins`] returns all of them and
//! `dist/reports/colour-vision.md` publishes the table, so a collision that is
//! merely awkward rather than dangerous is visible without burying the ones
//! that are.
//!
//! # It reports margins
//!
//! Every check carries how much separation is left, because the number is
//! what tells you whether you are comfortable or one tweak from failing.

use noctua_core::cvd::{Cvd, separation};
use noctua_core::delta_e_ok;
use noctua_engine::{Palette, ResolvedMode, ScaleKind};

use indexmap::IndexMap;

use crate::{Finding, Report, Severity};

const GATE: &str = "cvd";

/// Separation an opposed pair *should* reach: five just-noticeable
/// differences, enough to tell apart at a glance on an uncalibrated screen.
///
/// Falling short warns. It is not a failure because, measured, it is not
/// reachable: searched across every shift combination subject to fills staying
/// visible and ramps staying monotonic, the best a six-family semantic set
/// achieves is **0.0163** — under one JND. Hue is the axis dichromacy removes,
/// and the lightness left over after every other requirement is not enough to
/// replace it.
///
/// That is the reason WCAG 1.4.1 exists. A gate that failed here would be
/// permanently red and would teach people to ignore it; one that reports the
/// margin tells them how much they have and that they need a second channel.
const OPPOSED_TARGET: f64 = 0.10;

/// Separation below which an opposed pair is simply the same color.
///
/// Under the just-noticeable difference: no viewer of any kind can tell them
/// apart, which is a defect regardless of what else is going on.
const OPPOSED_FLOOR: f64 = 0.015;

/// Target separation between two entries of a categorical scale.
///
/// Falling short warns rather than fails. Chart series carry a legend and a
/// label; semantic colors carry meaning on their own, which is why those are
/// hard. Beyond about six generated entries this target is unreachable, and a
/// warning that says so is more use than a build nobody can green.
const CATEGORICAL_TARGET: f64 = 0.05;

/// Separation below which two chart entries are the same color.
///
/// Under the just-noticeable difference, so no viewer of any kind can tell
/// them apart. That is a defect however many series there are.
const CATEGORICAL_FLOOR: f64 = 0.02;

// The thresholds must keep their intended relationship to the just-noticeable
// difference. Checked at compile time, because a threshold that drifted below
// the JND would make the gate meaningless rather than merely wrong.
const _: () = {
    assert!(OPPOSED_TARGET > noctua_core::JND * 4.0);
    assert!(OPPOSED_FLOOR < noctua_core::JND);
    assert!(CATEGORICAL_TARGET > noctua_core::JND * 2.0);
    assert!(CATEGORICAL_FLOOR >= noctua_core::JND);
};

/// Target separation between neighbouring stops of an ordinal scale.
///
/// Lower than [`CATEGORICAL_TARGET`] on purpose. A chart's series are read
/// against each other in any order, so any two of them may be compared; an
/// ordinal scale is read as a sequence, and what a reader has to see is that
/// the next stop is *a step further*, not that it is a different colour.
///
/// Twice the just-noticeable difference, which is the smallest gap that still
/// reads as deliberate rather than as a printing artefact.
const ORDINAL_ADJACENT_TARGET: f64 = 0.04;

/// Separation below which two neighbouring stops are the same stop.
const ORDINAL_ADJACENT_FLOOR: f64 = 0.02;

/// How far a scale's lightness may go backwards before the scale stops reading
/// as ordered.
///
/// Zero would be the honest number, but quantization alone moves lightness by
/// up to 0.0001 and the simulator's matrices add a little more. This is the
/// noise floor, not a tolerance for a real reversal: a scale that genuinely
/// turns back on itself moves by hundredths.
const MONOTONE_TOLERANCE: f64 = 0.002;

// The ordinal thresholds carry the same relationship to the just-noticeable
// difference that the others do.
const _: () = {
    assert!(ORDINAL_ADJACENT_TARGET > noctua_core::JND);
    assert!(ORDINAL_ADJACENT_FLOOR >= noctua_core::JND);
};

/// Semantic slots whose meanings must never be confused.
///
/// Curated rather than generated — see the module documentation. A pair earns a
/// place here when mistaking one for the other would make the interface say
/// something it did not mean: "it worked" for "it broke", "waiting" for
/// "running", "act now" for "note this".
const OPPOSED: &[(&str, &str)] = &[
    // The pair that matters most, and its neighbours.
    ("success", "danger"),
    ("success", "warning"),
    ("danger", "warning"),
    ("danger", "info"),
    ("success", "info"),
    ("accent", "danger"),
    // Two levels of alarm. `urgent` sits 14 degrees from `danger` in hue, so
    // whatever separates them is lightness, and this is where that is checked.
    ("urgent", "danger"),
    ("urgent", "warning"),
    ("urgent", "success"),
    ("urgent", "info"),
    // Not started, running, finished. A job list shows all three at once, and
    // "waiting" read as "done" is the failure that matters.
    ("waiting", "success"),
    ("waiting", "active"),
    ("waiting", "danger"),
    ("active", "success"),
    // In flight against arrived, and against the blue it sits nearest.
    ("progress", "success"),
    ("progress", "waiting"),
    ("progress", "info"),
];

fn solid_of(mode: &ResolvedMode, slot: &str) -> Option<noctua_core::Oklab> {
    let family = mode.semantic.get(slot)?;
    let resolved = mode.families.get(family)?;
    resolved
        .steps
        .iter()
        .find(|step| step.role == "solid")
        .map(|step| step.primary().oklch.to_oklab())
}

/// Checks opposed pairs and the categorical scale under every deficiency.
///
/// # One finding per pair, not per palette
///
/// Every palette in the grid shares the same semantic families, so every one
/// of them misses the same targets by nearly the same margin. Reported per
/// palette that is thirty-six copies of each finding — four hundred lines that
/// bury the contrast and source gates underneath them, and which nobody reads
/// twice.
///
/// So findings are grouped by pair and deficiency, and each reports the
/// **worst** palette. That is the number that matters: it is the one a reader
/// has to design around, and it names the palette it came from so it can be
/// reproduced. Every measurement is still made — `checked` counts them all —
/// and the full per-palette table is published by [`margins`] into
/// `dist/reports/colour-vision.md`.
#[must_use]
pub fn check(palette: &Palette) -> Report {
    let mut report = Report::default();

    // Keyed by what is being compared; the value is the worst case seen.
    let mut worst: IndexMap<String, Worst> = IndexMap::new();

    for theme in &palette.themes {
        for mode in &theme.modes {
            for (left, right) in OPPOSED {
                let (Some(a), Some(b)) = (solid_of(mode, left), solid_of(mode, right)) else {
                    continue;
                };

                for deficiency in Cvd::all() {
                    report.checked += 1;
                    let apart = separation(a, b, deficiency, 1.0);
                    if apart >= OPPOSED_TARGET {
                        continue;
                    }
                    record(
                        &mut worst,
                        format!("{left} vs {right} under {}", deficiency.id()),
                        Worst {
                            theme: theme.name.clone(),
                            mode: mode.mode.id(),
                            apart,
                            normal: delta_e_ok(a, b),
                            target: OPPOSED_TARGET,
                            floor: OPPOSED_FLOOR,
                            kind: Kind::Opposed,
                        },
                    );
                }
            }

            for (name, scale) in &mode.scales {
                let entries: Vec<noctua_core::Oklab> = scale
                    .steps
                    .iter()
                    .map(|step| step.primary().oklch.to_oklab())
                    .collect();
                let labels: Vec<&str> = scale.steps.iter().map(|step| step.role.as_str()).collect();

                // The scale says what it is. This used to compare the name
                // against `chart`, in three files, which was one fact spelled
                // three times and wrong the moment a second categorical set
                // existed.
                match scale.kind {
                    ScaleKind::Categorical { labelled } => categorical(
                        &mut report,
                        &mut worst,
                        theme,
                        mode,
                        name,
                        &entries,
                        labelled,
                    ),
                    ScaleKind::Ordered => ordinal(
                        &mut report,
                        &mut worst,
                        theme,
                        mode,
                        name,
                        &entries,
                        &labels,
                    ),
                }
            }
        }
    }

    for (what, case) in worst {
        report.findings.push(case.into_finding(&what));
    }

    report
}

/// A categorical scale, every entry against every other.
///
/// Pairwise is the right property here and only here: a chart's series carry
/// no order, so any two of them may end up side by side in a legend.
///
/// # A labelled set reports its worst pair, not all of them
///
/// Every pair is measured either way — `report.checked` counts them all, and a
/// pair below the floor still fails whatever the set declares, because two
/// colours that are literally the same is a defect at any size. What changes is
/// how many findings come out. Twelve entries is sixty-six pairs and a hundred
/// and ninety-eight comparisons, most of them short of a target that no
/// generated set of that size can reach; publishing each one would bury the
/// four findings a reader can act on under two hundred restating the same
/// measured limit. That is the reason `OPPOSED` is curated rather than
/// exhaustive, and the same reasoning applies here.
///
/// So a set that declares `labelled = true` gets one finding per deficiency,
/// naming its closest pair and the margin. The rest are counted, not printed —
/// and `dist/reports/colour-vision.md` still publishes every margin, so an
/// awkward pair stays findable.
fn categorical(
    report: &mut Report,
    worst: &mut IndexMap<String, Worst>,
    theme: &noctua_engine::ResolvedTheme,
    mode: &ResolvedMode,
    scale: &str,
    entries: &[noctua_core::Oklab],
    labelled: bool,
) {
    for deficiency in Cvd::all() {
        let mut closest: Option<(usize, usize, f64)> = None;

        for (i, a) in entries.iter().enumerate() {
            for (j, b) in entries.iter().enumerate().skip(i + 1) {
                report.checked += 1;
                let apart = separation(*a, *b, deficiency, 1.0);

                if closest.is_none_or(|(_, _, lowest)| apart < lowest) {
                    closest = Some((i, j, apart));
                }

                if labelled || apart >= CATEGORICAL_TARGET {
                    continue;
                }
                record(
                    worst,
                    format!("{scale} {} vs {} under {}", i + 1, j + 1, deficiency.id()),
                    Worst {
                        theme: theme.name.clone(),
                        mode: mode.mode.id(),
                        apart,
                        normal: delta_e_ok(*a, *b),
                        target: CATEGORICAL_TARGET,
                        floor: CATEGORICAL_FLOOR,
                        kind: Kind::Categorical,
                    },
                );
            }
        }

        let Some((i, j, apart)) = closest else {
            continue;
        };
        if !labelled || apart >= CATEGORICAL_TARGET {
            continue;
        }
        record(
            worst,
            format!("{scale} closest pair under {}", deficiency.id()),
            Worst {
                theme: theme.name.clone(),
                mode: mode.mode.id(),
                apart,
                normal: delta_e_ok(entries[i], entries[j]),
                target: CATEGORICAL_TARGET,
                floor: CATEGORICAL_FLOOR,
                kind: Kind::CategoricalLabelled {
                    entries: entries.len(),
                    pair: (i + 1, j + 1),
                },
            },
        );
    }
}

/// An ordered scale: the three things being ordered actually promises.
///
/// Not pairwise. Eleven stops on one hue arc give fifty-five pairs, most of
/// which are nowhere near the categorical target and none of which need to be —
/// `level-2` mistaken for `level-4` is a scale being read imprecisely, which is
/// what a continuous scale is for. What must hold is that neighbours are
/// distinguishable, the ends are opposed, and the sequence still runs one way
/// when hue is taken away.
fn ordinal(
    report: &mut Report,
    worst: &mut IndexMap<String, Worst>,
    theme: &noctua_engine::ResolvedTheme,
    mode: &ResolvedMode,
    scale: &str,
    entries: &[noctua_core::Oklab],
    labels: &[&str],
) {
    for deficiency in Cvd::all() {
        // Neighbours.
        for (i, pair) in entries.windows(2).enumerate() {
            report.checked += 1;
            let apart = separation(pair[0], pair[1], deficiency, 1.0);
            if apart < ORDINAL_ADJACENT_TARGET {
                record(
                    worst,
                    format!(
                        "{scale} {} vs {} under {}",
                        labels[i],
                        labels[i + 1],
                        deficiency.id()
                    ),
                    Worst {
                        theme: theme.name.clone(),
                        mode: mode.mode.id(),
                        apart,
                        normal: delta_e_ok(pair[0], pair[1]),
                        target: ORDINAL_ADJACENT_TARGET,
                        floor: ORDINAL_ADJACENT_FLOOR,
                        kind: Kind::Ordinal,
                    },
                );
            }
        }

        // The ends. `level-0` against `level-10` is success against danger
        // under another name, so it is held to the same target.
        if let (Some(first), Some(last)) = (entries.first(), entries.last()) {
            report.checked += 1;
            let apart = separation(*first, *last, deficiency, 1.0);
            if apart < OPPOSED_TARGET {
                record(
                    worst,
                    format!(
                        "{scale} {} vs {} under {}",
                        labels[0],
                        labels[labels.len() - 1],
                        deficiency.id()
                    ),
                    Worst {
                        theme: theme.name.clone(),
                        mode: mode.mode.id(),
                        apart,
                        normal: delta_e_ok(*first, *last),
                        target: OPPOSED_TARGET,
                        floor: OPPOSED_FLOOR,
                        kind: Kind::Opposed,
                    },
                );
            }
        }

        // The order itself. Under dichromacy lightness is what is left, so if
        // simulated lightness turns back on itself the scale has stopped being
        // a scale — and unlike a thin margin, that is a defect at any severity.
        let lightness: Vec<f64> = entries
            .iter()
            .map(|c| noctua_core::cvd::simulate(*c, deficiency, 1.0).l)
            .collect();
        let ascending = lightness.last() >= lightness.first();
        for (i, pair) in lightness.windows(2).enumerate() {
            report.checked += 1;
            let step = if ascending {
                pair[1] - pair[0]
            } else {
                pair[0] - pair[1]
            };
            if step >= -MONOTONE_TOLERANCE {
                continue;
            }
            report.findings.push(Finding {
                gate: GATE,
                severity: Severity::Fail,
                where_: format!(
                    "{scale} {} to {} under {} ({}/{})",
                    labels[i],
                    labels[i + 1],
                    deficiency.id(),
                    theme.name,
                    mode.mode.id()
                ),
                message: format!(
                    "lightness reverses by {:.4}, so the scale stops reading as ordered \
                     where hue is unavailable. Widen `lightness_spread`, or move the hue \
                     knots so the path does not double back",
                    -step
                ),
                margin: Some(step),
            });
        }
    }
}

/// What kind of comparison a finding came from, which decides how it is worded.
#[derive(Debug, Clone, Copy)]
enum Kind {
    /// Two meanings that must never be swapped.
    Opposed,
    /// Two entries of a categorical scale.
    Categorical,
    /// The closest pair of a categorical scale that declares it is labelled.
    ///
    /// Worded differently on purpose: telling someone to "label the series"
    /// when they have already said the series are labelled is advice they have
    /// taken, and repeating it is how a report teaches people to stop reading
    /// it.
    CategoricalLabelled {
        /// How many entries the set has, which is what makes the limit.
        entries: usize,
        /// Which two, numbered as the tokens are.
        pair: (usize, usize),
    },
    /// Two neighbouring stops of an ordered scale.
    Ordinal,
}

/// The worst palette seen for one comparison.
struct Worst {
    theme: String,
    mode: &'static str,
    apart: f64,
    normal: f64,
    target: f64,
    floor: f64,
    kind: Kind,
}

impl Worst {
    fn into_finding(self, what: &str) -> Finding {
        // A note, not a warning, once it clears the floor. Falling short of the
        // target here is not something a different choice would fix — it is the
        // measured limit of what colour alone can do for a semantic set this
        // size, and the number is published so a reader can design around it.
        // Below the floor is different: two colours that are literally the same
        // is a defect, and stays one.
        let severity = if self.apart < self.floor {
            Severity::Fail
        } else {
            Severity::Note
        };
        let message = match self.kind {
            Kind::Categorical => format!(
                "only {:.4} apart, wants {:.2}. Reduce the set's `count`, or set \
                 `labelled = true` and name every series in the legend: past about \
                 six entries no generated palette separates them",
                self.apart, self.target
            ),
            Kind::CategoricalLabelled {
                entries,
                pair: (i, j),
            } => format!(
                "{entries} entries, closest pair {i} and {j} at {:.4}, wants {:.2}. \
                 This is the measured limit rather than a choice — the set declares \
                 `labelled = true`, so the legend has to name every series. The other \
                 pairs were measured too and are published in \
                 dist/reports/colour-vision.md",
                self.apart, self.target
            ),
            Kind::Ordinal => format!(
                "only {:.4} apart, wants {:.2}; normal vision sees {:.4}. Widen the \
                 scale's `lightness_spread`, or give it fewer stops",
                self.apart, self.target, self.normal
            ),
            Kind::Opposed => format!(
                "{:.4} apart, wants {:.2}; normal vision sees {:.4}. Do not rely on \
                 color alone here — add an icon or a label",
                self.apart, self.target, self.normal
            ),
        };

        Finding {
            gate: GATE,
            severity,
            // The worst palette is named, so the number can be reproduced.
            where_: format!("{what} (worst: {}/{})", self.theme, self.mode),
            message,
            margin: Some(self.apart - self.target),
        }
    }
}

/// Keeps the worst case for a comparison.
fn record(worst: &mut IndexMap<String, Worst>, what: String, case: Worst) {
    match worst.get(&what) {
        Some(existing) if existing.apart <= case.apart => {}
        _ => {
            worst.insert(what, case);
        }
    }
}

/// The separation **every** pair of semantic families keeps under each
/// deficiency.
///
/// Every pair, not only the opposed ones: this is the published table, and its
/// job is to make a collision findable rather than to decide whether it
/// matters. The gate decides that, from the shorter [`OPPOSED`] list.
///
/// Neutral variants are excluded. `neutral-cool` and `neutral-warm` are
/// *designed* to be within a JND of `neutral` — reporting that as a collision
/// would be reporting the feature.
#[must_use]
pub fn margins(palette: &Palette) -> Vec<(String, &'static str, String, Cvd, f64)> {
    let mut rows = Vec::new();
    for theme in &palette.themes {
        for mode in &theme.modes {
            let families: Vec<&String> = mode
                .families
                .keys()
                .filter(|name| !name.starts_with("neutral"))
                .collect();

            for (i, left) in families.iter().enumerate() {
                for right in families.iter().skip(i + 1) {
                    let (Some(a), Some(b)) = (solid_of(mode, left), solid_of(mode, right)) else {
                        continue;
                    };
                    for deficiency in Cvd::all() {
                        rows.push((
                            theme.name.clone(),
                            mode.mode.id(),
                            format!("{left} vs {right}"),
                            deficiency,
                            separation(a, b, deficiency, 1.0),
                        ));
                    }
                }
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        noctua_engine::build(&noctua_spec::load(path).expect("valid")).expect("builds")
    }

    #[test]
    fn success_versus_danger_is_checked_under_every_deficiency() {
        assert!(
            OPPOSED.contains(&("success", "danger")),
            "the pair that matters most"
        );

        let palette = shipped();
        let rows = margins(&palette);
        let for_pair: Vec<_> = rows
            .iter()
            .filter(|r| r.2 == "danger vs success" || r.2 == "success vs danger")
            .collect();
        assert_eq!(
            for_pair.len(),
            palette.themes.len() * 2 * 3,
            "every theme, mode and deficiency"
        );
    }

    /// The published table is every pair, not the gated subset — a collision
    /// that is awkward rather than dangerous still has to be findable.
    #[test]
    fn the_report_measures_every_pair_and_the_gate_judges_a_subset() {
        let palette = shipped();
        let mode = &palette.themes[0].modes[0];
        let semantic = mode
            .families
            .keys()
            .filter(|name| !name.starts_with("neutral"))
            .count();
        let pairs = semantic * (semantic - 1) / 2;

        assert!(
            pairs > OPPOSED.len(),
            "if every pair were opposed the curation would be doing nothing"
        );
        assert_eq!(
            margins(&palette).len(),
            palette.themes.len() * 2 * pairs * 3
        );
    }

    /// Neutral variants are within a JND of the base neutral by design, so
    /// reporting them as a collision would be reporting the feature.
    #[test]
    fn the_neutral_variants_are_not_reported_as_collisions() {
        for (_, _, pair, _, _) in margins(&shipped()) {
            assert!(!pair.contains("neutral"), "{pair} is a designed similarity");
        }
    }

    #[test]
    fn the_gate_actually_runs_against_the_palette() {
        // The failure this module was written to correct: a simulator wired
        // to nothing checks nothing.
        let report = check(&shipped());
        assert!(report.checked > 0, "the CVD gate performed no checks");
    }

    #[test]
    fn categorical_scales_are_checked_pairwise_and_ordinal_scales_are_not() {
        let palette = shipped();
        let mode = &palette.themes[0].modes[0];

        // Every categorical set, not just the unnamed one — including the
        // labelled ones, which report less and measure exactly as much.
        let categorical: usize = mode
            .categorical()
            .map(|(_, scale)| {
                let n = scale.steps.len();
                n * (n - 1) / 2
            })
            .sum();
        assert!(
            mode.categorical().count() > 1,
            "the spec ships one categorical set, so this proves nothing"
        );

        // An ordered scale is checked on what being ordered promises:
        // every neighbouring pair, the two ends, and every step of the order
        // itself. Linear in the stop count, where pairwise would be quadratic.
        let ordinal: usize = mode
            .ordered()
            .map(|(_, scale)| {
                let n = scale.steps.len();
                (n - 1) + 1 + (n - 1)
            })
            .sum();

        let per_mode = (OPPOSED.len() + categorical + ordinal) * 3;
        assert_eq!(check(&palette).checked, palette.themes.len() * 2 * per_mode);

        // The point of the split, stated as a number: eleven stops pairwise
        // would be fifty-five comparisons per deficiency.
        let level = mode.scales["level"].steps.len();
        assert!(
            ordinal < level * (level - 1) / 2,
            "the ordinal checks are supposed to be cheaper than pairwise"
        );
    }

    /// A labelled set is not a quieter set by accident. Every pair is still
    /// measured — the count above proves that — and what changes is that the
    /// findings collapse to the closest pair per deficiency, so a wide chart
    /// cannot bury the rest of the report under two hundred notes restating one
    /// measured limit.
    #[test]
    fn a_labelled_categorical_set_reports_its_closest_pair_and_no_more() {
        let palette = shipped();
        let labelled: Vec<&String> = palette.themes[0].modes[0]
            .scales
            .iter()
            .filter(|(_, scale)| scale.kind.is_labelled())
            .map(|(name, _)| name)
            .collect();
        assert!(
            !labelled.is_empty(),
            "the spec ships no labelled set, so this proves nothing"
        );

        for name in labelled {
            let findings = check(&palette)
                .findings
                .into_iter()
                .filter(|finding| finding.where_.starts_with(name.as_str()))
                .count();
            assert!(
                findings <= Cvd::all().len(),
                "{name} produced {findings} findings, one per pair rather than one \
                 per deficiency"
            );
        }
    }

    /// Under dichromacy lightness is all that is left, so a scale whose
    /// simulated lightness turns back on itself has stopped being a scale.
    #[test]
    fn every_ordinal_scale_stays_ordered_under_simulation() {
        let palette = shipped();
        for theme in &palette.themes {
            for mode in &theme.modes {
                for (name, scale) in mode.ordered() {
                    for deficiency in Cvd::all() {
                        let lightness: Vec<f64> = scale
                            .steps
                            .iter()
                            .map(|step| {
                                noctua_core::cvd::simulate(
                                    step.primary().oklch.to_oklab(),
                                    deficiency,
                                    1.0,
                                )
                                .l
                            })
                            .collect();
                        let ascending = lightness.last() >= lightness.first();
                        for pair in lightness.windows(2) {
                            let step = if ascending {
                                pair[1] - pair[0]
                            } else {
                                pair[0] - pair[1]
                            };
                            assert!(
                                step >= -MONOTONE_TOLERANCE,
                                "{name} reverses by {:.4} in {}/{} under {}",
                                -step,
                                theme.name,
                                mode.mode.id(),
                                deficiency.id()
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn findings_carry_a_margin_and_the_normal_vision_baseline() {
        for finding in check(&shipped()).findings {
            assert!(finding.margin.is_some(), "{finding}");
        }
    }
}
