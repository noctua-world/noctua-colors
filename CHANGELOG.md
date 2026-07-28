# Changelog

Every notable change to this project. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow the
stability policy in [`TOKEN-POLICY.md`](TOKEN-POLICY.md) — which is stricter than
plain SemVer about token *names*, because those are what applications write CSS
against.

This file exists because a commit log cannot answer the question a consumer of a
colour system actually has: **which colour changed, and does my interface still
meet contrast?** Generated release notes list commits. They do not list that.

## [Unreleased]

## [0.1.0]

The first published version. Everything below describes what exists rather than
what changed, since there is nothing before it.

### Added

- **The compiler.** A declarative specification in `specs/noctua.toml` is
  compiled into every artifact under `dist/`, which is generated **and
  committed**, so every consumption path works with no build step.
- **Eight targets.** CSS custom properties, a Tailwind v4 theme, a `const` Rust
  crate, DTCG tokens, JSON/TypeScript, SCSS, a QML singleton, and compliance
  reports.
- **39 palettes** as a grid — 13 accent hues × 3 saturations — from eighteen
  lines of specification.
- **352 semantic contexts** over ten families, each emitted as five tokens. Ten
  families carry a hue of their own; the rest are aliases, because the hue wheel
  has room for ten and not for three hundred.
- **Two ordered scales**, four categorical sets, three neutral temperatures, and
  a translucency ladder.
- **Two gamuts.** sRGB as the base and Display P3 as an upgrade layer: the same
  token, more saturated where there is more room, redefined nowhere.
- **Five quality gates** — an APCA contrast matrix across families, colour-vision
  margins under all three dichromacies, perceptual spacing, source invariants,
  and consumer token references — run before anything is emitted.
- **A WebAssembly playground**: the compiler itself, in the browser, with the
  spec carried in the URL.
- **A palette importer** that fits an existing palette back to spec parameters
  and publishes the residual.
- **Installation through many channels**: npm, jsDelivr and unpkg, crates.io, a
  Cargo git dependency, GitHub Release tarballs with signed build provenance, a
  Nix flake, submodule/subtree/copy, and `cargo xtask export` for workspace
  siblings.

### Notes for consumers

- **DTCG tokens are conformant with Format Module 2025.10**, the first stable
  version. A colour `$value` is an object with `colorSpace` and `components`, not
  a hex string — the draft-era shape a lot of token packages still ship. OKLCH is
  the primary, lossless value and `hex` is the six-digit fallback the spec
  provides, so nothing is rounded away on the way in. Verified against Style
  Dictionary v5.
- **Colour alone cannot separate the semantic families under dichromacy**, and
  the gates measure exactly how far it gets: the best achievable worst-case
  separation for a six-family set is 0.0163, under one just-noticeable
  difference, and there are ten families. `dist/reports/colour-vision.md`
  publishes every margin. **Never convey information by colour alone** — this is
  measured, not cautious.
- **The translucency ladder is real alpha**, so no contrast gate can audit it:
  contrast is a property of two opaque colours, and an alpha token has none until
  it is composited.

[Unreleased]: https://github.com/noctua-world/noctua-colors/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/noctua-world/noctua-colors/releases/tag/v0.1.0
