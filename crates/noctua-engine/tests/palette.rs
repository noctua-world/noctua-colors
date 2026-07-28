//! End-to-end tests over the real specification.
//!
//! These build `specs/noctua.toml` — the file the project actually ships —
//! and check the properties the whole system rests on. Several of them are
//! early versions of the quality gates that move into `noctua-check` in
//! milestone 4; running them here means the engine cannot regress in the
//! meantime.

use noctua_core::{Gamut, apca, delta_e_ok};
use noctua_engine::{Mode, Palette, ResolvedFamily, build};
use noctua_spec::Spec;

/// The shipped specification, from the repository root.
fn shipped_spec() -> Spec {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
    noctua_spec::load(path).expect("the shipped spec must be valid")
}

fn shipped_palette() -> Palette {
    build(&shipped_spec()).expect("the shipped spec must build")
}

/// Every family in every theme and mode, for whole-palette sweeps.
fn all_families(palette: &Palette) -> impl Iterator<Item = (String, Mode, &ResolvedFamily)> {
    palette.themes.iter().flat_map(|theme| {
        theme.modes.iter().flat_map(move |mode| {
            mode.families
                .values()
                .map(move |family| (theme.name.clone(), mode.mode, family))
        })
    })
}

#[test]
fn the_shipped_specification_builds() {
    let palette = shipped_palette();
    // Twelve accents crossed with three saturations.
    assert_eq!(palette.themes.len(), 39, "the accent grid");
    assert_eq!(
        palette.themes[0].name, "ochre-balanced",
        "the first cell of the grid is the default palette"
    );
    assert_eq!(palette.gamuts, vec![Gamut::Srgb, Gamut::DisplayP3]);
    assert_eq!(palette.roles.len(), 12);
    assert_eq!(palette.neutral_ramp().len(), 24);

    for (theme, mode, family) in all_families(&palette) {
        assert_eq!(
            family.steps.len(),
            12,
            "{theme}/{} {}",
            mode.id(),
            family.name
        );
    }
}

#[test]
fn a_three_line_specification_builds() {
    // The promise on the front of the README.
    let spec = noctua_spec::parse("minimal.toml", "[families.accent]\nhue = 264").expect("valid");
    let palette = build(&spec).expect("builds");
    assert_eq!(palette.themes.len(), 1, "a default theme is synthesized");
    assert!(palette.themes[0].modes[0].families.contains_key("neutral"));
    assert!(palette.themes[0].modes[0].families.contains_key("accent"));
}

/// Lightness must not reverse along a ramp, or neighbouring steps swap places
/// and a component picks the wrong one.
///
/// The solid steps are excluded, and deliberately. A solid is chosen to be
/// recognised as a brand or a state, not to occupy a rung — every real
/// twelve-step scale steps off its own trajectory there, and this system uses
/// that freedom to push semantic families apart in lightness so a dichromat
/// can still tell them apart.
#[test]
fn lightness_is_monotonic_along_the_ramp_excluding_solids() {
    let palette = shipped_palette();

    for (theme, mode, family) in all_families(&palette) {
        for slot in 0..palette.gamuts.len() {
            let lightnesses: Vec<f64> = family
                .steps
                .iter()
                .filter(|s| !palette.shiftable_roles.contains(&s.role))
                .map(|s| s.renditions[slot].oklch.l)
                .collect();

            let ascending = lightnesses.windows(2).all(|p| p[1] >= p[0]);
            let descending = lightnesses.windows(2).all(|p| p[1] <= p[0]);
            assert!(
                ascending || descending,
                "{theme}/{} {} in {}: lightness reverses: {lightnesses:?}",
                mode.id(),
                family.name,
                palette.gamuts[slot].id()
            );

            // ...and in the direction the mode implies.
            match mode {
                Mode::Light => assert!(descending, "{theme}/light {} should darken", family.name),
                Mode::Dark => assert!(ascending, "{theme}/dark {} should lighten", family.name),
            }
        }
    }
}

/// No two adjacent steps may be indistinguishable.
///
/// The floor only. A step that is the same color as its neighbour is a defect
/// in the solver — two names for one value, and one of them is unreachable.
///
/// The *ceiling* is deliberately not checked here. "This gap reads as a
/// missing step" is a judgement about taste rather than correctness, it is
/// owned by `noctua_check::spacing`, and that gate reports it as a warning
/// with the margin attached. Asserting the same number in both places meant
/// the two disagreed the moment a palette landed between them: the gate said
/// "0.2684, eight thousandths over, your call" while this test failed the
/// build outright.
#[test]
fn no_two_adjacent_steps_are_the_same_color() {
    // Comfortably above the 0.02 just-noticeable difference, so neighbouring
    // steps are always tellable apart.
    const FLOOR: f64 = 0.012;

    let palette = shipped_palette();
    for (theme, mode, family) in all_families(&palette) {
        for pair in family.steps.windows(2) {
            let distance = delta_e_ok(
                pair[0].primary().oklch.to_oklab(),
                pair[1].primary().oklch.to_oklab(),
            );
            assert!(
                distance >= FLOOR,
                "{theme}/{} {}: steps {} and {} are only {distance:.4} apart",
                mode.id(),
                family.name,
                pair[0].index,
                pair[1].index
            );
        }
    }
}

/// Every emitted color must be inside the gamut it claims.
#[test]
fn every_color_is_inside_its_declared_gamut() {
    let palette = shipped_palette();

    let check = |label: &str, step: &noctua_engine::ResolvedStep| {
        for rendition in &step.renditions {
            assert!(
                rendition.gamut.contains(rendition.oklch.to_oklab()),
                "{label} step {} is outside {}",
                step.index,
                rendition.gamut.id()
            );
            for (name, value) in [
                ("r", rendition.rgb.r),
                ("g", rendition.rgb.g),
                ("b", rendition.rgb.b),
            ] {
                assert!(
                    (0.0..=1.0).contains(&value),
                    "{label} step {} channel {name} = {value}",
                    step.index
                );
            }
        }
    };

    for step in palette.neutral_ramp() {
        check("neutral ramp", step);
    }
    for (theme, mode, family) in all_families(&palette) {
        let label = format!("{theme}/{} {}", mode.id(), family.name);
        for step in &family.steps {
            check(&label, step);
        }
    }
}

/// The claim relative chroma exists to make.
#[test]
fn the_wider_gamut_is_never_less_saturated() {
    let palette = shipped_palette();
    assert!(palette.gamuts.len() > 1, "this test needs an upgrade gamut");

    for (theme, mode, family) in all_families(&palette) {
        for step in &family.steps {
            let base = &step.renditions[0];
            for wider in &step.renditions[1..] {
                assert!(
                    wider.oklch.c >= base.oklch.c - 1e-9,
                    "{theme}/{} {} step {}: {} has less chroma than {}",
                    mode.id(),
                    family.name,
                    step.index,
                    wider.gamut.id(),
                    base.gamut.id()
                );
            }
        }
    }
}

/// Every step must sit at the fraction of the boundary its curve asked for,
/// except where the gamut genuinely ran out.
#[test]
fn steps_land_at_the_relative_chroma_they_requested() {
    let palette = shipped_palette();

    for (theme, mode, family) in all_families(&palette) {
        for step in &family.steps {
            let color = step.primary();
            let requested = color.requested_relative_chroma;
            let achieved = color.achieved_relative_chroma;
            assert!(
                achieved <= requested + 0.02,
                "{theme}/{} {} step {}: got {achieved:.3} of the boundary, asked {requested:.3}",
                mode.id(),
                family.name,
                step.index
            );
        }
    }
}

/// Contrast-anchored roles must actually hit their targets.
#[test]
fn every_contrast_anchored_role_meets_its_target() {
    let spec = shipped_spec();
    let palette = build(&spec).expect("builds");

    for (theme, mode, family) in all_families(&palette) {
        let background = family.steps[0].primary();

        for (index, role) in spec.scale.roles.iter().enumerate() {
            let target = if mode == Mode::Light {
                &role.light
            } else {
                &role.dark
            };
            let Some(apca_target) = &target.apca else {
                continue;
            };

            let color = family.steps[index].primary();
            let achieved = apca(color.rgb, background.rgb).abs();

            // A family's colour-vision shift moves its solid targets on
            // purpose, so the number to check against is the shifted one.
            let shift = if role.shift {
                spec.families
                    .get(&family.name)
                    .map_or(0.0, |f| f.contrast_shift)
            } else {
                0.0
            };
            let wanted = (*apca_target.lc.get_ref() + shift).max(0.0);

            // Quantization moves a color by up to a ten-thousandth of a unit
            // of lightness, which is a fraction of an Lc.
            assert!(
                (achieved - wanted).abs() < 1.0,
                "{theme}/{} {} {}: wanted Lc {wanted}, got {achieved:.1}",
                mode.id(),
                family.name,
                role.name.get_ref()
            );
        }
    }
}

/// The neutral is tinted, not dead — and only just.
#[test]
fn the_neutral_ramp_is_tinted_but_not_colorful() {
    let palette = shipped_palette();

    let mut tinted = 0;
    for step in palette.neutral_ramp() {
        let color = step.primary();
        let boundary = Gamut::Srgb.max_chroma(color.oklch.l, color.oklch.h);
        if boundary > 0.0 {
            let relative = color.oklch.c / boundary;
            assert!(
                relative < 0.10,
                "neutral step {} is at {relative:.3} of the boundary, which is a color",
                step.index
            );
            if relative > 0.005 {
                tinted += 1;
            }
        }
    }
    assert!(
        tinted > palette.neutral_ramp().len() / 2,
        "only {tinted} of {} neutral steps carry any tint",
        palette.neutral_ramp().len()
    );
}

#[test]
fn an_achromatic_neutral_really_has_no_chroma() {
    let spec = noctua_spec::parse(
        "t.toml",
        "[families.accent]\nhue = 264\n[neutral]\nachromatic = true",
    )
    .expect("valid");
    let palette = build(&spec).expect("builds");

    for step in palette.neutral_ramp() {
        assert!(
            step.primary().oklch.c < 1e-9,
            "step {} has chroma {}",
            step.index,
            step.primary().oklch.c
        );
    }
}

/// A theme is a multiplier, and the multiplier must actually do something.
#[test]
fn themes_separate_along_the_chroma_axis() {
    let palette = shipped_palette();

    let mean_chroma = |theme_name: &str| -> f64 {
        let theme = palette
            .themes
            .iter()
            .find(|t| t.name == theme_name)
            .expect(theme_name);
        let family = &theme.modes[0].families["accent"];
        let total: f64 = family.steps.iter().map(|s| s.primary().oklch.c).sum();
        total / family.steps.len() as f64
    };

    let (sober, noctua, vivid) = (
        mean_chroma("ochre-sober"),
        mean_chroma("ochre-balanced"),
        mean_chroma("ochre-vivid"),
    );
    assert!(
        sober < noctua,
        "sober {sober:.4} should be quieter than noctua {noctua:.4}"
    );
    assert!(
        noctua < vivid,
        "noctua {noctua:.4} should be quieter than vivid {vivid:.4}"
    );
}

#[test]
fn the_categorical_scale_follows_its_theme() {
    let palette = shipped_palette();

    let mean_chroma = |theme_name: &str| -> f64 {
        let theme = palette
            .themes
            .iter()
            .find(|t| t.name == theme_name)
            .expect(theme_name);
        let chart = theme.modes[0].chart();
        chart.iter().map(|s| s.primary().oklch.c).sum::<f64>() / chart.len() as f64
    };

    assert!(
        mean_chroma("ochre-sober") < mean_chroma("ochre-vivid"),
        "a sober theme must not ship fully saturated charts"
    );
}

/// The neutral ramp puts its steps where interfaces need them.
#[test]
fn the_neutral_ramp_concentrates_on_interface_surfaces() {
    let palette = shipped_palette();
    let lightnesses: Vec<f64> = palette
        .neutral_ramp()
        .iter()
        .map(|s| s.primary().oklch.l)
        .collect();

    let in_band = |low: f64, high: f64| {
        lightnesses
            .iter()
            .filter(|l| (low..=high).contains(l))
            .count()
    };

    let dark_surfaces = in_band(0.10, 0.25);
    let light_surfaces = in_band(0.85, 0.99);
    let middle = in_band(0.45, 0.60);

    assert!(
        dark_surfaces >= 4,
        "dark-mode surfaces got {dark_surfaces} of 24 steps"
    );
    assert!(
        light_surfaces >= 4,
        "light-mode surfaces got {light_surfaces} of 24 steps"
    );
    assert!(
        middle < dark_surfaces && middle < light_surfaces,
        "the middle ({middle}) should be sparser than either end"
    );

    for pair in lightnesses.windows(2) {
        assert!(pair[1] > pair[0], "the ramp must be strictly increasing");
    }
}

/// Every step of the dense ramp must be a *different color* once quantized.
///
/// Increasing lightness is not enough. Below about 0.09 lightness, eight-bit
/// sRGB has no codes left and several distinct steps collapse onto `#000000`,
/// which looks like a working ramp in the numbers and like a solid black block
/// on screen.
#[test]
fn no_two_neutral_steps_render_to_the_same_color() {
    let palette = shipped_palette();

    for pair in palette.neutral_ramp().windows(2) {
        let (a, b) = (pair[0].primary().hex(), pair[1].primary().hex());
        assert_ne!(
            a, b,
            "neutral steps {} and {} are both {a}",
            pair[0].index, pair[1].index
        );
    }
}

/// Building twice must produce identical output, or nothing downstream can be
/// byte-reproducible.
#[test]
fn building_is_deterministic() {
    let spec = shipped_spec();
    let first = build(&spec).expect("builds");
    let second = build(&spec).expect("builds");

    assert_eq!(render(&first), render(&second));
}

/// An unreachable target fails loudly, naming what is achievable.
#[test]
fn an_impossible_target_is_reported_rather_than_clamped() {
    let spec = noctua_spec::parse(
        "t.toml",
        r#"
        [families.accent]
        hue = 264
        [[scale.roles]]
        name = "bg-app"
        light = { lightness = 0.55 }
        dark = { lightness = 0.55 }
        [[scale.roles]]
        name = "text"
        light = { apca = { against = "bg-app", lc = 105 } }
        dark = { apca = { against = "bg-app", lc = 105 } }
        "#,
    )
    .expect("valid spec");

    let error = build(&spec).expect_err("Lc 105 against a mid gray is impossible");
    let message = error.to_string();
    assert!(
        message.contains("105"),
        "should name what was asked: {message}"
    );
    assert!(message.contains("cannot reach"), "{message}");
    assert!(
        !error.fix().is_empty(),
        "every failure must say what to change"
    );
}

// --- Golden ---------------------------------------------------------------

/// Renders the whole palette as stable text.
fn render(palette: &Palette) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();

    writeln!(out, "prefix {}", palette.prefix).unwrap();
    writeln!(
        out,
        "gamuts {}",
        palette
            .gamuts
            .iter()
            .map(|g| g.id())
            .collect::<Vec<_>>()
            .join(" ")
    )
    .unwrap();

    writeln!(out, "\n[neutral-ramp]").unwrap();
    for step in palette.neutral_ramp() {
        let color = step.primary();
        writeln!(
            out,
            "  {:>3}  {}  oklch({:.4} {:.4} {:.2})",
            step.index,
            color.hex(),
            color.oklch.l,
            color.oklch.c,
            color.oklch.h
        )
        .unwrap();
    }

    for theme in &palette.themes {
        for mode in &theme.modes {
            writeln!(out, "\n[{}/{}]", theme.name, mode.mode.id()).unwrap();
            for family in mode.families.values() {
                writeln!(out, "  {}", family.name).unwrap();
                for step in &family.steps {
                    let color = step.primary();
                    writeln!(
                        out,
                        "    {:>2} {:<18} {}  oklch({:.4} {:.4} {:.2})  cr {:.3}",
                        step.index,
                        step.role,
                        color.hex(),
                        color.oklch.l,
                        color.oklch.c,
                        color.oklch.h,
                        color.achieved_relative_chroma
                    )
                    .unwrap();
                }
            }
            write!(out, "  chart").unwrap();
            for step in mode.chart() {
                write!(out, " {}", step.primary().hex()).unwrap();
            }
            writeln!(out).unwrap();
        }
    }
    out
}

/// Snapshots the whole palette so unintended drift shows up in review.
///
/// To accept an intentional change, run:
///
/// ```text
/// UPDATE_GOLDEN=1 cargo test -p noctua-engine --test palette
/// ```
///
/// This folds into `cargo xtask build` in milestone 3, when `system/` becomes
/// the single golden mechanism with a single update command.
#[test]
fn the_shipped_palette_matches_its_golden_file() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/golden/palette.txt"
    );
    let rendered = render(&shipped_palette());

    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(std::path::Path::new(path).parent().expect("has a parent"))
            .expect("create golden directory");
        std::fs::write(path, &rendered).expect("write golden");
        return;
    }

    let expected = std::fs::read_to_string(path).unwrap_or_else(|_| {
        panic!("golden file missing; create it with UPDATE_GOLDEN=1 cargo test")
    });

    if expected != rendered {
        let first_difference = expected
            .lines()
            .zip(rendered.lines())
            .position(|(a, b)| a != b)
            .unwrap_or(expected.lines().count().min(rendered.lines().count()));
        panic!(
            "the palette changed at line {}:\n  golden:   {:?}\n  produced: {:?}\n\n\
             If this was intended, run: UPDATE_GOLDEN=1 cargo test -p noctua-engine --test palette",
            first_difference + 1,
            expected.lines().nth(first_difference),
            rendered.lines().nth(first_difference),
        );
    }
}
