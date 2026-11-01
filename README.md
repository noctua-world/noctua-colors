<div align="center">

# noctua-colors

**A colour system whose colours were solved, not picked.**

Palettes on an accent × saturation grid · semantic names for hundreds of product states ·
light and dark from one file · every contrast pair checked on every build

[Browse the palettes](https://noctua-world.github.io/noctua-colors/) ·
[Install](#install) ·
[How it was made](docs/COMPILER.md)

</div>

---

Most colour systems are a list of hex values somebody chose. This one is the **output of a
compiler**: you describe a hue, and a program solves every step of every ramp against a
perceptual contrast target, in every palette, then checks every resulting pair before anything
ships.

You do not need to care about any of that to use it. You link one file:

```html
<link rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/palette/blue-vivid.css">
```

That is a complete install. `--nc-color-surface`, `--nc-color-fg`, `--nc-color-accent`,
`--nc-color-danger` and every other semantic name are defined, and dark mode already works.

---

## Contents

- [Install](#install)
- [Use it](#use-it)
- [Choose a palette](#choose-a-palette)
- [The names you get](#the-names-you-get)
- [Light and dark](#light-and-dark)
- [Mistakes that fail silently](#mistakes-that-fail-silently)
- [Why these colours are trustworthy](#why-these-colours-are-trustworthy)
- [Every way to install](#every-way-to-install)
- [Two versions, and which one is yours](#two-versions-and-which-one-is-yours)
- [Documentation](#documentation)

---

## Install

Pick the one that matches you. Each is complete: there is no step two.

### A web page

Copy this into your `<head>`, and swap `blue-vivid` for any palette in
[the grid](#choose-a-palette):

```html
<link rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/palette/blue-vivid.css">
```

That one file is a whole palette — the neutral ramp, every semantic name, light and dark — at a
few dozen kilobytes gzipped. In production, put a version in the URL
(`@noctua-world/colors@<version>/system/…`), because colour values may move between minor
versions ([`TOKEN-POLICY.md`](TOKEN-POLICY.md)).

### npm

```sh
npm install @noctua-world/colors
```

Then, in your CSS:

```css
@import "@noctua-world/colors/palette/blue-vivid.css";
```

### Tailwind v4

```sh
npm install @noctua-world/colors
```

Two lines in your entry CSS are the whole integration:

```css
@import "tailwindcss";
@import "@noctua-world/colors/tailwind/palette/blue-vivid.css";
```

You now write `bg-surface`, `text-fg`, `border-border`, `bg-accent`, `text-danger` and
`bg-gray-18`, and they follow light and dark on their own.

### Rust

```sh
cargo add noctua-colors-tokens --no-default-features --features blue_vivid
```

```rust
use noctua_colors_tokens::blue_vivid::light::accent;

let hex = accent::SOLID.hex;        // a &'static str, "#rrggbb"
let rgb = accent::SOLID.packed();   // a u32, 0x00rrggbb
```

`no_std`, no dependencies, every value a `const`.

---

## Use it

### Name meanings, not colours

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
`--nc-color-overdue` names a *meaning*, not a particular red.

### A complete page you can copy

Save this as `index.html`, open it, then switch your operating system between light and dark
appearance.

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

**`ochre-balanced` is the default**: the palette `index.css` binds to `:root`, and what you get
if you do not choose.

The saturation axis is a single multiplier on relative chroma, which is why it is an axis at all
rather than a set of separately tuned palettes.
[See them side by side.](https://noctua-world.github.io/noctua-colors/)

### Switching at runtime

To let your users choose, load the file that carries every palette, and set an attribute on the
root element:

```html
<link rel="stylesheet"
      href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/index.css">
```

```js
document.documentElement.dataset.palette = "violet-sober";
```

That file is many times the size of one palette's. If you ship a single palette, link its own
file instead.

---

## The names you get

Every name in this section exists in every palette, in both modes.

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

Each **subject** — a state or a meaning, such as `danger`, `overdue` or `deployed` — gives you
five names:

| Pattern | Example | For |
|---|---|---|
| `--nc-color-<subject>` | `--nc-color-danger` | a solid fill |
| `--nc-color-<subject>-hover` | `--nc-color-danger-hover` | its hover state |
| `--nc-color-<subject>-bg` | `--nc-color-danger-bg` | a tinted background |
| `--nc-color-<subject>-border` | `--nc-color-danger-border` | a border to match that background |
| `--nc-color-on-<subject>` | `--nc-color-on-danger` | **text that is legible on the solid fill** |

`--nc-color-on-danger` was solved against `--nc-color-danger` to a contrast target, so you never
guess whether white or black goes on top. The five come in **two pairings**, and mixing them is
the one easy mistake:

```css
/* A solid chip — the fill and the text solved against each other. */
.chip { background: var(--nc-color-danger); color: var(--nc-color-on-danger); }

/* A tinted panel — the background and its border. Ordinary page text on top. */
.panel { background: var(--nc-color-danger-bg);
         border: 1px solid var(--nc-color-danger-border); }
```

A sample of the subjects:

```text
accent  danger  warning  success  info  urgent  waiting  active  progress
pending  approved  rejected  archived  draft  published  deployed  failed
expired  overdue  locked  verified  deprecated  offline  syncing  throttled
quarantined  chargeback  backordered  onboarding  ratelimited  …
```

Ordinary product vocabulary: the point is that you rarely have to invent one. The complete list
is in [`system/css/contexts.css`](system/css/contexts.css) and on
[the documentation site](https://noctua-world.github.io/noctua-colors/).

### Underneath: the palette itself

Every semantic name points at one of these. Use them directly when no meaning fits.

| Names | What they are |
|---|---|
| `--nc-<family>-<role>` | each family's ramp, by role: `bg-app`, `bg-subtle`, `bg-element`, `bg-element-hover`, `bg-element-active`, `border-subtle`, `border-element`, `border-strong`, `solid`, `solid-hover`, `text-muted`, `text-strong` |
| `--nc-<family>-1` … `-12` | the same twelve steps, numbered |
| `--nc-gray-1` … `--nc-gray-24`, `--nc-gray-cool-*`, `--nc-gray-warm-*` | dense neutral ramps, **mode-independent**: `--nc-gray-4` is one colour, not two |
| `--nc-neutral-a1` … `-a12`, `--nc-accent-a1` … `-a12` | translucent washes, built with `color-mix` so they follow the active mode |
| `--nc-chart-1` …, `--nc-level-1` …, `--nc-magnitude-low` / `-high` | scales: categorical sets for charts, and ordered ones meant to be read in sequence |

The families are `accent`, `danger`, `warning`, `success`, `info`, `urgent`, `waiting`,
`active`, `progress`, and `neutral` with its `neutral-cool` and `neutral-warm` variants. The
`--nc-gray-*` ramps are shared by every palette.

---

## Light and dark

**It already works.** Three mechanisms are emitted, composed so they cannot conflict:

| On `<html>`, you set | It does |
|---|---|
| nothing | follows the operating system's preference |
| `data-theme="dark"` or `data-theme="light"` | forces that mode |
| `class="dark"` or `class="light"` | forces that mode |

So a toggle is one line:

```js
document.documentElement.dataset.theme = wantsDark ? "dark" : "light";
```

`color-scheme` is set too, so scrollbars, form controls and the canvas match. Without it, a dark
page still gets light scrollbars.

**Set the mode on the root, not on a section.** The same attribute or class on a smaller element
switches the palette's own steps (`--nc-<family>-<role>`) inside it, but **not** the semantic
names: CSS resolves a `var()` inside a custom property on the element that declares it, and the
semantic names are declared once, on the root. A `data-palette` on a section behaves the same way.
To give one section its own mode, declare again on it the semantic names it uses — their
definitions are in [`system/css/contexts.css`](system/css/contexts.css):

```css
.dark, .light, [data-theme] {
  --nc-color-surface: var(--nc-neutral-bg-app);
  --nc-color-fg:      var(--nc-neutral-text-strong);
}
```

---

## Mistakes that fail silently

Each of these leaves a page that looks plausible and a console that says nothing.

1. **Linking a theme file on its own.** `system/css/ochre-balanced.css` and
   `system/css/theme-<name>.css` carry one palette's values and nothing shared, so
   `--nc-gray-*` and `--nc-color-*` stay undefined — and CSS drops an undefined custom property
   without a word. Link the palette's own file, `palette/<name>.css`, or
   [all three layers](#plain-css-from-a-checkout-or-a-copy).
2. **`on-X` on `X-bg`.** `--nc-color-on-danger` is solved against the saturated fill, which puts
   it right beside the tinted `-bg` in lightness, in either mode: on that background it is nearly
   invisible, and since both are valid colours nothing warns you. The solid takes `on-X`; `-bg`
   takes `-border` and ordinary text.
3. **A mode or a palette set on a section.** The palette steps follow it; the semantic names do
   not ([Light and dark](#light-and-dark)).
4. **Linking from `raw.githubusercontent.com`.** It serves files as `text/plain` with `nosniff`,
   so browsers refuse to apply them as stylesheets. Use jsDelivr, through npm or through `gh`.
5. **An unpinned version in production.** Colour values may change in a minor release, and
   before 1.0 a minor release may rename; pin a version ([`TOKEN-POLICY.md`](TOKEN-POLICY.md)).
6. **A plain `@theme` in your own Tailwind mapping.** It bakes the values in at build time, so
   dark mode never applies. The package's mapping uses `@theme inline`; keep it that way if you
   write your own.

---

## Why these colours are trustworthy

Two ideas do the work. Both are covered at full depth in [**docs/COMPILER.md**](docs/COMPILER.md);
here is what they buy you.

**Chroma is relative, not absolute.** Saturation is stored as a fraction of the most a display can
actually show at that lightness and hue. So the same token is richer on a wide-gamut screen
without being redefined anywhere, and a yellow and a blue look equally saturated at the same step,
although blue has nearly twice the chroma available.

**Lightness is solved, not authored.** Every step's lightness is derived from a contrast target,
using **APCA** rather than WCAG 2.x. That distinction is not academic:

| pair | WCAG 2.x | APCA |
|---|---|---|
| `#767676` on `#ffffff` | 4.54:1 — barely AA | Lc **+71.6** — comfortable |
| `#9a9a9a` on `#000000` | 7.46:1 — passes AAA | Lc **−47.7** — too weak for body text |

WCAG rates the dark-mode pair as substantially *better*. APCA, which models polarity and the
actual perceptual response, rates it markedly worse — which matches what your eyes report. A
system tuned to WCAG ships both as equivalent. WCAG ratios are still emitted, as a compliance
report; no gate targets them.

**And it is all checked.** Every build checks every contrast pair in every palette and mode, and
fails on a regression. Colour-vision simulation runs too, and what it cannot fix is **published
rather than hidden**: `system/reports/colour-vision.md` gives the measured separation of every
pair of semantic colours under each dichromacy. No palette can make all of them distinguishable
by colour alone, so pair a colour that carries meaning with an icon, a label or a shape.

---

## Every way to install

| Route | Command or line |
|---|---|
| **CDN, one palette** | `<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/palette/blue-vivid.css">` |
| **CDN, every palette** | `<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@noctua-world/colors/system/css/index.css">` |
| **CDN, without npm** | `https://cdn.jsdelivr.net/gh/noctua-world/noctua-colors@v<version>/system/…` — the complete tree |
| **npm** | `npm install @noctua-world/colors` |
| **Tailwind v4** | `@import "@noctua-world/colors/tailwind/palette/blue-vivid.css";` |
| **Rust** | `cargo add noctua-colors-tokens --no-default-features --features blue_vivid` |
| **Cargo, from git** | `noctua-colors-tokens = { git = "https://github.com/noctua-world/noctua-colors", tag = "v<version>", default-features = false, features = ["blue_vivid"] }` |
| **SCSS** | `@use "@noctua-world/colors/scss/noctua" as noctua;` |
| **Design tokens (DTCG)** | `@noctua-world/colors/tokens/blue-vivid-light.json` |
| **TypeScript / JSON** | in the release tarball; npm leaves that payload out |
| **Qt / QML** | copy `system/qml/` from a [release](https://github.com/noctua-world/noctua-colors/releases) |
| **Nix** | `inputs.noctua-colors.url = "github:noctua-world/noctua-colors";` |
| **Submodule, subtree, plain copy** | `system/` is committed; take what you want |

### Plain CSS, from a checkout or a copy

`system/` is committed, so there is no build step:

```html
<link rel="stylesheet" href="system/css/palette/ochre-balanced.css">
```

Or the three layers separately, if you want to swap only the palette:

```html
<link rel="stylesheet" href="system/css/ramp.css">            <!-- the dense greys -->
<link rel="stylesheet" href="system/css/contexts.css">        <!-- the contract -->
<link rel="stylesheet" href="system/css/ochre-balanced.css">  <!-- the default palette's values -->
```

For any other palette the third file is `system/css/theme-<name>.css`, which applies under
`data-palette="<name>"` on the root. **Link all three, or none**: a theme file alone defines no
`--nc-gray-*` and no `--nc-color-*` (mistake 1 above).

### npm subpaths

| Subpath | Contents |
|---|---|
| `@noctua-world/colors` | every palette (`css/index.css`) |
| `@noctua-world/colors/palette/<name>.css` | one palette, self-contained |
| `@noctua-world/colors/css/<file>.css` | any single file of `system/css/`: `ramp.css`, `contexts.css`, a theme's values |
| `@noctua-world/colors/tailwind` | Tailwind, every palette |
| `@noctua-world/colors/tailwind/palette/<name>.css` | Tailwind, one palette |
| `@noctua-world/colors/scss/noctua` | the SCSS entry |
| `@noctua-world/colors/tokens/<name>-<mode>.json` | DTCG tokens |
| `@noctua-world/colors/axes.json` | the accent × saturation grid |
| `@noctua-world/colors/manifest.json` | hashes and both versions |

### Tailwind v4, every palette

```css
@import "tailwindcss";
@import "@noctua-world/colors/tailwind";
```

Utilities come out as `var()` references rather than frozen values, so they follow the active
mode. That is `@theme inline`, and it is load-bearing: a plain `@theme` would stamp `bg-accent`
with whichever mode was active at build time, and dark mode would silently stop working.

The `dark:` variant follows the `.dark` class and `data-theme="dark"`; it does not fire on the
operating system's preference alone. The colour utilities rarely need it — `bg-surface` is
already dark in dark mode.

### Rust

One Cargo feature per palette, so a program compiles in only the palettes it names.
`--no-default-features` leaves out the default palette, `ochre_balanced`.

```rust
use noctua_colors_tokens::blue_vivid::{dark, light};
use noctua_colors_tokens::gray;

const CARD: &str = light::neutral::BG_ELEMENT.hex;
let packed: u32 = dark::accent::SOLID.packed();
let solid = light::accent::SOLID;
let (l, c, h) = (solid.l, solid.c, solid.h);   // OKLCH, as the colour was defined
let step = gray::STEP_18;                       // the shared neutral ramp
```

A path is `<palette>::<light|dark>::<family>::<ROLE>`; `gray`, `gray_cool` and `gray_warm` are
shared by every palette and always present.

### Design tokens (DTCG 2025.10)

`system/tokens/<palette>-<mode>.json` is standards-compliant DTCG. Colour values are objects with
`colorSpace: "oklch"` and `components`, plus a six-digit `hex` fallback — which is what the
2025.10 stable specification requires, and what tools such as Style Dictionary v5 actually read.

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

Values are hex, because Sass resolves at compile time and cannot follow a `var()`. Both modes are
emitted; you pick one.

### TypeScript and JSON

`system/json/palette.json` is every theme, mode, family, step, role, gamut rendition and relative
chroma as plain JSON; `system/ts/index.js` is the same data, with `.d.ts` types. Both run to many
megabytes, so they are **deliberately left out of the npm package** and shipped in the
[release](https://github.com/noctua-world/noctua-colors/releases) tarball instead.
`npm pack --dry-run` shows what the package carries.

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
inputs.noctua-colors.url = "github:noctua-world/noctua-colors";
```

The flake provides the package as `packages.<system>.default`, and an overlay,
`overlays.default`, that adds `pkgs.noctua-colors`. Everything is installed under
`share/noctua-colors`:

```nix
installPhase = ''
  mkdir -p $out/static
  cp -r ${pkgs.noctua-colors}/share/noctua-colors/css $out/static/
'';
```

### Verifying what you installed

The npm package and the release assets are published from CI with no stored credentials, over
OIDC, and carry signed build provenance.

```sh
# npm
npm audit signatures

# release assets
gh attestation verify noctua-colors-v<version>-system.tar.gz \
  --repo noctua-world/noctua-colors
sha256sum -c SHA256SUMS

# any file, against the manifest
b3sum -l 16 system/css/index.css   # the css/index.css entry in system/MANIFEST.json
```

---

## Two versions, and which one is yours

This repository publishes two things, on two clocks:

| | What it is | Where you see it |
|---|---|---|
| **The colour system** | the palettes, the names, the values | npm, crates.io, the git tag |
| **The compiler** | the program that generated them | `MANIFEST.json`'s `version` |

**The colour system's version is the one that matters to you.** A tag names it, both registries
publish it, and [`TOKEN-POLICY.md`](TOKEN-POLICY.md) says exactly what a bump may and may not
change. The compiler's version is bookkeeping: it is published nowhere, and a refactor that
touches no colour does not move the number you depend on.

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
| [`AGENTS.md`](AGENTS.md) | the operating manual for anyone, human or agent, changing this repository |
| [`specs/noctua.toml`](specs/noctua.toml) | the specification — heavily commented, and the only file a developer edits |
| [`examples/`](examples/) | a Rust consumer and a web consumer |

---

## Licence

MIT OR Apache-2.0, at your option. See [`LICENSE-MIT`](LICENSE-MIT) and
[`LICENSE-APACHE`](LICENSE-APACHE).
