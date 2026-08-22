<div align="center">

# noctua-colors

**A colour system whose colours were solved, not picked.**

39 palettes · 1,767 semantic names · light and dark from one file ·
48,441 contrast pairs checked on every build

[Browse the palettes](https://noctua-world.github.io/noctua-colors/) ·
[Install](#install-in-one-minute) ·
[How it was made](docs/COMPILER.md)

</div>

---

Most colour systems are a list of hex values somebody chose. This one is the
**output of a compiler**. You describe a hue; a program solves every step of
every ramp against a perceptual contrast target, in every palette, and checks
all 48,441 resulting pairs before anything ships.

You do not have to care about any of that to use it. You link one file.

```html
<link rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/palette/blue-vivid.css">
```

That is a complete install. You now have `--nc-color-surface`, `--nc-color-fg`,
`--nc-color-accent`, `--nc-color-danger` and 1,763 more — and dark mode already
works.

---

## Contents

- [Install in one minute](#install-in-one-minute)
- [Use it](#use-it)
- [Choose a palette](#choose-a-palette)
- [The names you get](#the-names-you-get)
- [Light and dark](#light-and-dark)
- [Why these colours are trustworthy](#why-these-colours-are-trustworthy)
- [Every way to install](#every-way-to-install)
- [Two versions, and which one is yours](#two-versions-and-which-one-is-yours)
- [Documentation](#documentation)

---

## Install in one minute

Pick the row that matches you. Each one is complete — there is no step two.

### I just want colours in a web page

Copy this into your `<head>`:

```html
<link rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/palette/blue-vivid.css">
```

Swap `blue-vivid` for any palette in [the grid below](#choose-a-palette).

**29 KB gzipped**, and that is one palette complete: the neutral ramp, every
semantic name, light and dark. There is also a file carrying all 39, for
switching at runtime — it is 730 KB gzipped, or **25× more**, which is exactly
why the one-palette file exists.

### I use npm

```sh
npm install @noctua-world/colors
```

Then in your CSS:

```css
@import "@noctua-world/colors/palette/blue-vivid.css";
```

### I use Tailwind v4

```sh
npm install @noctua-world/colors
```

Two lines in your entry CSS, and that is the whole integration:

```css
@import "tailwindcss";
@import "@noctua-world/colors/tailwind/palette/blue-vivid.css";
```

You now write `bg-surface`, `text-fg`, `border-border`, `bg-accent`,
`text-danger`, `bg-gray-18` — and `dark:` works, following the same signals the
tokens do.

### I write Rust

```sh
cargo add noctua-colors-tokens --features blue_vivid
```

```rust
use noctua_colors_tokens::blue_vivid::light::accent;

let hex = accent::SOLID.hex();      // a &'static str
let rgb = accent::SOLID.packed();   // a u32
```

`no_std`, no dependencies, every value a `const`.

---

## Use it

### You name meanings, not colours

```css
.card {
  background: var(--nc-color-surface-raised);
  color:      var(--nc-color-fg);
  border:     1px solid var(--nc-color-border);
}

.card:focus-visible {
  outline: 2px solid var(--nc-color-ring);
}

.badge-overdue {
  background: var(--nc-color-overdue);
  color:      var(--nc-color-on-overdue);
}
```

Nothing there names a colour. Change the palette and all of it follows, because
`--nc-color-overdue` is a name for a *meaning*, not for a particular red.

### A complete page you can copy

Save this as `index.html`, open it, then switch your operating system between
light and dark appearance.

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>noctua-colors</title>
  <link rel="stylesheet"
        href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/palette/jade-balanced.css">
  <style>
    body {
      background: var(--nc-color-surface);
      color: var(--nc-color-fg);
      font-family: system-ui, sans-serif;
      margin: 0;
      padding: 3rem 1.5rem;
    }
    .card {
      max-width: 32rem;
      margin-inline: auto;
      background: var(--nc-color-surface-raised);
      border: 1px solid var(--nc-color-border);
      border-radius: 12px;
      padding: 1.5rem;
    }
    .muted { color: var(--nc-color-fg-muted); }
    /* A solid fill takes on-<name>, which was solved against it. */
    .pill {
      display: inline-block;
      padding: 0.2rem 0.7rem;
      border-radius: 999px;
      background: var(--nc-color-success);
      color: var(--nc-color-on-success);
      font-size: 0.85rem;
    }
    /* A tinted background is a different pairing: -bg with -border, and the
       page's ordinary text colour on top. */
    .note {
      margin-top: 1rem;
      padding: 0.75rem 1rem;
      border-radius: 8px;
      background: var(--nc-color-info-bg);
      border: 1px solid var(--nc-color-info-border);
    }
    button {
      background: var(--nc-color-accent);
      color: var(--nc-color-on-accent);
      border: 0;
      border-radius: 8px;
      padding: 0.6rem 1.1rem;
      font: inherit;
      cursor: pointer;
    }
    button:hover { background: var(--nc-color-accent-hover); }
  </style>
</head>
<body>
  <div class="card">
    <span class="pill">Deployed</span>
    <h1>Dark mode already works</h1>
    <p class="muted">
      One stylesheet, both modes, nothing configured.
    </p>
    <button>Primary action</button>
    <div class="note">A tinted panel, using -bg with -border.</div>
  </div>
</body>
</html>
```

---

## Choose a palette

Two axes — an **accent hue** and a **saturation** — so a palette name is always
`<accent>-<saturation>`:

| | balanced | vivid | sober |
|---|---|---|---|
| **ochre** | `ochre-balanced` | `ochre-vivid` | `ochre-sober` |
| **amber** | `amber-balanced` | `amber-vivid` | `amber-sober` |
| **lime** | `lime-balanced` | `lime-vivid` | `lime-sober` |
| **jade** | `jade-balanced` | `jade-vivid` | `jade-sober` |
| **teal** | `teal-balanced` | `teal-vivid` | `teal-sober` |
| **azure** | `azure-balanced` | `azure-vivid` | `azure-sober` |
| **blue** | `blue-balanced` | `blue-vivid` | `blue-sober` |
| **indigo** | `indigo-balanced` | `indigo-vivid` | `indigo-sober` |
| **violet** | `violet-balanced` | `violet-vivid` | `violet-sober` |
| **magenta** | `magenta-balanced` | `magenta-vivid` | `magenta-sober` |
| **rose** | `rose-balanced` | `rose-vivid` | `rose-sober` |
| **clay** | `clay-balanced` | `clay-vivid` | `clay-sober` |
| **umber** | `umber-balanced` | `umber-vivid` | `umber-sober` |

**`ochre-balanced` is the default** — the one bound to `:root` in `index.css`,
and what you get if you do not choose.

The saturation axis is a single multiplier on relative chroma. That is why it
is an axis at all, rather than 39 separately tuned palettes.
[See them side by side.](https://noctua-world.github.io/noctua-colors/)

### Switching at runtime

To let your users choose, load the all-palettes file and set an attribute:

```html
<link rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/index.css">
```

```js
document.documentElement.dataset.palette = "violet-sober";
```

This is the 730 KB-gzipped route. If you ship one palette, do not use it.

---

## The names you get

Every name below exists in every palette, in both modes.

### Page furniture

| Name | For |
|---|---|
| `--nc-color-surface` | the page background |
| `--nc-color-surface-subtle` | a slightly recessed area |
| `--nc-color-surface-raised` | cards, panels, popovers |
| `--nc-color-fg` | body text |
| `--nc-color-fg-muted` | secondary text |
| `--nc-color-border` | ordinary dividers and outlines |
| `--nc-color-border-strong` | emphasised borders |
| `--nc-color-ring` | focus rings |

### States and meanings

The contract carries **1,767 names**, covering **349 subjects**. Each subject
gives you a family:

| Pattern | Example | For |
|---|---|---|
| `--nc-color-<subject>` | `--nc-color-danger` | a solid fill |
| `--nc-color-<subject>-hover` | `--nc-color-danger-hover` | its hover state |
| `--nc-color-<subject>-bg` | `--nc-color-danger-bg` | a tinted background |
| `--nc-color-<subject>-border` | `--nc-color-danger-border` | a border to match that background |
| `--nc-color-on-<subject>` | `--nc-color-on-danger` | **text that is legible on the solid fill** |

`--nc-color-on-danger` was solved against `--nc-color-danger` to a contrast
target, so you never guess whether white or black goes on top.

**They come in two pairs, and mixing them is the one easy mistake:**

```css
/* A solid chip — the fill and the text solved against each other. */
.chip { background: var(--nc-color-danger); color: var(--nc-color-on-danger); }

/* A tinted panel — the background and its border. Ordinary page text on top. */
.panel { background: var(--nc-color-danger-bg);
         border: 1px solid var(--nc-color-danger-border); }
```

`on-danger` is a *near-white*, chosen to sit on the saturated fill. On the pale
`-bg` it would be nearly invisible — and since both are valid colours, nothing
would warn you.

A sample of the subjects:

```text
accent  danger  warning  success  info  urgent  waiting  active  progress
pending  approved  rejected  archived  draft  published  deployed  failed
expired  overdue  locked  verified  deprecated  offline  syncing  throttled
quarantined  chargeback  backordered  onboarding  ratelimited  …
```

Ordinary product vocabulary — the point is that you rarely have to invent one.
The complete list is in [`system/css/contexts.css`](system/css/contexts.css)
and on [the documentation site](https://noctua-world.github.io/noctua-colors/).

### The neutral ramp

`--nc-gray-1` through `--nc-gray-24`, plus `--nc-gray-cool-*` and
`--nc-gray-warm-*`, for when the semantic names are not granular enough.

These are **mode-independent**: `--nc-gray-4` is one colour, not two. The ramp
is a shared resource both modes draw from.

### Translucency

`--nc-neutral-a1` … `-a12` and `--nc-accent-a1` … `-a12` — washes built with
`color-mix`, so they follow whichever mode and gamut layer is active rather
than freezing a value.

---

## Light and dark

**It already works.** All three mechanisms are emitted, composed so they cannot
conflict:

| You do | It does |
|---|---|
| nothing | follows the operating system's preference |
| `<html data-theme="dark">` | forces dark |
| `<html class="dark">` | forces dark |
| either, on *any* element | switches that subtree only |

So a toggle is one line:

```js
document.documentElement.dataset.theme = wantsDark ? "dark" : "light";
```

`color-scheme` is set too, so scrollbars, form controls and the canvas match.
Without it a dark page still gets light scrollbars.

---

## Why these colours are trustworthy

Two ideas do the work. Both are covered at full depth in
[**docs/COMPILER.md**](docs/COMPILER.md) — here is what they buy you.

**Chroma is relative, not absolute.** Saturation is stored as a fraction of the
most a display can actually show at that lightness and hue. So the same token
is richer on a wide-gamut screen without being redefined anywhere, and a yellow
and a blue look equally saturated at the same step despite blue having nearly
twice the chroma available.

**Lightness is solved, not authored.** Every step's lightness is derived from a
contrast target, using **APCA** rather than WCAG 2.x. That distinction is not
academic:

| pair | WCAG 2.x | APCA |
|---|---|---|
| `#767676` on `#ffffff` | 4.54:1 — barely AA | Lc **+71.6** — comfortable |
| `#9a9a9a` on `#000000` | 7.46:1 — passes AAA | Lc **−47.7** — too weak for body text |

WCAG rates the dark-mode pair as substantially *better*. APCA, which models
polarity and the actual perceptual response, rates it markedly worse — which
matches what your eyes report. A system tuned to WCAG ships both as
equivalent. WCAG ratios are still emitted, as a compliance report; no gate
targets them.

**And it is all checked.** Every build runs 48,441 checks across every palette,
mode and pair, and fails on a regression. Colour-vision simulation runs too —
and where a pair genuinely cannot be made distinguishable, that is **published
rather than hidden**, in `system/reports/colour-vision.md`, with the measured
numbers. Twelve chart series cannot all be told apart under protanopia; no
palette can fix that, so the honest answer is a legend, and the report says so.

---

## Every way to install

| Route | Command or line |
|---|---|
| **CDN, one palette** | `<link href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/palette/blue-vivid.css">` |
| **CDN, all palettes** | `<link href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/index.css">` |
| **npm** | `npm install @noctua-world/colors` |
| **Tailwind v4** | `@import "@noctua-world/colors/tailwind/palette/blue-vivid.css";` |
| **Rust** | `cargo add noctua-colors-tokens --features blue_vivid` |
| **Cargo, from git** | `noctua-colors-tokens = { git = "https://github.com/noctua-world/noctua-colors", tag = "v0.2.0" }` |
| **SCSS** | `@use "@noctua-world/colors/scss/noctua" as noctua;` |
| **Design tokens (DTCG)** | `@noctua-world/colors/tokens/blue-vivid-light.json` |
| **TypeScript / JSON** | in the release tarball — npm leaves out the 36 MB payload |
| **Qt / Quickshell** | copy `system/qml/` from a [release](https://github.com/noctua-world/noctua-colors/releases) |
| **Nix** | `inputs.noctua-colors.url = "github:noctua-world/noctua-colors";` |
| **Submodule, subtree, plain copy** | `system/` is committed; take what you want |

### Plain CSS, from a checkout or a copy

`system/` is committed, so there is no build step:

```html
<link rel="stylesheet" href="system/css/palette/ochre-balanced.css">
```

Or the three layers separately, if you want to swap only the palette:

```html
<link rel="stylesheet" href="system/css/ramp.css">      <!-- the dense grays -->
<link rel="stylesheet" href="system/css/contexts.css">  <!-- the contract -->
<link rel="stylesheet" href="system/css/ochre-balanced.css">  <!-- the values -->
```

**Link all three, or none.** A theme file on its own defines no `--nc-gray-*`
and no `--nc-color-*`, and CSS drops an undefined custom property *without
saying so* — the page renders unstyled and the console stays empty. The
per-palette file exists so that you cannot make this mistake.

### npm subpaths

| Subpath | Contents |
|---|---|
| `@noctua-world/colors` | all palettes (`css/index.css`) |
| `@noctua-world/colors/palette/<name>.css` | one palette, self-contained |
| `@noctua-world/colors/css/<name>.css` | one theme's values only |
| `@noctua-world/colors/tailwind` | Tailwind, all palettes |
| `@noctua-world/colors/tailwind/palette/<name>.css` | Tailwind, one palette |
| `@noctua-world/colors/scss/noctua` | the SCSS entry |
| `@noctua-world/colors/tokens/<name>-<mode>.json` | DTCG tokens |
| `@noctua-world/colors/axes.json` | the accent × saturation grid |
| `@noctua-world/colors/manifest.json` | hashes and both versions |

### Tailwind v4, all palettes

```css
@import "tailwindcss";
@import "@noctua-world/colors/tailwind";
```

Utilities come out as `var()` references rather than frozen values, so they
follow the active mode. That is `@theme inline`, and it is load-bearing: a
plain `@theme` would stamp `bg-accent` with whichever mode was active at build
time, and dark mode would silently stop working.

### Rust

```toml
[dependencies]
noctua-colors-tokens = { version = "0.2.0", features = ["blue_vivid"] }
```

One Cargo feature per palette, so a program shipping two does not carry the
other 37.

```rust
use noctua_colors_tokens::blue_vivid::{dark, light};

const CARD: &str = light::neutral::BG_ELEMENT.hex();
let packed: u32 = dark::accent::SOLID.packed();
let (l, c, h) = light::accent::SOLID.oklch();
```

### Design tokens (DTCG 2025.10)

`system/tokens/<palette>-<mode>.json` is standards-compliant DTCG. Colour
values are objects with `colorSpace: "oklch"` and `components`, plus a 6-digit
`hex` fallback — which is what the 2025.10 stable specification requires, and
what tools like Style Dictionary v5 actually read.

```js
// style-dictionary config
export default {
  source: ["node_modules/@noctua-world/colors/system/tokens/blue-vivid-light.json"],
  platforms: {
    css: {
      transformGroup: "css",
      files: [{ destination: "vars.css", format: "css/variables" }],
    },
  },
};
```

### SCSS

```scss
@use "@noctua-world/colors/scss/noctua" as noctua;

.card { background: noctua.$nc-blue-vivid-light-neutral-bg-element; }
```

Values are hex, because Sass resolves at compile time and cannot follow a
`var()`. Both modes are emitted; you pick one.

### TypeScript and JSON

`system/json/palette.json` is every theme, mode, family, step, role, gamut
rendition and relative chroma as plain JSON. `system/ts/index.js` is the same
data with `.d.ts` types.

Each is ~18 MB and both are **deliberately excluded from the npm package** — they
are in the release tarball instead, because almost nobody installing colours
wants to download them. `npm pack --dry-run` is what says which files ship.

### Qt / QML

Copy `system/qml/` next to your QML, then:

```qml
import "qml"

Rectangle {
    color: BlueVividDark.surface
    border.color: BlueVividDark.border
}
```

### Nix

```nix
{
  inputs.noctua-colors.url = "github:noctua-world/noctua-colors";

  # then, in a derivation:
  installPhase = ''
    mkdir -p $out/static
    cp -r ${noctua-colors}/share/noctua-colors/css $out/static/
  '';
}
```

### Verifying what you installed

Every release is published from CI with no stored credentials, over OIDC, and
carries signed build provenance.

```sh
# npm
npm audit signatures

# release assets
gh attestation verify noctua-colors-v0.2.0-system.tar.gz \
  --repo noctua-world/noctua-colors
sha256sum -c SHA256SUMS

# any file, against the manifest
b3sum system/css/index.css   # compare with system/MANIFEST.json
```

---

## Two versions, and which one is yours

This repository publishes two things, on two clocks:

| | What it is | Where you see it |
|---|---|---|
| **The colour system** | the palettes, the names, the values | npm, crates.io, the git tag |
| **The compiler** | the program that generated them | `MANIFEST.json`'s `version` |

**The colour system's version is the one that matters to you.** A tag names it,
both registries publish it, and [`TOKEN-POLICY.md`](TOKEN-POLICY.md) says
exactly what a bump may and may not change. The compiler's version is
bookkeeping: it is published nowhere, and a refactor that touches no colour
does not move the number you depend on.

---

## Documentation

| Document | For |
|---|---|
| [**The palette browser**](https://noctua-world.github.io/noctua-colors/) | seeing every palette, live |
| [`docs/COMPILER.md`](docs/COMPILER.md) | **how this was made** — the two ideas at depth, the gamut-boundary derivation, and what the quality gates found but could not fix |
| [`TOKEN-POLICY.md`](TOKEN-POLICY.md) | what you may depend on, and what a version bump means |
| [`CHANGELOG.md`](CHANGELOG.md) | which colour changed, and whether your contrast still holds |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | working on this repository — **read *Before you clone* first**: this repository belongs to a workspace, and both cloning it elsewhere and starting an agent inside it break the tooling |
| [`SECURITY.md`](SECURITY.md) | supply-chain posture, and how to report an issue |
| [`AGENTS.md`](AGENTS.md) | the operating manual: invariants, commands, and the gotchas that have already cost debugging time here |
| [`specs/noctua.toml`](specs/noctua.toml) | the specification — heavily commented, and the only file a developer edits |
| [`examples/`](examples/) | a Rust consumer and a web consumer, both built by CI |

---

## Licence

MIT OR Apache-2.0, at your option. See [`LICENSE-MIT`](LICENSE-MIT) and
[`LICENSE-APACHE`](LICENSE-APACHE).
