# Token stability policy

What you may depend on, and what a version bump means.

This document exists because SemVer alone is not enough for a colour system. A
library's public API is its function signatures. **This project's public API is a
set of names** — `--nc-color-surface`, `accent::SOLID`,
`$nc-ochre-balanced-light-neutral-bg-app` — and applications write those names
directly into their stylesheets. A rename is as breaking as deleting a function,
and no compiler will tell the consumer.

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
| File names in `dist/` | `css/ochre-balanced.css`, `tokens/ochre-balanced-light.json` | **Public** — people link them directly |

**Not public API:**

- The compiler's Rust crates. They are `publish = false` and their API is shaped
  entirely by one job.
- `dist/reports/*` — measurements, regenerated freely.
- The `$extensions` bag in DTCG tokens beyond `colors.noctua.relativeChroma`.
- `specs/noctua.toml`'s internal structure. It is the input, not the interface.
- The documentation site's markup and its `data-` attributes.

## What each kind of change is

**Major** — anything that can break a consumer silently:

- Renaming or removing a token, a family, a scale, a role, or a Cargo feature.
- Renaming a file under `dist/`, or an npm subpath.
- Changing the DTCG value shape, or the `colorSpace` a value is expressed in.
- Changing what a theme selector is spelled as.
- Raising the minimum Rust version, or the minimum Node version.

**Minor** — additive, and safe to take without reading:

- A new semantic context, family, palette, scale or Cargo feature.
- A new emitted target or a new file under `dist/`.
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
(`=0.1.0`, or a lockfile) if you need stability before then.

## Deprecation

From 1.0, a token is never removed without first being kept as an alias for at
least one minor version, with the replacement named in `CHANGELOG.md`. An alias
costs five `var()` references, so there is no reason to be stingy about it.

## How to find out what changed

- [`CHANGELOG.md`](CHANGELOG.md) — what changed and why, per version.
- `dist/MANIFEST.json` — a BLAKE3 hash per generated file, plus the spec's hash.
  Diff two versions' manifests and you know exactly which artifacts moved,
  without downloading them.
- `dist/reports/wcag.md` and `dist/reports/colour-vision.md` — every measurement,
  regenerated each build.
