//! The examples printed in `README.md`, compiled and run.
//!
//! A README describing an API that no longer exists is worse than no README,
//! so these are kept identical to what is published there. If one needs
//! changing, change both.

use noctua_core::cvd::worst_separation;
use noctua_core::map::from_hex;
use noctua_core::{Gamut, Oklch, apca, map_into_gamut, to_hex, wcag21};

#[test]
fn quick_start_example_produces_a_colour() {
    // Ask for 90% of the most chroma sRGB can show at this lightness and hue.
    let hue = 264.0;
    let lightness = 0.62;
    let max = Gamut::Srgb.max_chroma(lightness, hue);

    let color = Oklch {
        l: lightness,
        c: max * 0.9,
        h: hue,
    };
    let mapped = map_into_gamut(color, Gamut::Srgb);

    // The figures quoted in the README.
    assert!((max - 0.2043).abs() < 0.001, "max chroma drifted: {max}");
    assert!(
        (mapped.oklch.c - 0.1839).abs() < 0.001,
        "chroma drifted: {}",
        mapped.oklch.c
    );
    assert_eq!(to_hex(mapped.rgb).len(), 7);
}

#[test]
fn contrast_and_cvd_example_matches_the_published_figures() {
    let fg = from_hex("#767676").expect("valid hex"); // allow-literal: published reference pair, quoted in README.md
    let bg = from_hex("#ffffff").expect("valid hex");

    let lc = apca(fg, bg);
    let ratio = wcag21(fg, bg);
    assert!((lc - 71.6).abs() < 0.1, "APCA drifted: {lc}");
    assert!((ratio - 4.54).abs() < 0.01, "WCAG drifted: {ratio}");

    let a = Oklch {
        l: 0.55,
        c: 0.15,
        h: 25.0,
    }
    .to_oklab();
    let b = Oklch {
        l: 0.55,
        c: 0.15,
        h: 145.0,
    }
    .to_oklab();
    let (_deficiency, margin) = worst_separation(a, b);
    assert!(
        (margin - 0.009).abs() < 0.002,
        "CVD margin drifted: {margin}"
    );
}

/// The comparison table in the README: WCAG rates the dark pair better, APCA
/// rates it worse. If this ever stops holding, the table is wrong.
#[test]
fn the_wcag_versus_apca_table_still_holds() {
    let light = (from_hex("#767676").unwrap(), from_hex("#ffffff").unwrap()); // allow-literal: published reference pair, quoted in README.md
    let dark = (from_hex("#9a9a9a").unwrap(), from_hex("#000000").unwrap()); // allow-literal: published reference pair, quoted in README.md

    assert!((wcag21(light.0, light.1) - 4.54).abs() < 0.01);
    assert!((wcag21(dark.0, dark.1) - 7.46).abs() < 0.01);
    assert!((apca(light.0, light.1) - 71.6).abs() < 0.1);
    assert!((apca(dark.0, dark.1) - -47.7).abs() < 0.1);

    assert!(
        wcag21(dark.0, dark.1) > wcag21(light.0, light.1),
        "WCAG should prefer the dark pair"
    );
    assert!(
        apca(dark.0, dark.1).abs() < apca(light.0, light.1).abs(),
        "APCA should prefer the light pair"
    );
}
