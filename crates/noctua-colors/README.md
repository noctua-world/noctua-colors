# noctua-colors

**A colour system compiler.** A small declarative specification goes in; every
artifact other projects consume comes out — CSS custom properties, a Tailwind v4
theme, DTCG tokens, SCSS, JSON/TypeScript, a QML singleton, and a `const` Rust
crate.

## This crate contains no code

It holds the project's name on crates.io. There is nothing to call, and depending
on it does nothing. What you want is almost certainly one of these:

| You are | Use |
|---|---|
| Painting in Rust — Dioxus, egui, a TUI, a graphics API | [`noctua-colors-tokens`](https://crates.io/crates/noctua-colors-tokens) |
| Styling a web view or a website | the CSS custom properties |
| Using Tailwind v4 | the generated theme, one `@import` |
| Running a design-token pipeline | the DTCG 2025.10 tokens |
| Writing Qt or Quickshell | the QML singleton |

**Every installation route is documented in the
[repository](https://github.com/noctua-world/noctua-colors)**, and the palette is
browsable at <https://noctua-world.github.io/noctua-colors/>.

## What the system is

Not a palette, and not a UI library: **the repository versions the curves, not
the colours.** No hand-picked hex value exists anywhere in its source.

- **Relative chroma** — chroma is a fraction of what the target gamut can show at
  that lightness and hue, so one definition renders correctly on sRGB and more
  vividly on Display P3 without being redefined, and "sober ↔ vivid" is one
  multiplier.
- **Contrast-anchored steps** — a step's lightness is *solved* from an APCA
  contrast target against a declared reference, not authored as a ramp. The
  contrast guarantees are therefore a property of how the colours were made.

Quality gates run before anything is emitted: an APCA contrast matrix across
every family, colour-vision margins under all three dichromacies, perceptual
spacing, and a check that no consumer references a token that does not exist.
They publish their measurements rather than a verdict — including the limits no
palette can pass, because a limit you can see is worth more than a claim you
cannot check.

## Why the compiler itself is not published

Its crates are `publish = false`. They implement one specification rather than
being a general-purpose colour library, and their API is shaped entirely by that
job. The output is the product.

## Licence

MIT OR Apache-2.0.
