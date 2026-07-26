//! Property tests for the color math.
//!
//! The unit tests inside `noctua-core` check specific values and specific
//! known-hard cases. These check the *laws* — the things that must hold for
//! every color, not just the ones somebody thought to write down. Between them
//! they cover the guarantees the rest of the compiler is built on:
//! gamut mapping never invents saturation or shifts hue, conversions round
//! trip, and the gamut boundary really is a boundary.

// Assertions here compare against literal sentinels the functions return
// verbatim (exactly 0.0, exactly 1.0). Exact comparison is the assertion.
#![allow(clippy::float_cmp)]

use noctua_core::cvd::{Cvd, separation};
use noctua_core::map::{from_hex, oklab_to_rgb_unmapped, rgb_to_oklch};
use noctua_core::space::hue_difference;
use noctua_core::{Gamut, Oklab, Oklch, Rgb, apca, delta_e_ok, map_into_gamut, to_hex, wcag21};
use proptest::prelude::*;

/// Any color expressible in OKLCH, including many no display can show.
fn any_oklch() -> impl Strategy<Value = Oklch> {
    (0.0..=1.0_f64, 0.0..=0.5_f64, 0.0..360.0_f64).prop_map(|(l, c, h)| Oklch { l, c, h })
}

/// Any color a display can actually show, given as encoded RGB.
fn any_rgb() -> impl Strategy<Value = Rgb> {
    (0.0..=1.0_f64, 0.0..=1.0_f64, 0.0..=1.0_f64).prop_map(|(r, g, b)| Rgb { r, g, b })
}

fn any_gamut() -> impl Strategy<Value = Gamut> {
    prop_oneof![
        Just(Gamut::Srgb),
        Just(Gamut::DisplayP3),
        Just(Gamut::Rec2020)
    ]
}

fn any_cvd() -> impl Strategy<Value = Cvd> {
    prop_oneof![
        Just(Cvd::Protanopia),
        Just(Cvd::Deuteranopia),
        Just(Cvd::Tritanopia)
    ]
}

proptest! {
    // The default 256 cases leave a lot of the (lightness, chroma, hue,
    // gamut) space unvisited, and the whole file still runs in well under a
    // second. Raising it is free coverage.
    #![proptest_config(ProptestConfig { cases: 2048, ..ProptestConfig::default() })]

    // --- Gamut mapping ---------------------------------------------------

    /// Mapping is a reduction. If it could add chroma it would be inventing
    /// saturation the author did not ask for.
    #[test]
    fn gamut_mapping_never_increases_chroma(color in any_oklch(), gamut in any_gamut()) {
        let mapped = map_into_gamut(color, gamut);
        prop_assert!(
            mapped.oklch.c <= color.c + 1e-9,
            "{:?} -> {:?} in {}", color, mapped.oklch, gamut.id()
        );
    }

    /// Hue is held fixed. This is the property per-channel clipping destroys,
    /// and the reason this project forbids it.
    #[test]
    fn gamut_mapping_preserves_hue(color in any_oklch(), gamut in any_gamut()) {
        let mapped = map_into_gamut(color, gamut);

        // Hue is undefined once chroma reaches zero, so only colors that keep
        // some chroma can be asked about it.
        prop_assume!(mapped.oklch.c > 1e-4);

        let drift = hue_difference(color.h, mapped.oklch.h).abs();
        prop_assert!(
            drift < 1.0,
            "hue drifted {drift} degrees mapping {:?} into {}", color, gamut.id()
        );
    }

    /// Lightness is held fixed too, except at the extremes where the gamut has
    /// collapsed to a point and there is nowhere else to go.
    #[test]
    fn gamut_mapping_preserves_lightness(color in any_oklch(), gamut in any_gamut()) {
        let mapped = map_into_gamut(color, gamut);
        prop_assert!(
            (mapped.oklch.l - color.l).abs() < 0.03,
            "lightness moved {} mapping {:?} into {}",
            (mapped.oklch.l - color.l).abs(), color, gamut.id()
        );
    }

    /// The output is always displayable. Reaching this assertion at all also
    /// demonstrates termination: the bisection inside cannot loop forever.
    #[test]
    fn gamut_mapping_always_produces_a_displayable_color(
        color in any_oklch(), gamut in any_gamut()
    ) {
        let m = map_into_gamut(color, gamut);
        for (name, v) in [("r", m.rgb.r), ("g", m.rgb.g), ("b", m.rgb.b)] {
            prop_assert!(
                (0.0..=1.0).contains(&v) && v.is_finite(),
                "{} channel {name} = {v} for {:?}", gamut.id(), color
            );
        }
    }

    /// Mapping is an exact fixed point, not merely a near one.
    ///
    /// This was a real bug. The reported coordinates used to be the
    /// bisection's last candidate, which is *not* in gamut — the algorithm
    /// stops when the candidate is perceptually indistinguishable from its
    /// clipped form, not identical to it. So mapping a mapped color mapped it
    /// again, shaving about 7e-5 of chroma every pass with no limit.
    ///
    /// A tolerance would have hidden that. Exact equality is what proves the
    /// result is genuinely in gamut.
    #[test]
    fn gamut_mapping_is_an_exact_fixed_point(color in any_oklch(), gamut in any_gamut()) {
        let once = map_into_gamut(color, gamut);
        let twice = map_into_gamut(once.oklch, gamut);
        prop_assert_eq!(
            once.oklch, twice.oklch,
            "mapping a mapped color changed it"
        );
    }

    /// The coordinates and the channels must describe the same color.
    ///
    /// An emitter writes `oklch()` from one and a hex fallback from the other.
    /// If they disagree, the two layers of every generated stylesheet are
    /// different colors.
    #[test]
    fn the_reported_coordinates_match_the_reported_channels(
        color in any_oklch(), gamut in any_gamut()
    ) {
        let mapped = map_into_gamut(color, gamut);
        let from_channels = gamut.linear_to_oklab(gamut.decode(mapped.rgb));
        prop_assert!(
            delta_e_ok(mapped.oklch.to_oklab(), from_channels) < 0.02,
            "coordinates {:?} and channels {:?} disagree", mapped.oklch, from_channels
        );
    }

    // --- Gamut boundary --------------------------------------------------

    /// The contract of `max_chroma`: everything beneath it is displayable.
    /// Relative chroma depends on this for every value below one.
    #[test]
    fn everything_below_the_boundary_is_in_gamut(
        l in 0.001..0.999_f64, h in 0.0..360.0_f64, fraction in 0.0..=1.0_f64,
        gamut in any_gamut()
    ) {
        let max = gamut.max_chroma(l, h);
        let color = Oklch { l, c: max * fraction, h };
        prop_assert!(
            gamut.contains(color.to_oklab()),
            "{} l={l} h={h}: {fraction} of max {max} is outside", gamut.id()
        );
    }

    /// A wider gamut can never show less. This is what makes one token
    /// definition render more vividly on a P3 display without redefinition.
    #[test]
    fn srgb_never_exceeds_the_wider_gamuts(l in 0.001..0.999_f64, h in 0.0..360.0_f64) {
        let srgb = Gamut::Srgb.max_chroma(l, h);
        for wider in [Gamut::DisplayP3, Gamut::Rec2020] {
            prop_assert!(
                wider.max_chroma(l, h) >= srgb - 1e-9,
                "{} at l={l} h={h}: {} < srgb {srgb}", wider.id(), wider.max_chroma(l, h)
            );
        }
    }

    // --- Conversions -----------------------------------------------------

    /// Encoded RGB survives a trip through Oklab and back, bit for bit once
    /// quantized to hex. Everything the compiler emits depends on this.
    #[test]
    fn rgb_round_trips_through_oklch(rgb in any_rgb(), gamut in any_gamut()) {
        let back = map_into_gamut(rgb_to_oklch(rgb, gamut), gamut);
        prop_assert_eq!(to_hex(back.rgb), to_hex(rgb));
    }

    /// The rectangular and polar forms of Oklab are the same color.
    #[test]
    fn oklab_and_oklch_are_the_same_color(color in any_oklch()) {
        let back = color.to_oklab().to_oklch();
        prop_assert!((back.l - color.l).abs() < 1e-12);
        prop_assert!((back.c - color.c).abs() < 1e-12);
        if color.c > 1e-6 {
            prop_assert!(hue_difference(color.h, back.h).abs() < 1e-9);
        }
    }

    /// Hex parsing and formatting are inverses.
    #[test]
    fn hex_round_trips(rgb in any_rgb()) {
        let text = to_hex(rgb);
        let parsed = from_hex(&text).expect("output of to_hex must parse");
        prop_assert_eq!(to_hex(parsed), text);
    }

    // --- Perceptual difference -------------------------------------------

    /// Delta-E is a metric: zero only on identity, symmetric, and obeys the
    /// triangle inequality. Gates compare against thresholds assuming all
    /// three.
    #[test]
    fn delta_e_is_a_metric(a in any_oklch(), b in any_oklch(), c in any_oklch()) {
        let (a, b, c) = (a.to_oklab(), b.to_oklab(), c.to_oklab());
        prop_assert!(delta_e_ok(a, a) < 1e-15);
        prop_assert!((delta_e_ok(a, b) - delta_e_ok(b, a)).abs() < 1e-12);
        prop_assert!(delta_e_ok(a, c) <= delta_e_ok(a, b) + delta_e_ok(b, c) + 1e-9);
    }

    // --- Contrast ---------------------------------------------------------

    /// APCA's sign is its polarity, and the polarity follows luminance order.
    /// Roles depend on the sign to know whether they are text or surface.
    #[test]
    fn apca_sign_follows_polarity(text in any_rgb(), background in any_rgb()) {
        let lc = apca(text, background);
        prop_assert!(lc.is_finite());

        // Only judge the sign once the pair is clearly separated; APCA
        // deliberately clips near-equal luminances to exactly zero.
        prop_assume!(lc.abs() > 1.0);

        let luminance = |c: Rgb| 0.2126 * c.r.powf(2.4)
            + 0.7152 * c.g.powf(2.4)
            + 0.0722 * c.b.powf(2.4);
        if lc > 0.0 {
            prop_assert!(luminance(background) > luminance(text), "positive Lc {lc}");
        } else {
            prop_assert!(luminance(background) < luminance(text), "negative Lc {lc}");
        }
    }

    /// A color has no contrast against itself, whatever the color.
    #[test]
    fn nothing_contrasts_with_itself(rgb in any_rgb()) {
        prop_assert_eq!(apca(rgb, rgb), 0.0);
        prop_assert!((wcag21(rgb, rgb) - 1.0).abs() < 1e-9);
    }

    /// WCAG ratios stay inside their defined range. Reported, never solved
    /// against, but the report should not be nonsense.
    #[test]
    fn wcag_stays_within_its_defined_range(a in any_rgb(), b in any_rgb()) {
        let ratio = wcag21(a, b);
        prop_assert!((1.0..=21.0).contains(&ratio), "ratio {ratio}");
        prop_assert!((ratio - wcag21(b, a)).abs() < 1e-12, "not symmetric");
    }

    // --- Colour vision deficiency ----------------------------------------

    /// Simulation is a projection, so applying it twice changes nothing new —
    /// wherever the projection lands inside sRGB.
    ///
    /// Saturated colors project *outside* the cube and are clamped back, and a
    /// clamped result is no longer on the dichromatic surface, so it is not
    /// covered by this law. See `simulate`'s documentation for why clamping is
    /// nonetheless the right choice there.
    #[test]
    fn cvd_simulation_is_idempotent_where_the_projection_fits(
        rgb in any_rgb(), deficiency in any_cvd()
    ) {
        let color = rgb_to_oklch(rgb, Gamut::Srgb).to_oklab();
        let once = noctua_core::simulate(color, deficiency, 1.0);

        // A channel resting exactly on a limit is the fingerprint of a clamp.
        let linear = Gamut::Srgb.oklab_to_linear(once);
        let untouched = [linear.r, linear.g, linear.b]
            .iter()
            .all(|v| (0.002..0.998).contains(v));
        prop_assume!(untouched);

        let twice = noctua_core::simulate(once, deficiency, 1.0);
        prop_assert!(
            delta_e_ok(once, twice) < 0.01,
            "{}: {:?} then {:?}", deficiency.id(), once, twice
        );
    }

    /// Even where clamping does occur, the second application moves the color
    /// far less than the first did. Simulation contracts; it never oscillates.
    ///
    /// A color already on the dichromatic surface is the awkward case: both
    /// passes then move it by nothing, and "nothing" is a float round-trip
    /// through two matrices and a clamp rather than a clean zero. Which of two
    /// quantities near 2e-6 is larger says something about rounding and
    /// nothing about the simulator, so the claim is that the second pass
    /// either contracts or is imperceptible — at a floor four orders of
    /// magnitude below a just-noticeable difference.
    #[test]
    fn cvd_simulation_contracts(rgb in any_rgb(), deficiency in any_cvd()) {
        const IMPERCEPTIBLE: f64 = 1e-4;

        let color = rgb_to_oklch(rgb, Gamut::Srgb).to_oklab();
        let once = noctua_core::simulate(color, deficiency, 1.0);
        let twice = noctua_core::simulate(once, deficiency, 1.0);

        let first = delta_e_ok(color, once);
        let second = delta_e_ok(once, twice);
        prop_assert!(
            second <= first.max(IMPERCEPTIBLE),
            "{}: second pass moved {second} but first moved {first}",
            deficiency.id()
        );
    }

    /// Simulation stays bounded: it never wildly inflates the distance between
    /// two colors.
    ///
    /// Note what this deliberately does *not* claim. "Dichromacy can only
    /// merge colors, never separate them" sounds obvious and is false. The
    /// projection contracts distances in *linear RGB*, but delta-E is measured
    /// in Oklab, which is a nonlinear function of it — and the projection is
    /// piecewise, with the two half-planes applying different matrices to the
    /// two colors. Both effects can stretch a pair apart. A sweep of 10648
    /// colors puts the worst real expansion at 1.34x, under protanopia.
    ///
    /// So this is a smoke test, not a law: a mistyped projection matrix would
    /// blow far past the bound, while the genuine expansion sits well inside
    /// it.
    #[test]
    fn cvd_separation_stays_bounded(
        a in any_rgb(), b in any_rgb(), deficiency in any_cvd()
    ) {
        let (a, b) = (
            rgb_to_oklch(a, Gamut::Srgb).to_oklab(),
            rgb_to_oklch(b, Gamut::Srgb).to_oklab(),
        );
        let normal = delta_e_ok(a, b);
        let simulated = separation(a, b, deficiency, 1.0);

        prop_assert!(
            simulated <= normal * 1.6 + 0.02,
            "{}: {simulated} is far beyond normal {normal}", deficiency.id()
        );
    }

    /// Neutrals look the same to everyone. If a gray acquired a tint under
    /// simulation, the tinted-neutral machinery could not be trusted.
    #[test]
    fn grays_are_untouched_by_every_deficiency(l in 0.0..=1.0_f64, deficiency in any_cvd()) {
        let gray = Oklch { l, c: 0.0, h: 0.0 }.to_oklab();
        let simulated = noctua_core::simulate(gray, deficiency, 1.0);
        prop_assert!(
            delta_e_ok(gray, simulated) < 5e-3,
            "{} tinted a gray at l={l}: {:?}", deficiency.id(), simulated
        );
    }

    /// Zero severity is exactly normal vision.
    #[test]
    fn zero_severity_changes_nothing(rgb in any_rgb(), deficiency in any_cvd()) {
        let color = rgb_to_oklch(rgb, Gamut::Srgb).to_oklab();
        prop_assert!(delta_e_ok(color, noctua_core::simulate(color, deficiency, 0.0)) < 1e-9);
    }

    // --- Totality ---------------------------------------------------------

    /// No input produces a NaN. Every value here feeds a comparison somewhere,
    /// and a NaN would silently pass a threshold rather than fail it.
    #[test]
    fn no_conversion_ever_yields_a_nan(color in any_oklch(), gamut in any_gamut()) {
        let lab: Oklab = color.to_oklab();
        prop_assert!(lab.l.is_finite() && lab.a.is_finite() && lab.b.is_finite());

        let xyz = lab.to_xyz();
        prop_assert!(xyz.x.is_finite() && xyz.y.is_finite() && xyz.z.is_finite());

        let unmapped = oklab_to_rgb_unmapped(lab, gamut);
        prop_assert!(unmapped.r.is_finite() && unmapped.g.is_finite() && unmapped.b.is_finite());

        let linear = gamut.oklab_to_linear(lab);
        prop_assert!(linear.r.is_finite() && linear.g.is_finite() && linear.b.is_finite());

        prop_assert!(gamut.max_chroma(color.l, color.h).is_finite());
    }
}
