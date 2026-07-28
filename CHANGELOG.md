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

## [0.2.1]

No colour changed. One file was missing, and its absence was the kind that only
shows up when someone builds on top of the system.

### Added

- **`system/css/theme-ochre-balanced.css`** — the default palette in its
  scopable form. Every other palette was emitted twice: once bound to `:root`
  and once to `[data-palette="<name>"]`. The default was emitted only at
  `:root`, which made it **the one palette of thirty-nine that could not be
  applied to a subtree**: `<div data-palette="ochre-balanced">` matched nothing
  and the element silently inherited whatever the page was on.

  Falling back to `:root` is not a substitute. It works only while the page
  itself is on the default; inside a page set to another palette, a subtree
  asking for the default got the page's colours. That is invisible until
  someone builds a component with its own theme, and then it reads as the
  component ignoring its own setting.

  `css/index.css` now imports both forms of the default.

  Additive: a new file, nothing renamed, no token moved. Two new tests assert
  that **every** palette in the spec has a scopable file and that the file
  actually binds to the attribute rather than to `:root` — the second because
  the first would pass on a file that existed but was scoped wrongly.

## [0.2.0]

**The reframing.** This release changes almost nothing about the colours and
almost everything about how they are presented, installed and versioned. Of the
250 generated files that existed at 0.1.1, **245 are byte-identical**; the five
that differ carry the spec hash or a path in a comment, and not one changed line
contains a colour.

### Added

- **A self-contained stylesheet for every palette**, at
  `system/css/palette/<name>.css`. One `<link>` and you are done: it carries the
  neutral ramp, the semantic contract and that palette's values, bound to
  `:root`, with light and dark both working from the one file.

  This is the difference between linking **29 KB gzip and 741 KB gzip** — 25×.
  The only entry point that existed carried all 39 palettes, because it has to
  serve someone who switches at runtime; someone who had settled on one palette
  was downloading the other 38.

  | | raw | gzip |
  |---|---|---|
  | `css/palette/blue-vivid.css` | 229 KB | **29 KB** |
  | every palette (`index.css` and its imports) | 5.0 MB | 741 KB |

- **`system/tailwind/palette/<name>.css`** — the same, for Tailwind v4. Two
  `@import` lines each. Compiled against Tailwind 4.3.3 it produces **byte-for-byte
  the same utility rules** as the all-palettes entry, at 31 KB gzip instead of
  702 KB, `dark:` variant included.
- **`system/tailwind/bridge.css`** — the `@theme inline` mapping, lifted out of
  `theme.css` so it is written once rather than 40 times. It is palette-independent:
  the `--color-*` names never vary, only the values behind them. Inlining it into
  each entry would have cost ~4.3 MB of byte-identical text.
- npm subpaths for all of it: `./palette/*.css`, `./css/palette/*.css`,
  `./tailwind/palette/*.css`, `./tailwind/bridge.css`.
- **An `Install` section on the documentation site**, which it did not have. The
  site opened straight into an explanation of relative chroma; someone who
  wanted a stylesheet had no way to get one without reading the model first.
- **`/how-it-works.html`** — the compiler's own page, in both locales. The site
  was one page on the reasoning that a reference is scrolled rather than
  navigated. That was right about the reference and wrong about the audience.
  Nothing was cut: the engineering material moved and gained room, including a
  new section on what the quality gates measured *and could not fix*.

### Removed

- `package-lock.json`, now gitignored. A lockfile for a package with **zero**
  dependencies records nothing, and it was a sixth place a version could drift —
  it already had: it said `0.1.0` while every other artifact said `0.1.1`.
  Nothing wrote it and nothing read it.

### Changed

- **The colour system and the compiler now have separate versions.** The one you
  install — npm, crates.io, the tag — is the **colour system's**, declared in
  `specs/noctua.toml`'s new `[system]` table. The compiler keeps its own in
  `Cargo.toml`, where nothing publishes from it.

  One number could not say both "the compiler was refactored" and "a colour
  moved", and only the second is a reason for anyone to upgrade or is bound by
  `TOKEN-POLICY.md`. `MANIFEST.json` now carries both: `systemVersion` is new and
  is the colours; **`version` still means the compiler's**, unchanged, because
  `TOKEN-POLICY.md` tells consumers to diff that file.

  `cargo xtask release <version>` bumps the colour system; `--tool <version>`
  bumps the compiler.
- **The framing.** This repository was described as a *colour system compiler* —
  a tool. That was half the truth, and the half that does not matter to almost
  anyone who arrives. It publishes **two** things: a colour system, which is the
  product, and the compiler that produced it, which is the proof. `README.md` is
  rewritten from zero around the first; the engineering material moved intact to
  [`docs/COMPILER.md`](docs/COMPILER.md) and gained room rather than losing it.
  Contributor material moved to `CONTRIBUTING.md`.
- **`dist/` is now `system/`.** Breaking for the routes that name the directory,
  which is every route except npm. See below for exactly who this reaches.
- **`tailwind/theme.css` is now two `@import` lines** rather than the mapping
  inline. Same tokens, same utilities, same `dark:` variant — verified against
  Tailwind 4.3.3 — because the mapping it used to carry now lives in
  `bridge.css`, which it imports.
- The npm package is **22.2 MiB unpacked, 2.38 MiB packed** (was 13.5 / 1.33),
  entirely because of the 78 new per-palette files. The CI size ceiling moved
  from 20 to 32 MiB to match; it exists to catch the whole tree leaking in, not
  to police deliberate growth.
- **`cargo xtask build` no longer writes the published colour system.** It writes
  `target/system/`, a gitignored scratch tree, and `cargo xtask dev` serves from
  there. Publishing is now a typed intent: `cargo xtask build --system`.

  This closes a real hole rather than a hypothetical one. `dist/` was both the
  product people install *and* the directory every build overwrote, so trying a
  hue out rewrote the shipped colours in the working tree, and the only thing
  between that and a commit was noticing a 250-file diff. The guard is symmetric:
  `cargo xtask check` still verifies `system/` against the spec, so the opposite
  mistake — meaning a change and forgetting to publish it — fails in CI.

  No colour moved. Of the 250 generated files, 245 are byte-identical to 0.1.1;
  the five that differ carry the spec hash or a path in a comment, and not one
  changed line contains a colour.

#### Who the rename reaches

| Route | Affected? |
|---|---|
| npm, and CDNs resolving through npm's `exports` | **No.** `@noctua-world/colors/css/index.css` never named the directory |
| crates.io `noctua-colors-tokens` | **No.** The crate is published from the directory, not by it |
| jsDelivr / unpkg **by path** | Yes — `.../dist/css/…` becomes `.../system/css/…` |
| Nix `src`, submodule, subtree, plain copy | Yes |
| Cargo path or git dependency on the generated crate | Yes — `dist/rust` becomes `system/rust` |
| GitHub Release assets | Renamed `…-dist.tar.gz` → `…-system.tar.gz` |

Every affected route is version-pinned, so **`v0.1.0` and `v0.1.1` keep serving
`dist/` for as long as those tags exist.** Nothing that works today stops
working; only new tags move.

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

[Unreleased]: https://github.com/noctua-world/noctua-colors/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/noctua-world/noctua-colors/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/noctua-world/noctua-colors/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/noctua-world/noctua-colors/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/noctua-world/noctua-colors/releases/tag/v0.1.0
