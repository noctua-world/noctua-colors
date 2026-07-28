//! Semantic validation.
//!
//! Parsing catches a malformed file. This catches a well-formed file that says
//! something impossible: a role anchored to a role that does not exist, a
//! theme mapping a semantic slot onto a missing family, a relative chroma of
//! 1.4.
//!
//! Everything is checked before anything is reported, so one pass surfaces
//! every problem.

use crate::error::{Problem, did_you_mean};
use crate::model::{Chart, Spec, TargetSpec};

/// Checks a parsed spec, returning every problem found.
pub(crate) fn check(spec: &Spec) -> Vec<Problem> {
    let mut problems = Vec::new();

    check_families(spec, &mut problems);
    check_roles(spec, &mut problems);
    check_neutral(spec, &mut problems);
    check_axes(spec, &mut problems);
    check_themes(spec, &mut problems);
    check_semantic(spec, &mut problems);
    check_chart(spec, &mut problems);
    check_scales(spec, &mut problems);
    check_alpha(spec, &mut problems);
    check_system(spec, &mut problems);

    problems
}

/// The colour system's declared version must be usable as one.
///
/// It is stamped into `package.json`, the generated crate's `Cargo.toml` and
/// the manifest, and both registries reject a malformed version — but only
/// after a tag has been pushed and a release workflow has started. Catching it
/// here costs nothing and turns a failed publish into a failed `check`.
///
/// Deliberately not a full semver parser: this rejects the shapes that are
/// certainly wrong rather than ruling on every shape that is right, because
/// pre-release and build metadata are legitimate and this crate takes no
/// dependency to know it.
fn check_system(spec: &Spec, problems: &mut Vec<Problem>) {
    let version = spec.system.version.trim();

    if version.is_empty() {
        problems.push(
            Problem::whole_file("[system] has no version")
                .fix("give it one, as in `version = \"0.2.0\"`"),
        );
        return;
    }

    let core = version
        .split_once(['-', '+'])
        .map_or(version, |(core, _)| core);
    let numeric = core.split('.').collect::<Vec<_>>();

    if numeric.len() != 3
        || numeric
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        problems.push(
            Problem::whole_file(format!(
                "[system] version \"{version}\" is not major.minor.patch"
            ))
            .fix("both registries this publishes to require SemVer, as in `version = \"0.2.0\"`"),
        );
    }

    if spec.system.name.trim().is_empty() {
        problems.push(
            Problem::whole_file("[system] has an empty name")
                .fix("generated package metadata needs one, as in `name = \"noctua-colors\"`"),
        );
    }
}

/// Every family a semantic slot may point at, including the synthesized ones.
///
/// The engine builds `neutral` from `[neutral]` unless one is declared, and
/// `neutral-cool` and `neutral-warm` whenever `[neutral]` names their hues — so
/// all three are valid targets that never appear under `[families]`.
fn semantic_targets(spec: &Spec) -> Vec<String> {
    let mut targets: Vec<String> = spec.families.keys().cloned().collect();
    if !targets.iter().any(|name| name == "neutral") {
        targets.push("neutral".to_owned());
        if spec.neutral.cool_hue.is_some() && !spec.neutral.achromatic {
            targets.push("neutral-cool".to_owned());
        }
        if spec.neutral.warm_hue.is_some() && !spec.neutral.achromatic {
            targets.push("neutral-warm".to_owned());
        }
    }
    targets
}

/// Stems the emitted token names are built from.
///
/// A slot is interpolated straight into a token name — `rejected` becomes
/// `--nc-color-rejected`, `--nc-color-rejected-bg`, `--nc-color-on-rejected` —
/// so a slot called `surface` would emit a second `--nc-color-surface` from
/// whatever family it named, silently overriding the page's own.
///
/// The shapes these come from live in `noctua_emit::tokens`. This crate cannot
/// depend on that one, so the list is repeated; it is short, it changes when the
/// contract changes, and getting it wrong costs a missed diagnostic rather than
/// a wrong palette.
const RESERVED_STEMS: [&str; 5] = ["surface", "fg", "border", "ring", "on"];

/// The global `[semantic]` map: are the targets real, and are the names usable?
///
/// Before this existed a mistyped family here reached the engine, which reported
/// it without a span or a suggestion, and a mistyped *slot* reached the output —
/// emitted as a token nobody asked for, with no diagnostic at all.
fn check_semantic(spec: &Spec, problems: &mut Vec<Problem>) {
    let targets = semantic_targets(spec);

    for (slot, family) in &spec.semantic {
        if !targets.iter().any(|name| name == family.get_ref()) {
            let mut problem = Problem::at(
                family.span(),
                format!(
                    "`{slot}` is mapped to family `{}`, which does not exist",
                    family.get_ref()
                ),
            )
            .labelled("no such family");
            problem = match did_you_mean(family.get_ref(), targets.iter().map(String::as_str)) {
                Some(suggestion) => problem.fix(format!("did you mean `{suggestion}`?")),
                None => problem.fix(format!(
                    "the families available are: {}",
                    targets.join(", ")
                )),
            };
            problems.push(problem);
        }

        // A slot name has to survive being pasted into a custom property, a
        // Rust identifier and a QML property. Anything outside this alphabet
        // does not.
        let usable = !slot.is_empty()
            && !slot.starts_with('-')
            && !slot.ends_with('-')
            && slot
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !usable {
            problems.push(
                Problem::whole_file(format!("`{slot}` is not a usable semantic slot name"))
                    .fix(
                        "use lowercase letters, digits and interior hyphens:                          `rejected`, `level-3`",
                    ),
            );
        }

        let stem = slot.split('-').next().unwrap_or(slot);
        if RESERVED_STEMS.contains(&stem) {
            problems.push(
                Problem::whole_file(format!(
                    "slot `{slot}` starts with `{stem}`, which the page's own tokens use"
                ))
                .fix(format!(
                    "the emitted contract already defines `{stem}` names from the neutral                      family; pick a slot name that does not begin with one of: {}",
                    RESERVED_STEMS.join(", ")
                )),
            );
        }

        // `neutral-cool` is read as a *temperature of the page* and emits
        // surfaces, text and borders rather than a fill. Pointing one at a
        // coloured family emits `fg-cool` from an accent, which is nonsense.
        if (slot == "neutral" || slot.starts_with("neutral-"))
            && !family.get_ref().starts_with("neutral")
        {
            problems.push(
                Problem::at(
                    family.span(),
                    format!(
                        "slot `{slot}` names a neutral temperature but points at family `{}`",
                        family.get_ref()
                    ),
                )
                .labelled("not a neutral")
                .fix(
                    "a `neutral*` slot emits the page's surfaces and text, so it has to                      point at a neutral family; for a coloured context use a name that                      does not begin with `neutral`",
                ),
            );
        }
    }
}

/// The `[alpha]` ladder: real percentages, of a role that exists.
fn check_alpha(spec: &Spec, problems: &mut Vec<Problem>) {
    if spec.alpha.percentages.is_empty() {
        problems.push(
            Problem::whole_file("the alpha ladder has no stops")
                .fix("give it percentages, as in `percentages = [4, 8, 16]`"),
        );
    }

    for percentage in &spec.alpha.percentages {
        if !(0.0..=100.0).contains(percentage) {
            problems.push(
                Problem::whole_file(format!("alpha stop {percentage} is not a percentage"))
                    .fix("opacity runs from 0 (invisible) to 100 (opaque)"),
            );
        }
    }

    // Only when named. Left out it means the last role, which always exists.
    let Some(named) = &spec.alpha.role else {
        return;
    };

    let roles: Vec<&str> = spec
        .scale
        .roles
        .iter()
        .map(|role| role.name.get_ref().as_str())
        .collect();
    if !roles.contains(&named.as_str()) {
        let mut problem = Problem::whole_file(format!(
            "the alpha ladder is a wash of role `{named}`, which is not a role"
        ));
        problem = match did_you_mean(named, roles.iter().copied()) {
            Some(suggestion) => problem.fix(format!("did you mean `{suggestion}`?")),
            None => problem.fix(format!("the roles defined are: {}", roles.join(", "))),
        };
        problems.push(problem);
    }
}

/// The `[[scales]]` list: named once each, and with something to place.
fn check_scales(spec: &Spec, problems: &mut Vec<Problem>) {
    let mut seen: Vec<&str> = Vec::new();

    for scale in &spec.scales {
        if scale.name.is_empty() {
            problems.push(
                Problem::whole_file("a scale has no name")
                    .fix("`name` becomes the token stem, as in `level-0`"),
            );
        }

        // `chart` is where the categorical scale lives in the resolved palette,
        // and a second entry under that key would replace it.
        if scale.name == "chart" {
            problems.push(
                Problem::whole_file("a scale is named `chart`, which is already taken")
                    .fix("the categorical scale is configured under `[chart]`; rename this one"),
            );
        }

        if seen.contains(&scale.name.as_str()) {
            problems.push(
                Problem::whole_file(format!("two scales are named `{}`", scale.name))
                    .fix("scale names become token stems, so they have to be distinct"),
            );
        }
        seen.push(&scale.name);

        if scale.stops.is_empty() {
            problems.push(
                Problem::whole_file(format!("scale `{}` has no stops", scale.name)).fix(
                    "give it a count (`stops = 11`) or a list of names                      (`stops = [\"low\", \"high\"]`)",
                ),
            );
        }

        for knot in scale.cr.knots() {
            if !(0.0..=1.0).contains(&knot[1]) {
                problems.push(
                    Problem::whole_file(format!(
                        "scale `{}` asks for relative chroma {}",
                        scale.name, knot[1]
                    ))
                    .fix("relative chroma is a fraction of what the gamut allows: 0 to 1"),
                );
            }
        }
    }
}

fn check_families(spec: &Spec, problems: &mut Vec<Problem>) {
    if spec.families.is_empty() {
        problems.push(
            Problem::whole_file("the specification defines no color families")
                .fix("add one, for example:\n\n    [families.accent]\n    hue = 264"),
        );
    }

    for (name, family) in &spec.families {
        for knot in family.cr.knots() {
            if !(0.0..=1.0).contains(&knot[1]) {
                problems.push(
                    Problem::whole_file(format!(
                        "family `{name}` has a relative chroma of {} at t = {}",
                        knot[1], knot[0]
                    ))
                    .fix(
                        "relative chroma is a fraction of what the gamut allows, \
                         so it must be between 0 and 1. To make a family more \
                         saturated than the gamut permits, widen `output.gamut`.",
                    ),
                );
            }
        }

        for correction in &family.hue_correction {
            if !(0.0..=1.0).contains(&correction[0]) {
                problems.push(
                    Problem::whole_file(format!(
                        "family `{name}` has a hue correction anchored at lightness {}",
                        correction[0]
                    ))
                    .fix("the first number of each pair is a lightness, so it must be between 0 and 1"),
                );
            }
        }
    }
}

fn check_roles(spec: &Spec, problems: &mut Vec<Problem>) {
    let names: Vec<&str> = spec
        .scale
        .roles
        .iter()
        .map(|r| r.name.get_ref().as_str())
        .collect();

    if spec.scale.roles.is_empty() {
        problems.push(
            Problem::whole_file("the scale defines no roles")
                .fix("remove the `[scale]` section to use the default twelve-step scale"),
        );
        return;
    }

    for (position, role) in spec.scale.roles.iter().enumerate() {
        let role_name = role.name.get_ref();

        if names.iter().filter(|n| *n == role_name).count() > 1 {
            problems.push(
                Problem::at(
                    role.name.span(),
                    format!("duplicate role name `{role_name}`"),
                )
                .labelled("declared more than once")
                .fix("role names must be unique; every emitted token is keyed by one"),
            );
        }

        for (mode, target) in [("light", &role.light), ("dark", &role.dark)] {
            check_target(role_name, mode, target, &names, position, spec, problems);
        }
    }
}

fn check_target(
    role_name: &str,
    mode: &str,
    target: &TargetSpec,
    names: &[&str],
    position: usize,
    spec: &Spec,
    problems: &mut Vec<Problem>,
) {
    let set = usize::from(target.lightness.is_some())
        + usize::from(target.apca.is_some())
        + usize::from(target.delta_l.is_some());

    if set != 1 {
        let message = if set == 0 {
            format!("role `{role_name}` sets no {mode} target")
        } else {
            format!("role `{role_name}` sets {set} {mode} targets at once")
        };
        problems.push(Problem::whole_file(message).fix(
            "a target must set exactly one of `lightness`, `apca` or `delta_l`. \
             Use `apca` for text and solids, `delta_l` for surfaces and borders, \
             and `lightness` to anchor the ramp.",
        ));
        return;
    }

    if let Some(value) = &target.lightness
        && !(0.0..=1.0).contains(value.get_ref())
    {
        problems.push(
            Problem::at(
                value.span(),
                format!("lightness {} is out of range", value.get_ref()),
            )
            .labelled("must be between 0 and 1")
            .fix("0 is black and 1 is white"),
        );
    }

    let reference = target
        .apca
        .as_ref()
        .map(|t| (&t.against, "apca"))
        .or_else(|| target.delta_l.as_ref().map(|t| (&t.against, "delta_l")));

    if let Some((against, kind)) = reference {
        let referenced = against.get_ref();

        if !names.contains(&referenced.as_str()) {
            let mut problem = Problem::at(
                against.span(),
                format!("role `{role_name}` targets `{referenced}`, which is not a role"),
            )
            .labelled("no such role");
            problem = match did_you_mean(referenced, names.iter().copied()) {
                Some(suggestion) => problem.fix(format!("did you mean `{suggestion}`?")),
                None => problem.fix(format!("the roles in this scale are: {}", names.join(", "))),
            };
            problems.push(problem);
        } else if referenced == role_name {
            problems.push(
                Problem::at(against.span(), format!("role `{role_name}` targets itself"))
                    .labelled("circular reference")
                    .fix("a role must be anchored to a different role, or given a `lightness`"),
            );
        } else {
            // Resolution walks the scale in order, so a role may only depend
            // on one declared before it.
            let referenced_position = spec
                .scale
                .roles
                .iter()
                .position(|r| r.name.get_ref() == referenced);
            if referenced_position.is_some_and(|p| p > position) {
                problems.push(
                    Problem::at(
                        against.span(),
                        format!("role `{role_name}` targets `{referenced}`, declared after it"),
                    )
                    .labelled("declared later in the scale")
                    .fix(format!(
                        "move `{referenced}` above `{role_name}`, or anchor `{role_name}` \
                         to a role that comes earlier"
                    )),
                );
            }
        }

        check_target_ranges(kind, target, problems);
    }
}

/// Range checks on the numeric part of a target.
///
/// Split out of `check_target` purely for length; it reads as one idea.
fn check_target_ranges(kind: &str, target: &TargetSpec, problems: &mut Vec<Problem>) {
    if kind == "apca"
        && let Some(apca) = &target.apca
    {
        let lc = *apca.lc.get_ref();
        if lc < 0.0 {
            problems.push(
                Problem::at(apca.lc.span(), format!("negative contrast target {lc}"))
                    .labelled("must be a magnitude")
                    .fix(
                        "`lc` is a magnitude; polarity follows the mode, so light and \
                         dark use the same positive number",
                    ),
            );
        } else if lc > 108.0 {
            problems.push(
                Problem::at(
                    apca.lc.span(),
                    format!("contrast target {lc} is unreachable"),
                )
                .labelled("beyond the maximum")
                .fix(
                    "the most APCA contrast any pair can reach is about 108, \
                         black on white. Try 90 or lower for body text.",
                ),
            );
        }
    }

    if let Some(delta) = &target.delta_l {
        let amount = *delta.amount.get_ref();
        if !(0.0..=1.0).contains(&amount) {
            problems.push(
                Problem::at(
                    delta.amount.span(),
                    format!("separation {amount} is out of range"),
                )
                .labelled("must be between 0 and 1")
                .fix(
                    "this is a lightness separation, and lightness spans 0 to 1. \
                         Surface steps usually want 0.02 to 0.3.",
                ),
            );
        }
    }
}

fn check_neutral(spec: &Spec, problems: &mut Vec<Problem>) {
    let neutral = &spec.neutral;

    if neutral.steps < 2 {
        problems.push(
            Problem::whole_file(format!("the neutral ramp has {} step(s)", neutral.steps))
                .fix("a ramp needs at least 2 steps; the default is 12"),
        );
    }

    if !(0.0..=1.0).contains(&neutral.tint_strength) {
        problems.push(
            Problem::whole_file(format!(
                "neutral tint strength {} is out of range",
                neutral.tint_strength
            ))
            .fix(
                "this is a relative chroma, so it must be between 0 and 1. \
                 A tinted gray usually wants 0.02 to 0.05; set `achromatic = true` for none.",
            ),
        );
    }

    let mut previous_high = f64::NEG_INFINITY;
    for band in &neutral.density {
        if band.range[0] >= band.range[1] {
            problems.push(
                Problem::whole_file(format!(
                    "neutral density band {:?} covers nothing",
                    band.range
                ))
                .fix("a band is `[low, high]` with low below high"),
            );
        }
        if band.weight <= 0.0 {
            problems.push(
                Problem::whole_file(format!(
                    "neutral density band {:?} has weight {}",
                    band.range, band.weight
                ))
                .fix("weight is a relative density and must be greater than 0"),
            );
        }
        if band.range[0] < previous_high {
            problems.push(
                Problem::whole_file(format!(
                    "neutral density band {:?} overlaps the one before it",
                    band.range
                ))
                .fix("bands must be sorted and must not overlap"),
            );
        }
        previous_high = band.range[1];
    }
}

/// The accent grid, checked before it is used rather than after it has
/// produced thirty-six confusing themes.
///
/// Runs against the *expanded* spec, so `spec.accents` is what was written and
/// `spec.themes` already contains the grid.
fn check_axes(spec: &Spec, problems: &mut Vec<Problem>) {
    if !spec.accents.is_empty() && !spec.families.contains_key("accent") {
        problems.push(
            Problem::whole_file(
                "`[accents]` replaces the `accent` family's hue, but no `[families.accent]` \
                 is declared",
            )
            .fix("add `[families.accent]` with a `hue`, or remove `[accents]`"),
        );
    }

    if !spec.accents.is_empty() && spec.saturations.is_empty() {
        problems.push(
            Problem::whole_file(
                "`[accents]` is set but `[saturations]` is empty, so no palettes are generated",
            )
            .fix("add at least one entry to `[saturations]`, such as `balanced = 0.82`"),
        );
    }

    for (name, multiplier) in &spec.saturations {
        if *multiplier < 0.0 {
            problems.push(
                Problem::whole_file(format!(
                    "saturation `{name}` has a negative chroma multiplier ({multiplier})"
                ))
                .fix("chroma multipliers scale relative chroma, so they cannot be negative"),
            );
        }
    }

    // A generated name that collides with a hand-written theme would be
    // silently dropped by the expander, leaving a hole in the grid that only
    // shows up as a missing entry in the picker.
    for accent in spec.accents.keys() {
        for saturation in spec.saturations.keys() {
            let generated = format!("{accent}-{saturation}");
            let clashes = spec
                .themes
                .get(&generated)
                .is_some_and(|theme| theme.accent.is_none());
            if clashes {
                problems.push(
                    Problem::whole_file(format!(
                        "`[themes.{generated}]` has the same name as the palette generated \
                         from accent `{accent}` and saturation `{saturation}`"
                    ))
                    .fix("rename the hand-written theme, or the accent, or the saturation"),
                );
            }
        }
    }
}

fn check_themes(spec: &Spec, problems: &mut Vec<Problem>) {
    let mut families: Vec<&str> = spec.families.keys().map(String::as_str).collect();

    // The engine synthesizes a `neutral` family from the `[neutral]` section
    // unless one is declared explicitly, so it is always a valid target even
    // though it never appears under `[families]`.
    if !families.contains(&"neutral") {
        families.push("neutral");
    }

    for (theme_name, theme) in &spec.themes {
        if theme.chroma < 0.0 {
            problems.push(
                Problem::whole_file(format!(
                    "theme `{theme_name}` has a negative chroma multiplier"
                ))
                .fix("0 makes every family gray; 1 leaves them as defined"),
            );
        }

        for (slot, family) in &theme.semantic {
            if !families.contains(&family.get_ref().as_str()) {
                let mut problem = Problem::at(
                    family.span(),
                    format!(
                        "theme `{theme_name}` maps `{slot}` to family `{}`, which does not exist",
                        family.get_ref()
                    ),
                )
                .labelled("no such family");
                problem = match did_you_mean(family.get_ref(), families.iter().copied()) {
                    Some(suggestion) => problem.fix(format!("did you mean `{suggestion}`?")),
                    None if families.is_empty() => {
                        problem.fix("no families are defined; add one under `[families]`")
                    }
                    None => {
                        problem.fix(format!("the families defined are: {}", families.join(", ")))
                    }
                };
                problems.push(problem);
            }
        }

        for name in theme.families.keys() {
            if !families.contains(&name.as_str()) {
                let mut problem = Problem::whole_file(format!(
                    "theme `{theme_name}` overrides family `{name}`, which does not exist"
                ));
                problem = match did_you_mean(name, families.iter().copied()) {
                    Some(suggestion) => problem.fix(format!("did you mean `{suggestion}`?")),
                    None => {
                        problem.fix(format!("the families defined are: {}", families.join(", ")))
                    }
                };
                problems.push(problem);
            }
        }
    }
}

/// The unnamed `[chart]` and every `[[charts]]` entry.
///
/// The name rules are here rather than left to fail downstream because a
/// duplicate would not fail downstream: both charts land in one map keyed by
/// stem, so the second silently replaces the first and the build succeeds with
/// a scale missing.
fn check_chart(spec: &Spec, problems: &mut Vec<Problem>) {
    if spec.chart.name.is_some() {
        problems
            .push(Problem::whole_file("`[chart]` cannot be renamed").fix(
                "it is always emitted as `chart-*`; for another set add a `[[charts]]` block",
            ));
    }

    // Every name a scale could already be using. A chart that collides with one
    // replaces it in the resolved palette.
    let mut taken: Vec<String> = vec![noctua_chart_stem()];
    taken.extend(spec.scales.iter().map(|scale| scale.name.clone()));

    for chart in &spec.charts {
        let Some(name) = chart.name.as_deref() else {
            problems.push(
                Problem::whole_file("a chart in `[[charts]]` has no name")
                    .fix("`name` becomes the token stem, as in `chart-wide-1`"),
            );
            continue;
        };

        if name.is_empty() {
            problems.push(
                Problem::whole_file("a chart has an empty name")
                    .fix("`name` becomes the token stem, as in `chart-wide-1`"),
            );
        }

        if taken.iter().any(|other| other == name) {
            problems.push(
                Problem::whole_file(format!("`{name}` is already the name of a scale"))
                    .fix("chart and scale names become token stems, so they have to be distinct"),
            );
        }
        taken.push(name.to_owned());
    }

    for chart in std::iter::once(&spec.chart).chain(&spec.charts) {
        check_one_chart(chart, problems);
    }
}

/// The stem the unnamed chart occupies.
///
/// Repeated from `noctua_engine::CHART_SCALE`, which this crate cannot depend
/// on. Getting it wrong costs a missed diagnostic, not a wrong palette.
fn noctua_chart_stem() -> String {
    "chart".to_owned()
}

fn check_one_chart(chart: &Chart, problems: &mut Vec<Problem>) {
    let named = chart
        .name
        .as_deref()
        .map_or_else(|| "chart".to_owned(), |name| format!("chart `{name}`"));

    if chart.count == 0 {
        problems.push(
            Problem::whole_file(format!("{named} has no entries"))
                .fix("set `count` to at least 1, or leave it out for the default of 6"),
        );
    }

    if !(0.0..=1.0).contains(&chart.cr) {
        problems.push(
            Problem::whole_file(format!("{named} asks for relative chroma {}", chart.cr))
                .fix("relative chroma must be between 0 and 1"),
        );
    }

    for (field, value) in [
        ("lightness_light", chart.lightness_light),
        ("lightness_dark", chart.lightness_dark),
    ] {
        if !(0.0..=1.0).contains(&value) {
            problems.push(
                Problem::whole_file(format!("{named} {field} of {value} is out of range"))
                    .fix("lightness runs from 0 (black) to 1 (white)"),
            );
        }
    }
}
