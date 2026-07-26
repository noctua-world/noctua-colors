//! Consuming the generated token crate from another project.
//!
//! This is the whole integration: one path dependency, then `use`. There is
//! no build script, no macro, no runtime, and no initialization — every color
//! is a `const`, so this program does its work at compile time and prints the
//! results.
//!
//! It also stands in as a test of the export path. If a rename in the emitter
//! breaks a consumer, it breaks here, in this repository, rather than in a
//! sibling three weeks later.

use noctua_colors_tokens::{Color, blue_balanced, gray, ochre_balanced, ochre_sober, ochre_vivid};

fn main() {
    println!("noctua-colors — consuming the generated crate\n");

    surfaces();
    themes();
    packed_colors();
    the_neutral_ramp();
}

/// The tokens a real interface reaches for first.
fn surfaces() {
    use ochre_balanced::light::neutral;

    println!("light mode surfaces");
    for (name, color) in [
        ("bg-app", neutral::BG_APP),
        ("bg-subtle", neutral::BG_SUBTLE),
        ("bg-element", neutral::BG_ELEMENT),
        ("border-subtle", neutral::BORDER_SUBTLE),
        ("text-strong", neutral::TEXT_STRONG),
    ] {
        describe(name, color);
    }
    println!();
}

/// The same token across the palette grid.
///
/// This is what the compiler buys: `accent/solid` means one thing, and every
/// palette is a resolution of it rather than a hand-maintained copy that
/// drifts. The grid is two axes — an accent hue and a saturation — so the
/// three below differ only in saturation and the fourth only in hue.
fn themes() {
    println!("accent/solid, one token across the grid");
    describe("ochre / balanced", ochre_balanced::light::accent::SOLID);
    describe("ochre / vivid", ochre_vivid::light::accent::SOLID);
    describe("ochre / sober", ochre_sober::light::accent::SOLID);
    describe("blue  / balanced", blue_balanced::light::accent::SOLID);
    println!();
}

/// `packed()` is `const`, so a lookup table costs nothing at runtime.
fn packed_colors() {
    const ACCENT: u32 = ochre_balanced::dark::accent::SOLID.packed();
    const DANGER: u32 = ochre_balanced::dark::danger::SOLID.packed();

    println!("packed for a graphics API");
    println!("  accent  0x{ACCENT:06x}");
    println!("  danger  0x{DANGER:06x}\n");
}

/// The neutral ramp is shared by every theme, and is monotone in lightness by
/// construction — which is what makes indexing into it safe.
fn the_neutral_ramp() {
    let ramp = [
        gray::STEP_1,
        gray::STEP_6,
        gray::STEP_12,
        gray::STEP_18,
        gray::STEP_24,
    ];

    println!("neutral ramp, every sixth step");
    for color in ramp {
        println!("  {}  L {:.3}", color.hex, color.l);
    }

    let rising = ramp.windows(2).all(|pair| pair[0].l < pair[1].l);
    println!("\nlightness increases along the ramp: {rising}");
    assert!(rising, "the neutral ramp must be monotone");
}

fn describe(name: &str, color: Color) {
    println!(
        "  {name:<17} {}  oklch({:.3} {:.4} {:.1})",
        color.hex, color.l, color.c, color.h
    );
}
