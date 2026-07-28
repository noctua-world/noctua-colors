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

## [0.1.1]

No colour changed. This release exists to prove the automated publishing path
end to end: both registries reached from CI over OIDC, with no token stored
anywhere, and an npm provenance attestation — which v0.1.0 could not have,
because a trusted publisher cannot be configured for a package that does not
yet exist.

### Fixed

- The release workflow's tarball inspection assumed `npm pack --json` returns an
  array. npm 12 returns an object keyed by package name; the fields inside are
  identical. It now accepts either. This only ever failed in CI, which installs
  `npm@latest` because trusted publishing requires >= 11.5.1.
- Both release workflows now decide whether to publish from **the registry's
  answer** rather than from a prior lookup. The read path lags the write path —
  observed during the v0.1.0 bootstrap, where `npm view` returned 404 while a
  publish was rejected for already existing — so "not readable" never means "not
  published".
- `cargo xtask check` passes on a fresh clone. A documentation test probed the
  rendered site, which is gitignored and written only by `cargo xtask build`, so
  it passed only on a machine that had built recently.
- `cargo xtask export` announces writes outside the repository again.
  `Path::starts_with` is lexical and does not understand `..`, so every
  `../sibling` consumer — the case the notice exists for — was silently treated
  as inside.
- `cargo xtask release` regenerates `dist/` in a freshly compiled subprocess.
  The version reaches the artifacts through `env!("CARGO_PKG_VERSION")`, which is
  baked in when the binary is compiled, so regenerating in-process stamped them
  with the version that had just been replaced. Found the first time the verb was
  ever run, and now covered by a test that asserts all five places carrying the
  version agree.


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

### Known, and deliberate

- **v0.1.0 carries no npm provenance attestation.** Provenance requires OIDC from
  CI, CI requires a trusted publisher, and a trusted publisher requires the
  package to already exist — so the first publish of any package is necessarily
  manual and unattested. Every release after this one is attested. The
  alternative was burning a throwaway version to create the package, which would
  have sat in the version history forever.
- **The v0.1.0 release workflows show as failed.** Both packages were published by
  hand for the reason above; the workflows could not authenticate to registries
  that had no trusted publisher yet. The artifacts are correct and verified — see
  the release assets' checksums and `npm audit signatures`.

[Unreleased]: https://github.com/noctua-world/noctua-colors/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/noctua-world/noctua-colors/releases/tag/v0.1.0
