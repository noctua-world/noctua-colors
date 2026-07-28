# Token stability policy

What you may depend on, and what a version bump means.

This document exists because SemVer alone is not enough for a colour system. A
library's public API is its function signatures. **This project's public API is a
set of names** — `--nc-color-surface`, `accent::SOLID`,
`$nc-ochre-balanced-light-neutral-bg-app` — and applications write those names
directly into their stylesheets. A rename is as breaking as deleting a function,
and no compiler will tell the consumer.

## Which version this document governs

This repository publishes two things, on two clocks, and **everything below is
about the first**:

| | Declared in | Where you see it | Governed here? |
|---|---|---|---|
| **The colour system** — the palettes, the names, the values | `specs/noctua.toml`'s `[system]` | npm, crates.io, the tag, `MANIFEST.json`'s `systemVersion` | **Yes** |
| **The compiler** that generated it | `Cargo.toml`'s `[workspace.package]` | `MANIFEST.json`'s `version`, `cargo xtask --version` | No — it is not published |

They were one number through 0.1.x. One number cannot say both "the compiler was
refactored" and "a colour moved", and only the second is a reason for anyone to
upgrade. Under one number, every internal cleanup looked from the outside exactly
like a change to the colours.

So a tag means the colour system. `cargo xtask release <version>` bumps it;
`--tool <version>` bumps the compiler and publishes nothing.

`MANIFEST.json` carries **both**, under two keys. `version` still means the
compiler's, unchanged since 0.1.0, because this document tells you to diff that
file — adding a key is safe, repurposing one silently changes what your diff
means.

## What is public API

**Token names are public API.** All of them:

| Layer | Example | Stability |
|---|---|---|
| Semantic contexts | `--nc-color-surface`, `--nc-color-rejected-bg` | **Public.** Renaming or removing one is a major bump |
| Palette steps | `--nc-accent-solid`, `--nc-danger-bg-subtle` | **Public** |
| Numbered steps | `--nc-accent-1` … `-12` | **Public** |
| Neutral ramps | `--nc-gray-1` … `-24`, and `-cool` / `-warm` | **Public** |
| Alpha ladder | `--nc-neutral-a1` … `-a12`, `--nc-accent-a*` | **Public** |
| Scales | `--nc-chart-3`, `--nc-level-7`, `--nc-magnitude-high` | **Public** |
| Theme selectors | `[data-palette="blue-vivid"]`, `[data-theme="dark"]`, `.light` / `.dark` | **Public** |
| Tailwind utilities | `bg-surface`, `text-fg-muted`, `ring-ring` | **Public** |
| Rust paths | `ochre_balanced::light::accent::SOLID` | **Public** |
| Cargo features | `ochre_balanced`, `all` | **Public** |
| npm subpaths | `@noctua-world/colors/css/index.css`, `/tailwind` | **Public** |
| DTCG token structure | `$type`, `$value.colorSpace`, `$value.components` | **Public** |
| File names in `system/` | `css/ochre-balanced.css`, `tokens/ochre-balanced-light.json` | **Public** — people link them directly |
| The output directory itself | `system/` | **Public** on every route that is not npm |

The last row is why `dist/` → `system/` in 0.2.0 is a major change, and it is
worth being precise about who it reaches. npm and any CDN that resolves through
npm's `exports` map are **unaffected** — `@noctua-world/colors/css/index.css`
never named the directory. The routes that do name it are jsDelivr-by-path, a
Nix `src`, a submodule or plain copy, and a Cargo path dependency. All four are
version-pinned, so `v0.1.0` and `v0.1.1` keep serving `dist/` for as long as
those tags exist. Nothing that works today stops working; only new tags move.

**Not public API:**

- The compiler's Rust crates. They are `publish = false` and their API is shaped
  entirely by one job.
- `system/reports/*` — measurements, regenerated freely.
- The `$extensions` bag in DTCG tokens beyond `colors.noctua.relativeChroma`.
- `specs/noctua.toml`'s internal structure. It is the input, not the interface.
- The documentation site's markup and its `data-` attributes.

## What each kind of change is

**Major** — anything that can break a consumer silently:

- Renaming or removing a token, a family, a scale, a role, or a Cargo feature.
- Renaming a file under `system/`, the output directory itself, or an npm subpath.
- Changing the DTCG value shape, or the `colorSpace` a value is expressed in.
- Changing what a theme selector is spelled as.
- Raising the minimum Rust version, or the minimum Node version.

**Minor** — additive, and safe to take without reading:

- A new semantic context, family, palette, scale or Cargo feature.
- A new emitted target or a new file under `system/`.
- A new npm subpath export.
- **A changed colour value**, within the rules below.

**Patch** — a fix that changes no name and no value: documentation, a report, a
gate's message, generated-file formatting.

## Colour values change in minor versions, and here is the contract

This is the part people are surprised by, so it is stated plainly: **the numeric
value behind a token is not frozen.** The repository versions the curves, not the
colours. Retuning a curve, correcting a hue drift, or fixing a gamut-mapping bug
changes hex values without changing a single name.

What *is* guaranteed across a minor version:

1. **The contrast floor holds.** Every pair the APCA gate checks continues to meet
   its target. A change that lowered a measured contrast below its target would
   be a defect, not a release.
2. **The meaning holds.** `danger` stays a red, `success` stays a green,
   `--nc-color-surface` stays the page background. A token's *role* is part of its
   name.
3. **Monotonicity holds.** Ramps stay ordered in lightness. Indexing into
   `--nc-gray-*` stays safe.
4. **Direction holds.** A step that was darker than another in light mode stays
   darker.

What is **not** guaranteed: an exact hex value, and therefore anything you
hard-coded from a screenshot or a colour picker. If you need a value frozen
forever, pin the version — that is what versions are for.

**If you have a visual regression test that snapshots pixels, pin an exact
version.** That is not a defect in this project; it is what a pixel snapshot
means.

## Pre-1.0

While the version is `0.x`, **minor bumps may break.** Cargo and npm both treat
`0.x` as "every minor is a potential major", and this project uses that: the
first releases will refine names as real consumers appear, and it is better to
fix a name at 0.2 than to carry it to 2.0.

The rules above are the *intent* now and the *commitment* from 1.0. Pin exactly
(`=0.2.0`, or a lockfile) if you need stability before then.

## Deprecation

From 1.0, a token is never removed without first being kept as an alias for at
least one minor version, with the replacement named in `CHANGELOG.md`. An alias
costs five `var()` references, so there is no reason to be stingy about it.

## How to find out what changed

- [`CHANGELOG.md`](CHANGELOG.md) — what changed and why, per version.
- `system/MANIFEST.json` — a BLAKE3 hash per generated file, the spec's hash,
  and both versions: `systemVersion` (the colours) and `version` (the compiler).
  Diff two versions' manifests and you know exactly which artifacts moved,
  without downloading them.
- `system/reports/wcag.md` and `system/reports/colour-vision.md` — every measurement,
  regenerated each build.
