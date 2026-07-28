//! **A colour system compiler.** A small declarative specification goes in;
//! every artifact other projects consume comes out.
//!
//! # This crate contains no code
//!
//! It exists so that the project's name on crates.io belongs to the project.
//! There is deliberately nothing to call here, and depending on it does nothing.
//! What you probably want is one of these instead:
//!
//! | You are | Use |
//! |---|---|
//! | Painting in Rust — Dioxus, egui, a TUI, a graphics API | [`noctua-colors-tokens`](https://crates.io/crates/noctua-colors-tokens) |
//! | Styling a web view or a website | the CSS custom properties |
//! | Using Tailwind v4 | the generated theme, one `@import` |
//! | Running a design-token pipeline | the DTCG tokens |
//! | Writing Qt or Quickshell | the QML singleton |
//!
//! Every route is documented at
//! <https://github.com/noctua-world/noctua-colors>, and the palette is browsable
//! at <https://noctua-world.github.io/noctua-colors/>.
//!
//! # What the system actually is
//!
//! Not a palette, and not a UI library: **the repository versions the curves,
//! not the colours.** No hand-picked hex value exists anywhere in its source.
//! Two ideas carry it.
//!
//! **Relative chroma.** Chroma is a fraction of what the target gamut can show
//! at that lightness and hue, rather than an absolute number. So one definition
//! renders correctly on sRGB and more vividly on Display P3 without being
//! redefined, and "sober ↔ vivid" is a single multiplier.
//!
//! **Contrast-anchored steps.** A step's lightness is *solved* from an APCA
//! contrast target against a declared reference, not authored as a ramp. Which
//! means the contrast guarantees are a property of how the colours were made,
//! not a spreadsheet somebody checked once.
//!
//! Quality gates run before anything is emitted: an APCA contrast matrix across
//! every family, colour-vision-deficiency margins under all three dichromacies,
//! perceptual spacing, and a check that no consumer references a token that does
//! not exist. The gates publish their measurements rather than a verdict —
//! including the ones no palette can pass, because a limit you can see is worth
//! more than a claim you cannot check.
//!
//! # Why the compiler is not published
//!
//! Its crates are marked `publish = false`. They are an implementation of one
//! specification, not a general-purpose colour library, and their API is shaped
//! entirely by that job — publishing them would promise a stability nobody
//! needs and nothing would benefit from. The output is the product.
//!
//! # Licence
//!
//! MIT OR Apache-2.0.

// Nothing is exported. The crate is documentation, and a reader who lands here
// looking for an API should be sent somewhere useful rather than given an empty
// module to wonder about.
#![no_std]
