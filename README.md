# noctua-colors

A **color system compiler**. A small declarative spec goes in; every color
artifact other projects consume comes out — CSS custom properties, a Tailwind
v4 theme, Rust constants, DTCG tokens, JSON/TypeScript, SCSS, and a QML
singleton.

It is not a palette, and not a UI library. **The repository versions the
curves, not the colors.** No hand-picked hex value exists anywhere in the
source; every color is computed, checked, and regenerated on demand.

---

## Working on this repository

This repository is part of the **Noctua workspace**, and two rules make the rest
of the tooling work:

- **Clone it only inside `noctua-workspace/repos/`.** Nothing outside that
  directory can reach the shared documents or the sibling repositories.
- **Start your agents only from the `noctua-workspace` root**, never from inside
  this repository. An agent started here cannot see `noctua-design`, cannot read
  the master technical reference, and will guess instead of looking. It detects
  this itself and prints a loud warning — if you see one, close it and reopen it
  at the workspace root.

The workspace's `NOCTUA.md` is the master technical reference for the whole
project; this repository's [`AGENTS.md`](AGENTS.md) is its own operating manual.

---

## Why

Two sibling projects, `noctua-hub` and `noctua-shell`, each keep a `Theme.qml`.
One of them describes itself in a comment as a fork of the other with "the same
palette". Five of their thirteen shared tokens have since drifted apart, and
neither file has a `success` or `warning` color — so two different greens were
invented independently at nineteen call sites between them.

That is what happens to a palette maintained as a list of hex values. This
project makes the list an *output*.

## The two ideas

**Relative chroma.** Chroma is not stored as an absolute number. It is stored
as a fraction `cr ∈ [0,1]` of the most a target gamut can actually show at that
lightness and hue. One definition, three renderings:

```text
l = 0.62, h = 264, cr = 0.9
  sRGB        max chroma 0.2043  →  C = 0.1839
  Display P3  max chroma 0.2200  →  C = 0.1980
  Rec.2020    max chroma 0.2228  →  C = 0.2005
```

The same token is more saturated on a wider display without being redefined,
families with very different natural chroma stay balanced at the same step, and
the whole "sober ↔ vivid" axis collapses to a single multiplier.

**Contrast-anchored steps.** A step's lightness is *solved* from a contrast
target, not authored as a ramp — and the contrast metric is APCA, not WCAG 2.x.
Here is why that matters, from this repository's own test suite:

| pair | WCAG 2.x | APCA |
|---|---|---|
| `#767676` on `#ffffff` | 4.54:1 — barely AA | Lc **+71.6** — comfortable |
| `#9a9a9a` on `#000000` | 7.46:1 — passes AAA | Lc **−47.7** — too weak for body text |

WCAG rates the dark-mode pair as substantially *better*. APCA, which models
polarity and the actual perceptual response, rates it markedly worse — which
matches what your eyes report. A system tuned to WCAG ships both as equivalent.
WCAG ratios are still emitted, as a compliance report, and no gate ever targets
them.

## Palettes come in a grid

Two axes: an accent hue and a saturation. A palette is one point on each, so
thirteen accents and three saturations describe **thirty-nine palettes in
eighteen lines** of specification:

```toml
[accents]
ochre = { hue = { base = 59.3, torsion = -7.0 } }
blue  = { hue = { base = 250.0, torsion = -8.0 }, hue_correction = [[0.55, 6.5]] }
# … eleven more

[saturations]
balanced = 0.82
vivid    = 1.15
sober    = 0.55
```

These expand into ordinary themes (`ochre-balanced`, `blue-vivid`, …) before
anything downstream sees them, so the engine, the eight emitters and the five
gates never learned about axes. Every generated palette is exactly what a
hand-written `[themes.*]` block would have been.

**The hues are not evenly spaced, and the gaps are measured.** Every accent sits
at least 14° from every semantic family, so a chosen accent is never mistakable
for a status colour. And one band is excluded outright: sweeping the arc in 2.5°
steps, an accent between roughly **158° and 172°** collapses onto `danger` under
protanopia — worst at 165°, where the two sit **0.0018** apart, far inside a
just-noticeable difference. Since 130–158° is already too close to `success`,
every green above lime is unusable. That is why the set has one green and three
blue-greens rather than a tidy dial.

## Every context an application needs

**Three hundred and fifty-two semantic contexts**, each emitted as five tokens —
the fill, its hover, a tinted background, a border, and a foreground to put on
the fill. Grouped in `specs/noctua.toml` by subject, because that is how a name
gets looked up:

```
severity      critical fatal severe failed alert major caution minor
              notice hint tip subtle silent debug disabled default
lifecycle     pending queued scheduled running processing stopping
              retrying stalled blocked complete done canceled aborted
work items    open reopened merged solved approved submitted escalated
              duplicate wontfix dismissed withdrawn revoked
connectivity  online offline connected reconnecting unreachable degraded
              outage maintenance healthy unstable stable operational
release       alpha beta preview canary nightly deprecated legacy retired
pipeline      deploying building testing passing installed synced verified
commerce      featured premium trial expiring overdue paid refunded shipped
              delivered available reserved backordered restocked
security      secure encrypted signed trusted compromised breached patched
… and inbox, content, configuration, capacity, magnitude, planning, people
```

**Ten of these have a hue; the other three hundred and forty-two are aliases.**
A family costs a full twelve-role ramp in every theme, mode and gamut; an alias
costs five `var()` references. The wheel has room for ten hues and not for three
hundred and fifty, so a context earns a family only when its meaning has to be
told apart from every other *without a legend* — `waiting` beside `active` in a
job list, `urgent` beside `danger` in an alert feed.

The alias layer is the same in every palette, so it is emitted **once**, into
`dist/css/contexts.css` and the top of `dist/json/palette.json`, with only what
a `[themes.<name>.semantic]` block overrides written per theme. Repeated across
thirty-nine palettes it was 97 KB of every 225 KB stylesheet.

There is also no room for more. Sixteen hues were already occupied by five
semantic families and twelve accents, and the 158–172° protanopia band is
unusable, so exactly three gaps could sit 14° clear of both neighbours. Two of
the four new families sit closer to an *accent* than that, deliberately: an
accent is chosen by whoever installs the palette and a status colour is not, so
the accent gives way.

```css
.rejected { background: var(--nc-color-rejected-bg);
            border: 1px solid var(--nc-color-rejected-border); }
.badge    { background: var(--nc-color-urgent); color: var(--nc-color-on-urgent); }
```

**Two ordered scales**, independent of each other and both traffic lights:
`level-0` … `level-10` and `lower` / `low` / `medium` / `high` / `higher`. Stops
are placed by perceptual distance along the hue path rather than evenly in the
parameter, and lightness descends by equal steps of stop *index* — because
lightness is the only axis left when hue is unavailable, and only the index
guarantees the step is even. Direction is measured, not chosen: an ascending
ramp put the worst neighbouring pair 0.0185 apart under protanopia, inside a
just-noticeable difference, because protanopia darkens reds and cancels each
step at the red end. Descending, the same darkening adds: **0.0447**.

**Three neutral temperatures.** `--nc-gray-*` leans toward the brand;
`--nc-gray-cool-*` and `--nc-gray-warm-*` are fixed temperatures beside it. All
three share the same step lightnesses, so `gray-7` can be swapped for
`gray-cool-7` without moving any contrast.

**A translucency ladder**, `--nc-neutral-a1` … `a12` and the same for the
accent, for overlays, scrims and hover washes. These are *real* alpha —
`color-mix(in oklab, <token> N%, transparent)`, which is premultiplied and so
composites correctly over any backdrop — not a hex solved against one fixed
background. The honest limitation travels with them: contrast is a property of
two opaque colours, so no gate can audit a translucent token.

## Status

**All seven milestones are complete.** One command compiles the spec into a
committed `dist/`; one command gates it.

What exists today is tested and load-bearing:

| | |
|---|---|
| Oklab / OKLCH / XYZ / linear + encoded RGB | all matrices derived at compile time from published chromaticities |
| Gamut boundary | exact, solved analytically (see below) |
| Gamut mapping | CSS Color 4 — chroma reduction, never per-channel clipping |
| APCA (Lc) | validated against the published anchors: black on white 106.04, white on black −107.88 |
| WCAG 2.x | reporting only |
| ΔE OK | Euclidean in Oklab |
| CVD simulation | Brettel 1997, protan / deutan / tritan, with severity |
| Spec format | TOML, every field defaulted, `miette` diagnostics that report every problem at once |
| Engine | monotone cubic Hermite curves, contrast-anchored solving, density-weighted neutral ramp |
| Emitters | CSS, Tailwind v4, Rust, DTCG, JSON/TS, SCSS, QML, plus compliance reports |
| Palettes | **39**, generated as a grid: 13 accent hues x 3 saturations, from 18 lines of specification |
| Contexts | **352** semantic slots over 10 families, plus two ordered scales, three neutral temperatures and a translucency ladder |
| Quality gates | contrast matrix across families, colour-vision margins, perceptual spacing, source invariants, and consumer token references |
| Documentation site | static, generated from `dist/`, dogfooding its own tokens |
| Playground | the compiler itself, compiled to WebAssembly and running in the browser |
| Palette importer | fits an existing palette back to spec parameters and publishes the residual |

Property tests at 2048 cases each, a golden snapshot of the whole palette, a
`dist/` sync check that fails if a generated file was edited by hand, and **48,441
palette checks** across every gate (`cargo xtask check` prints the current count) — the contrast pairs and the colour-vision
margins are generated from the semantic contract, so adding a context extends the
matrix rather than leaving a hole nobody notices.

<details>
<summary><b>What the quality gates found, and what they could not fix</b></summary>

The gates were wired up after the emitters and immediately reported **175
failures** against a palette that had passed everything else. The cause was
structural: every semantic solid was anchored to the same contrast target, so
they all landed at the same lightness and differed only in hue — and hue is
precisely the axis dichromacy removes. Success and danger sat **0.038 apart**
under deuteranopia against 0.234 for normal vision.

Families now separate in lightness as well as hue, which is the only lever that
survives. That is not a complete fix and the system says so rather than
pretending. Searched across every combination subject to fills staying visible
and ramps staying sane, the best achievable worst-case separation for a
six-family semantic set is **0.0163** — under one just-noticeable difference, and
there are now ten families.

So the gate reports margins and warns; it fails only when two colors are
literally the same. `dist/reports/colour-vision.md` publishes every number.
This is the reason WCAG 1.4.1 exists: never convey information by colour alone.
The palette gets you as far as colour can, and tells you exactly how far that
is.

The same measurement decided the categorical scale. Eight generated colors
bottom out at 0.0416 separation; six reach 0.0724. The default is six, and
asking for more warns rather than silently shipping a chart a dichromat cannot
read.

</details>

<details>
<summary><b>One thing worth knowing about the gamut boundary</b></summary>

The obvious way to find the most chroma a gamut allows at a given lightness and
hue is to bisect on chroma. It is wrong.

The sRGB gamut is not convex in Oklab. Along the ray at lightness 0.4525 and
hue 264.1, the red channel crosses zero **three times**: the ray leaves the
gamut around chroma 0.270, re-enters at 0.311, and leaves for good at 0.313. A
bisection converges to whichever crossing its midpoints happen to bracket, so
maximum chroma jumped 19% between hue 264.0 and 264.1 — a visible kink in any
ramp built on relative chroma, right next to the blue primary.

So the boundary is solved instead. At fixed lightness and hue each cone response
is affine in chroma, which makes each linear RGB channel an exact **cubic** in
chroma; the boundary is the smallest positive root among six of them. That is
exact, continuous, faster than the bisection it replaced, and — because it finds
the *first* crossing — it guarantees that every `cr` below 1 is in gamut, which
is precisely what relative chroma needs.

</details>

## Quick start

Requires only a Rust toolchain. There is nothing to install first — the
`cargo xtask` alias ships in `.cargo/config.toml`.

```bash
git clone <this repository> && cd noctua-colors
cargo xtask check
```

That validates the spec, runs every quality gate, verifies `dist/` matches, and
runs formatting, lints and tests. It is the same command CI runs, so "passes
locally, fails in CI" is not a thing that happens here.

Change a hue in `specs/noctua.toml`, then:

```bash
cargo xtask build     # every target under dist/ is regenerated
cargo xtask export    # and copied into every registered consumer
```

<details>
<summary><b>The whole command surface</b></summary>

Six verbs. `just <verb>` is a one-word alias for each; neither is required.

| Command | Does |
|---|---|
| `cargo xtask build` | Compile the spec into every target under `dist/`, and render the site |
| `cargo xtask check` | Validate the spec, run every gate, verify `dist/`, then fmt, lints and tests |
| `cargo xtask dev` | Watch the spec and the site, rebuild, serve with live reload |
| `cargo xtask export` | Copy `dist/` into every consumer registered in the spec |
| `cargo xtask import <source>` | Fit an existing palette back to spec parameters |
| `cargo xtask release <version>` | Prepare a version; committing is left to a human |

`cargo xtask check --colors-only` skips fmt, lints and tests for a fast loop
while tuning colors. `--dry-run` on `export` and `release` reports what would
happen without doing it.

```bash
cargo doc --open -p noctua-core              # the color math, documented
```

Property tests run 2048 cases each. To reproduce a specific failure, proptest
writes the input to `crates/noctua-core/tests/*.proptest-regressions`; commit
that file so the case is replayed forever.

</details>

<details>
<summary><b>Using <code>noctua-core</code> directly</b></summary>

```rust
use noctua_core::{Gamut, Oklch, map_into_gamut, to_hex};

// Ask for 90% of the most chroma sRGB can show at this lightness and hue.
let hue = 264.0;
let lightness = 0.62;
let max = Gamut::Srgb.max_chroma(lightness, hue);

let color = Oklch { l: lightness, c: max * 0.9, h: hue };
let mapped = map_into_gamut(color, Gamut::Srgb);

println!("{}", to_hex(mapped.rgb));
```

`map_into_gamut` holds lightness and hue fixed and reduces chroma until the
color fits, clipping only at the very end to absorb floating-point error. It
returns how much chroma it had to give up, which is what a "distance to the
gamut boundary" readout is built from.

</details>

<details>
<summary><b>Checking a pair for contrast and colour-vision safety</b></summary>

```rust
use noctua_core::cvd::worst_separation;
use noctua_core::map::from_hex;
use noctua_core::{Oklch, apca, wcag21};

let fg = from_hex("#767676").expect("valid hex");
let bg = from_hex("#ffffff").expect("valid hex");

// Sign carries polarity: positive is dark-on-light, negative light-on-dark.
let lc = apca(fg, bg);
let ratio = wcag21(fg, bg);   // reporting only

// The weakest link across protanopia, deuteranopia and tritanopia.
let a = Oklch { l: 0.55, c: 0.15, h: 25.0 }.to_oklab();
let b = Oklch { l: 0.55, c: 0.15, h: 145.0 }.to_oklab();
let (deficiency, margin) = worst_separation(a, b);
```

`worst_separation` returns a **margin**, not a verdict. Those two equiluminant
colors sit ΔE 0.260 apart to normal vision and **0.009** apart under
deuteranopia — far below the 0.02 just-noticeable difference. Give them a
lightness difference instead of relying on hue and the distinction survives
every deficiency. That is the rule the semantic pairs will be built on.

</details>

## The documentation site

```bash
cargo xtask dev          # or: just dev
```

Then open **<http://127.0.0.1:8080>**. The command prints the address it bound
to, watches `specs/noctua.toml` and everything under `docs-site/`, and reloads
the page whenever either changes.

8080 is a popular port. If something already has it, `dev` says so and exits
rather than leaving a watcher running with nothing to browse:

```bash
cargo xtask dev --port 8137        # or: just dev 8137
```

Nothing needs installing first. The WebAssembly target the playground needs is
declared in `rust-toolchain.toml`, and `xtask` installs it itself if it is
missing — so this works on a fresh clone with nothing but rustup.

| Where | What |
|---|---|
| `/` | The reference: the model explained visually, the full palette browser with click-to-copy in five formats, every context painted from its own token, both ordered scales, the translucency ladder over two backdrops, a contrast matrix **measured live in your browser**, realistic UI previews, and per-target integration snippets |
| `/playground.html` | The compiler itself, compiled to WebAssembly. Edit a spec and every ramp, every gate and every generated file is recomputed by the same Rust that runs on the command line. The URL carries the spec, so a link reproduces exactly what you are looking at |
| `/index.pt.html`, `/playground.pt.html` | The same pages in Brazilian Portuguese |

Four settings persist across visits: the **accent** (thirteen hues), the
**saturation** (`balanced`, `vivid`, `sober`), the **appearance** (light, dark,
or follow the operating system) and the **language**. Appearance defaults to
following the system, and is a real third choice rather than the absence of one
— a two-position switch has no way back to it.

The page renders **one** palette and builds the others in the browser from
`dist/json/themes/<name>.json`, fetching a palette's stylesheet the first time
it is chosen. Rendering all thirty-nine would be a 2.4 MB page with seventeen
thousand nodes; as it is, `index.html` is 96 KB and first paint costs 57 KB of
CSS, whatever the grid grows to.

Each language is a complete page built by the same generator, not a dictionary
applied on load: no flash of the wrong language, and both work with script
disabled. A URL that names its language always wins over a stored preference,
so a link to `/index.pt.html` opens in Portuguese for everyone.

The site is a **consumer**, not a second opinion: it reads `dist/json/palette.json`
and paints itself with `dist/css/`, so it cannot show you a color the compiler
did not produce. A gate greps its sources for color literals and fails the
build on any hit.

To build it without serving:

```bash
cargo xtask build        # writes docs-site/public/
```

`docs-site/public/` is gitignored and entirely static — no server-side anything,
so it deploys to any static host by copying. The `/playground.html` route needs
`.wasm` served as `application/wasm`; most hosts already do.

## Consuming the output

Everything under `dist/` is generated and **committed**, so every consumption
path works with no build step on your side: git submodule, subtree, sparse
checkout, a plain file copy, or a raw URL.

<details>
<summary><b>Plain CSS</b></summary>

```html
<link rel="stylesheet" href="dist/css/ramp.css">      <!-- the dense grays -->
<link rel="stylesheet" href="dist/css/contexts.css">  <!-- the contract -->
<link rel="stylesheet" href="dist/css/ochre-balanced.css">  <!-- the values -->
```

That is the whole integration. Light and dark already work three ways — the
system preference, a `data-theme` attribute, and a `.light` / `.dark` class —
on the root or on any subtree, with nothing to configure:

```html
<html data-theme="dark">          <!-- force dark -->
<div class="light"> ... </div>    <!-- a light island inside it -->
```

Use the semantic contract rather than raw steps wherever you can:

```css
.card {
  background: var(--nc-color-surface-raised);
  color: var(--nc-color-fg);
  border: 1px solid var(--nc-color-border);
}
.card:focus-visible { outline: 2px solid var(--nc-color-ring); }
```

The third file is the default theme, named after it: every `--nc-<family>-*`
step. The first two are shared by every palette and change only when the spec
does — `ramp.css` holds the dense neutral ramps (`--nc-gray-1` … `--nc-gray-24`,
and the `-cool` and `-warm` variants) and `contexts.css` holds the semantic
contract, which is an indirection onto whichever theme is active and is
therefore identical in all thirty-nine.

`index.css` imports all three plus every alternative theme, and is the one name
that does not move if a theme is renamed:

```html
<link rel="stylesheet" href="dist/css/index.css">
<html data-palette="blue-vivid">
```

Reach for `--nc-gray-4` or `--nc-color-success` having linked only
`ochre-balanced.css` and nothing will complain: CSS drops an undefined custom
property silently and the element keeps its inherited color. Link all three, or
link `index.css`.

**Every name this project emits is prefixed**, the semantic layer included —
`--nc-color-surface`, not `--color-surface`. The bare `--color-*` namespace is
fixed by Tailwind v4 and belongs to it alone, so only `tailwind/theme.css` emits
those names; the plain CSS never claims them from a consumer who has never
installed Tailwind.

</details>

<details>
<summary><b>Tailwind CSS v4</b></summary>

```css
@import "tailwindcss";
@import "../noctua-colors/dist/tailwind/theme.css";
```

Then `bg-surface`, `text-fg-muted`, `border-border`, `ring-ring`,
`bg-accent hover:bg-accent-hover`, `bg-rejected-bg text-on-rejected`,
`text-chart-3`, `bg-level-7`, `bg-magnitude-high`, and every palette step as
`bg-accent-solid`, `bg-gray-18` or `bg-neutral-a3`. The `dark:` variant is wired
to the same signals the tokens use, and each `--color-*` here points at the
prefixed layer, so the contract is defined in exactly one place.

</details>

<details>
<summary><b>Rust, including Dioxus</b></summary>

```toml
[dependencies]
# One feature per palette; the default is the one the CSS binds to `:root`.
# Every consumer loads the metadata for whatever is compiled in, so a program
# shipping two palettes should not carry thirty-seven it never names.
noctua-colors-tokens = { path = "../noctua-colors/dist/rust",
                         default-features = false,
                         features = ["ochre_balanced", "blue_vivid"] }
```

```rust
use noctua_colors_tokens::balanced::dark::accent;

let button = accent::SOLID.hex;          // "#c78756"
let packed = accent::SOLID.packed();     // 0x00c78756
let (l, c, h) = (accent::SOLID.l, accent::SOLID.c, accent::SOLID.h);
```

Everything is `const`, the crate has no dependencies, and it is `no_std`.

</details>

<details>
<summary><b>Style Dictionary and other DTCG tools</b></summary>

`dist/tokens/<theme>-<mode>.json` is standards-compliant DTCG with plain hex
`$value`s, so an existing pipeline reads it unchanged:

```js
// config.js
export default {
  source: ["../noctua-colors/dist/tokens/noctua-light.json"],
  platforms: { /* your platforms */ },
};
```

The OKLCH coordinates and relative chroma ride along in `$extensions` for
anything that wants them, and are ignored by anything that does not.

</details>

<details>
<summary><b>JavaScript and TypeScript</b></summary>

```ts
import { palette } from "../noctua-colors/dist/ts/index.js";

const solid = palette.themes.balanced.light.families.accent.steps[8];
solid.renditions[0].hex;             // "#bf8253"
solid.renditions[0].css;             // "oklch(0.6584 0.0985 57.71)"
solid.renditions[0].chromaHeadroom;  // how much room is left in the gamut
```

`index.d.ts` narrows theme, mode, family, role and semantic names to string
unions, so a typo is a compile error rather than an `undefined` at runtime.

`dist/json/palette.json` is the same data as plain JSON — every theme, mode,
gamut and step, with relative chroma and gamut headroom on every color.

</details>

<details>
<summary><b>QML and Quickshell</b></summary>

Copy or symlink `dist/qml/` next to your QML, then:

```qml
import "."

Rectangle {
    color: NoctuaDark.surface
    border.color: NoctuaDark.border
    Text { color: NoctuaDark.fg }
    Rectangle { color: NoctuaDark.accent }
}
```

One singleton per theme and mode — `NoctuaDark`, `VividLight`, and so on —
registered in `qmldir`. Values are hex, because Qt's `color` type does not
parse `oklch()`. Note that Qt's eight-digit form is **ARGB**, not RGBA.

</details>

<details>
<summary><b>SCSS</b></summary>

```scss
@use "../noctua-colors/dist/scss/noctua" as noctua;

.card { background: noctua.$nc-balanced-light-neutral-bg-app; }
```

Or via the flat map:

```scss
@use "sass:map";
.button { background: map.get(noctua.$noctua-colors, "balanced-light-accent-solid"); }
```

Sass resolves these at build time, so SCSS output cannot follow a runtime mode
switch — both modes are emitted under distinct names. If the page needs to
switch live, use the CSS custom properties instead.

</details>

## Does the model actually hold?

The importer exists to answer that, and to publish the answer either way. It
fits an existing palette to hue and relative-chroma curves and reports the
worst residual per family, in Oklab units. A just-noticeable difference is
about 0.02.

Run against **Tailwind v4** — 26 families of 10 steps, authored in OKLCH by
people with no knowledge of this model:

| Measured against | Families expressed | Median worst | Largest residual |
|---|---|---|---|
| sRGB | 23 / 26 | 0.013 | 0.041 (amber) |
| Display P3 | **24 / 26** | 0.010 | 0.030 (amber) |

```bash
cargo xtask import path/to/tailwindcss/theme.css --gamut display-p3
```

Two findings came out of this, and both changed the code.

**The failures cluster in the orange-to-green arc, and the reason is the
gamut.** Amber's relative chroma reaches **1.13** — the ramp sits 13% beyond
what sRGB can display at those lightnesses. Relative chroma is a fraction of
the achievable maximum and cannot exceed 1 by construction, so measuring that
palette against sRGB measures the gamut rather than the model. Against P3 the
worst residual halves. This is the clearest demonstration of relative chroma
there is: the same authored token is a different color in a wider gamut, and
that difference is the point.

**A straight hue line was not enough.** Amber's hue sweeps 95° to 46° as an
S-curve, most of the movement in the middle third; a line through the endpoints
misses the middle by about 15°. The fitter now solves three hue knots and the
position of the chroma peak, and emits the readable `{ base, torsion }` form
when the path really is straight and explicit knots when it bends. That took
8 failures down to 2.

Run against **20 hand-authored design systems** the result is different and
more interesting: of 1,120 token groups, only **three** are sequential ramps of
five steps or more. The rest are flat semantic sets — three hand-picked values
per family, no curve anywhere. Which is the argument for this project stated as
a measurement rather than an opinion.

Categorical scales are correctly rejected: 0 of 150 chart palettes fit a single
hue curve, because a categorical scale is deliberately not one.

The importer refuses to overclaim. A ramp with fewer colors than the model has
parameters is reported as **weakly constrained**, because a near-zero residual
on three colors is arithmetic rather than evidence.

## Roadmap

| # | Milestone | State |
|---|---|---|
| 1 | **Foundations** — color math, property tests | **done** |
| 2 | **Spec and engine** — curves, contrast-anchored solving | **done** |
| 3 | **Emitters** — CSS, Tailwind v4, Rust, DTCG, JSON/TS, SCSS, QML | **done** |
| 4 | **Automation** — `cargo xtask`, CI, quality gates | **done** |
| 5 | **Docs site** — static, mobile-first, dogfooding its own tokens | **done** |
| 6 | **Playground and palette import** — WebAssembly, reverse-fitting | **done** |
| 7 | **Polish** — errors, documentation, example consumer projects | **done** |

The developer-facing surface is six verbs and does not grow past that: new
capability goes behind an existing verb or it does not ship.

## Documentation

- [`AGENTS.md`](AGENTS.md) — the operating manual: invariants, gotchas,
  conventions. Read it before changing anything.
- `cargo doc -p noctua-core --open` — the color math, with the reasoning.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT license](LICENSE-MIT), at your option. Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in this
project shall be dual licensed as above, without any additional terms or
conditions.
