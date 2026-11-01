# noctua-colors — Agent Guidelines

> ## Everything you write to disk is in English.
>
> Code, comments, identifiers, documentation, CLI help, error messages, test names, commit
> messages, the documentation site — all of it, always, whatever language the developer writes
> to you in. You may *reply* in that language; you may not *produce artifacts* in it. The one
> exception is the site's interface strings, which exist in English and Brazilian Portuguese
> (`NOCTUA.md §2.1`).

This file is the operating manual for `noctua-colors`: what the repository is, its invariants, the
command surface, where each kind of change goes, the traps this code has already met, and how a
release is published. Every agent reads it before changing anything here. `NOCTUA.md §8` is the
same repository seen from outside — what a consumer needs; this file carries the internal detail,
and the two must never disagree.

## Before anything else: are you in the right place?

This repository lives inside **`noctua-workspace`**, and agents must be started at
the workspace root — not here. From inside this repository the other repositories
are invisible, so an agent cannot read the shared colour contract, edit a
design-system component, or consult the master technical reference.

Run this **once, at the start of the session, before any other work.** Do not
re-run it per task.

```sh
d=$PWD; found=
while :; do
  [ -f "$d/.noctua-workspace" ] && { found=$d; break; }
  [ "$d" = / ] && break
  d=$(dirname "$d")
done
if [ -z "$found" ]; then
  echo "NOCTUA_WS=outside"
elif [ "$found" = "$PWD" ]; then
  echo "NOCTUA_WS=root"
else
  case $PWD in
    "$found"/repos/*) echo "NOCTUA_WS=inside-repo:$found" ;;
    *)                echo "NOCTUA_WS=inside-workspace:$found" ;;
  esac
fi
```

- **`root`** → proceed silently. Print nothing.
- **`inside-repo:…`** → print **ALARM A** below verbatim, stop, and ask whether to
  continue or abort. Touch nothing until answered.
- **`outside`** → print **ALARM B** below verbatim, and stop the same way.
- **`inside-workspace:…`** → not an alarm. Print the one-line correction, then
  continue, resolving paths against the workspace root.

**ALARM A — wrong starting directory:**

```
🚨🚨🚨 STOP — WRONG DIRECTORY 🚨🚨🚨

⛔ THIS AGENT WAS NOT STARTED FROM THE NOCTUA WORKSPACE. ⛔

YOU ARE INSIDE A SINGLE REPOSITORY. FROM HERE I CANNOT SEE THE OTHER
NOCTUA REPOSITORIES, SO I WILL NOT BE ABLE TO READ THE SHARED COLOR
TOKENS, EDIT A DESIGN-SYSTEM COMPONENT, OR CHECK THE MASTER TECHNICAL
REFERENCE. I WILL GUESS, AND MY GUESSES WILL BE WRONG. 🙈

✅ WHAT TO DO: STOP WHAT YOU ARE DOING, CLOSE THIS AGENT, AND REOPEN IT AT:

    THE WORKSPACE ROOT — THE DIRECTORY HOLDING `.noctua-workspace`,
    WHICH THE PROBE ABOVE JUST FOUND AND NAMED

❓ DO YOU WANT ME TO CONTINUE ANYWAY, OR ABORT THE CURRENT WORK?
   I WILL NOT TOUCH ANYTHING UNTIL YOU ANSWER. 🛑
```

**ALARM B — repository outside the workspace:**

```
🚨🚨🚨 STOP — REPOSITORY IN THE WRONG PLACE 🚨🚨🚨

⛔ THIS REPOSITORY IS NOT INSIDE THE NOCTUA WORKSPACE. ⛔

I SEARCHED EVERY PARENT DIRECTORY AND FOUND NO `.noctua-workspace`
MARKER. THIS CLONE IS SOMEWHERE IT SHOULD NOT BE, SO NOTHING SHARED —
COLOR TOKENS, THE DESIGN SYSTEM, THE MASTER TECHNICAL REFERENCE — IS
REACHABLE FROM HERE. 🙈

✅ WHAT TO DO: STOP, AND RE-CLONE THIS REPOSITORY INSIDE:

    THE `repos/` DIRECTORY OF YOUR NOCTUA WORKSPACE — THE ONE
    DIRECTORY THAT HOLDS THE `.noctua-workspace` MARKER

   THEN REOPEN THE AGENT AT THE WORKSPACE ROOT, NOT INSIDE THE REPOSITORY.

❓ DO YOU WANT ME TO CONTINUE ANYWAY, OR ABORT THE CURRENT WORK?
   I WILL NOT TOUCH ANYTHING UNTIL YOU ANSWER. 🛑
```

**The correction (not an alarm):**

```
⚠️ Started in a workspace subdirectory, not at the workspace root.
   Working from <path> instead. Everything is reachable, so continuing.
```

These texts are authored once, in `NOCTUA.md §3.6`, and this copy must match them verbatim. If
one changes, change it there and propagate.

## Local notes

An **`AGENTS.local.md`** may exist beside this file: the developer's private, machine-specific
notes. **Look for one.** It is gitignored and never committed (`NOCTUA.md §3.4`).

## What this is

**Two products, and confusing them is the mistake this section exists to prevent.**

1. **A colour system** — palettes and semantic names, published to npm and crates.io and
   consumed by CDN, Nix, submodule and copy. *This is the product*: what almost everyone who
   arrives here wants, what a tag means, and what `TOKEN-POLICY.md` governs.
2. **The compiler that produces it** — a declarative spec in; CSS custom properties, a Tailwind
   v4 theme, Rust constants, DTCG tokens, JSON/TypeScript, SCSS and a QML singleton out. *This is
   the proof, not the pitch.* Nobody else runs it on their own spec: the crate name
   `noctua-colors-tokens` and this repository's URL are hardcoded in
   `crates/noctua-emit/src/rust.rs`.

**Anything user-facing leads with the colours.** A README or docs page that opens with
`git clone`, or with technical reference, has it backwards.

**The repository versions the curves, not the colours.** No hand-picked hex exists in the source.
Two ideas carry it:

- **Relative chroma.** Chroma is a fraction `cr ∈ [0,1]` of what the target gamut can show at
  that lightness and hue, so one definition renders correctly on sRGB and more vividly on
  Display P3, and "sober ↔ vivid" is one multiplier.
- **Contrast-anchored steps.** A step's lightness is *solved* from an APCA contrast target
  against a declared reference, not authored as a ramp.

**`specs/noctua.toml` is the only file a developer edits.** Everything under `system/` is
generated, committed, and never edited by hand.

**Who consumes it.** `noctua-design` vendors `system/` as a `[[consumers]]` entry and defines no
colour of its own; `noctua-central` reaches the colours only through the design system
(`NOCTUA.md §8.6`). A colour another repository needs is a line in this spec — almost always one
`[semantic]` alias — never a value written over there.

## Status

**Complete and published**: `github.com/noctua-world/noctua-colors`, npm `@noctua-world/colors`,
crates.io `noctua-colors-tokens` and `noctua-colors` (**Publishing**, below).

**It is finished** (`NOCTUA.md §8.9`): no redesign, no refactor, no new emitter, no new verb, no
colour-science change. A colour that seems to be missing is a one-line `[semantic]` alias,
discussed with the developer first. The recipes below say where a change goes once the developer
has asked for it.

**Counts rot; commands do not.** Read the current shape instead of trusting a number in prose:

```bash
jq '.themes | length' system/json/axes.json        # how many palettes
jq '.semantic | length' system/json/palette.json   # how many semantic names
cat system/MANIFEST.json                           # every generated file, its hash, both versions
cat system/reports/colour-vision.md                # every measured colour-vision margin
```

## Repository map

```
crates/noctua-core/    colour math: space, gamut and cubic (the analytic chroma
                       boundary), map (CSS Color 4, hex), contrast (APCA, plus WCAG
                       for reporting), diff, cvd (Brettel 1997), matrix
crates/noctua-spec/    the TOML model, defaults, the accent × saturation expansion,
                       validation, miette diagnostics
crates/noctua-engine/  curves, contrast-anchored solving, palette construction
crates/noctua-emit/    the output targets and compliance reports, plus writing and
                       verifying system/
crates/noctua-check/   the quality gates: contrast, colour vision, spacing, source
                       literals, consumer token references
crates/noctua-docs/    the documentation site's generator (maud), two locales. Reads
                       system/ only
crates/noctua-wasm/    browser bindings over the engine, for the site's playground
crates/noctua-colors/  the facade crate that holds the name on crates.io. No code
xtask/                 the six verbs, plus the wasm bundle build
specs/noctua.toml      the spec — the only file a developer edits
system/                the published colour system. COMMITTED. Never hand-edited;
                       only `build --system` and `release` write here
target/system/         where an ordinary `build` writes. Gitignored scratch
docs-site/             the site's sources. public/ is build output, gitignored
docs/COMPILER.md       how the system is made, for readers of the repository
examples/              consumer-rust and consumer-web, outside the workspace
tests/golden/          palette.txt, a snapshot of the built palette
.github/workflows/     ci, pages, and the three release workflows
.TEMP/                 the landing area for hand-vendored inputs. Contents gitignored
```

## Invariants

Breaking any of these is a defect, not a trade-off.

1. **No hardcoded colour values outside the spec** — not in the engine, tests, docs site or
   examples. Construct them: `Oklch { l: 0.55, c: 0.15, h: 25.0 }`, or a fraction of
   `max_chroma` at a chosen hue, which covers the space deliberately rather than arbitrarily.
   Three narrow carve-outs: **hex string fixtures** where the *format* is under test (strings,
   not colours); **black and white**, the only colours with no free parameters; and **published
   reference constants** cited in a comment. `noctua_check::source` enforces it: a literal needs
   an `// allow-literal: <reason>` marker on its line, which puts every exception in front of a
   reviewer. Golden files and `system/` are exempt; both are tool-generated.
2. **Never clip per channel to fit a gamut** — it shifts hue silently. Use
   `map::map_into_gamut`. The one exception is `cvd::simulate` (**Gotchas**).
3. **APCA is the design criterion. WCAG 2.x is reporting only.** No solver and no gate may
   target a WCAG ratio.
4. **Determinism.** The same spec and version produce byte-identical output: no timestamps, no
   random ordering, no `HashMap` iteration order reaching a generated file — `BTreeMap`, or
   `IndexMap` where insertion order is the contract. `check` compares the committed `system/`
   with a fresh build, so output that varies from run to run fails it.
5. **Generated files are never hand-edited.** Every one says so in its header, and `check` fails
   on one that was.
6. **Stay in OKLCH.** CAM16 is not implemented and no stub exists for it: it needs
   viewing-condition parameters this project does not have, and its advantage for UI work is
   marginal. The `UniformSpace` trait keeps the door open; leave it shut until a *measured*
   failure justifies otherwise.
7. **Out of scope, permanently:** spectral data, chromatic adaptation, viewing-condition models,
   tone mapping. No stubs for them either.
8. **A context is an alias until proven otherwise.** A new `[families.*]` costs a hue the wheel
   does not have and megabytes of output across the palettes; a `[semantic]` line costs five
   `var()` references written **once**, because the semantic layer is shared (**The semantic
   layer is emitted once**). Hundreds of contexts on a handful of families is the shipped ratio.

**The dependency rule.** `noctua-core` has no workspace dependencies and never will; it knows
nothing of the spec format or the output targets. `noctua-emit` performs **no colour math**: it
formats resolved, quantized colours, because an emitter that computes one makes the gates and the
output disagree about what shipped.

## Commands

```bash
cargo xtask build                      # compile the spec into target/system/ — scratch
cargo xtask build --system             # ...into system/, the published colour system
cargo xtask check                      # the single gate
cargo xtask dev                        # watch the spec, rebuild, serve with live reload
cargo xtask export                     # copy system/ into every registered consumer
cargo xtask import <file>              # fit an existing palette back to spec parameters
cargo xtask release <version>          # prepare a version of the colour system
cargo xtask release --tool <version>   # ...or of the compiler
```

Six verbs, and the surface **does not grow past that**: new capability goes behind an existing
verb or it does not ship. `just` offers one-word aliases and is never the only path.

**`check` is the gate, and CI runs exactly that command**, not a reimplementation in YAML
(`NOCTUA.md §19.4`). In order: the spec builds; every quality gate runs; `system/` matches the
spec; the generated crate compiles; `examples/consumer-rust` runs; `package.json` is coherent;
then `cargo fmt --check`, clippy with warnings denied, and the tests. **`--colors-only` skips
those last three**, for a fast loop while tuning colours on a developer's machine. CI must never
pass it.

**Two versions, on two clocks.** The colour system's is the spec's `[system]` table: what a tag
means, what both registries publish, what `TOKEN-POLICY.md` governs. The compiler's is
`Cargo.toml`'s `[workspace.package]`, published nowhere. `system/MANIFEST.json` carries both, and
its `version` key stays the compiler's while `systemVersion` is the colours', because consumers
diff that file: adding a key is safe, repurposing one silently changes what their diff means.
Never merge the two — one number cannot tell a refactor from a colour change, and only the second
is a reason to upgrade.

**`build` never writes `system/`.** `system/` is the published product — linked by CDN URL,
pinned as a Nix `src`, vendored as a submodule — so the everyday loop of edit, build, look writes
the gitignored `target/system/`, which `dev` serves. Publishing is the typed intent `--system`.
The guard is symmetric: `check` verifies **`system/`** against the spec, so meaning a change and
forgetting to publish it fails too.

**`export`** copies the targets each `[[consumers]]` entry asks for into its path. A path outside
this repository is expected, and is announced before anything is written; a file is written only
when its bytes differ, so an unchanged export leaves the consumer's file watcher alone.

**`import`** reads colour literals out of CSS, SCSS, QML, JSON or plain text, groups them into
ramps by name, and reports how closely the curve model expresses each. `--gamut` measures `cr`
against a gamut other than the spec's — a palette authored for P3 has colours sRGB cannot show,
and `cr` cannot exceed 1.

**`release`** is under **Publishing**.

## Recipes

Where each kind of change goes, once the developer has asked for it. Each is a small edit; one
that grows into a refactor is a reason to stop and ask.

**Add a context.** One line in `[semantic]` mapping it to an existing family, in the subject group
it belongs to; it reaches every target. Read **The hue wheel is full** before adding a family
instead.

**Add a categorical set.** One `[[charts]]` block with a `name`. Past about six entries no
generated set separates under all three dichromacies; `labelled = true` declares that the legend
names every series, and reports one measured note per deficiency instead of one per pair — it does
not lower the floor, and two entries inside a JND still fail. **Measure rather than interpolate**:
placement is not monotone in the lightness centres, and the band is clamped into 0.08–0.95, so a
spread that runs off the top piles entries together. The spec's comments on
`[[charts]] name = "chart-wide"` carry the numbers.

**Add a family.** One block with a `hue`; the rest defaults. Rebuild `system/` and the golden
file, then read the new colour-vision margins. Invariant 8 first.

**Add a scale.** One `[[scales]]` block. `stops` is a count or a list of names; reversing the hue
knots reverses direction, and `lightness_spread` may be negative (**A scale says which kind it
is**).

**Add an accent or a saturation.** One line in `[accents]` or `[saturations]`, plus a label in
`crates/noctua-docs/src/controls.rs`, or the picker shows the raw identifier. An accent in the
158–172° arc needs re-measuring first. A one-off palette off the grid is still
`[themes.<name>]`.

**Rename a theme.** Its file moves with it, so `README.md`, `examples/`, the integration snippet
in `crates/noctua-docs/src/sections.rs` and `noctua_docs::token_files` move too;
`system/css/index.css` is the one name that never does. A rename is a breaking change under
`TOKEN-POLICY.md`.

**Add an emitter.** One file in `crates/noctua-emit/src/` implementing `Emitter`, a line in
`registry()` and a `pub mod`. Never a refactor. Use `tokens::` for what to emit and `value::` for
how to spell a colour, so the new target cannot drift from the others.

**Retune contrast.** Edit the role targets in `crates/noctua-spec/src/defaults.rs`, or override
them with `[[scale.roles]]`. Light and dark are not mirror images.

**Add a quality gate.** A module in `crates/noctua-check/src/` returning a `Report` of
`Finding`s, plus a line in `run()`. Every finding has a location, a message saying what to do, and
a margin where one is meaningful — a verdict alone tells nobody how close they were. Test that its
`checked` count is non-zero.

**Add a page to the site.** A module in `crates/noctua-docs/src/` plus an `Output` per locale in
`render()`; the tests read `render()` itself.

**Change a `#[wasm_bindgen]` signature.** `cargo xtask build` checks that every name
`docs-site/js/playground.js` imports is exported; `noctua-wasm`'s own tests check the JSON
*shape*.

**Register a consumer.** Three lines under `[[consumers]]` in the spec — `name`, `path`,
`targets` — then `cargo xtask export`.

## Gotchas

Each entry is a trap this code has already met: the rule, and the reason or measurement that
keeps it from being undone. They are grouped by where they bite.

### Colour math

**The sRGB gamut is not convex in Oklab, so bisecting chroma gets it wrong.** At lightness 0.4525
and hue 264.1 the red channel crosses zero three times, and a bisection lands on whichever
crossing its midpoints bracket: maximum chroma jumps 19% between hue 264.0 and 264.1.
`max_chroma` solves six cubics and takes the smallest positive root — the **first** crossing,
which relative chroma requires. Never replace it with a search;
`the_srgb_gamut_is_non_convex_near_the_blue_primary` pins it, and
`crates/noctua-core/src/gamut.rs` carries the derivation.

**Display P3 is not a subset of Rec.2020.** Near P3's red primary (xy 0.680, 0.320) a sliver of
orange-red lies outside Rec.2020, whose red–green edge passes y = 0.31828 there; sRGB is inside
both. "Wider gamut" is not a total ordering.

**Two constants look wrong and are not.** `screen_luminance` uses a plain 2.4 power with no
linear toe, because APCA's constants were fitted against that curve; `wcag_luminance` uses the
real one. And the published Oklab matrices put D65 white at 0.999_998_8 — fatal when steps are
anchored on lightness — so `crates/noctua-core/src/space.rs` conditions them at compile time.
Never paste the raw matrices back in.

**Gamut mapping reports coordinates that are in gamut.** Keep lightness and hue, and clamp chroma
to `max_chroma`: the bisection's last candidate is out of gamut (re-mapping erodes chroma on every
pass), and the clipped colour's coordinates are off in hue by up to 15°. The property test asserts
*exact* equality; a tolerance would hide the bug.

**`cvd::simulate` clamps channels — the one place that is right.** The dichromatic surface leaves
the sRGB cube (a saturated blue for a protanope gives red ≈ −0.32), and mapping cannot rescue a
colour with no meaningful hue. Simulation is therefore only *near*-idempotent, and the tests
scope that law to projections that land in gamut.

**Colour vision deficiency can push two colours *further* apart** — up to 1.34× under
protanopia, measured over 10,648 colours — because the projection contracts distances in linear
RGB, but ΔE is measured in Oklab with a different matrix on each half-plane. Never assume
contraction.

**No gamut lookup table without measuring again.** A table answers in 44 ns against 329 ns
analytic, but takes 43 ms to build (break-even near 152,000 lookups), and its worst error, 0.014
chroma, lands at the blue primary.

**Relative chroma cannot exceed 1**, so a palette authored for a wider gamut cannot be expressed
in a narrower one: fitted against sRGB, Tailwind v4's amber, orange and yellow sit up to 13%
outside it, and against P3 the worst residual halves. Check the gamut before blaming the model:
`cargo xtask import <file> --gamut display-p3`.

### Contrast and colour vision

**Light and dark are not mirror images; never copy values across.** APCA's reverse polarity is
far less sensitive — a separation of 0.265 from the app background reads Lc 46 in light and
Lc 15 in dark — so dark needs roughly 1.45×. Dark *solid* targets are lower too, since Lc 58 on a
dark ground is lightness 0.75, a paler brand. Text targets stay near-equal.

**Two role targets carry a measured allowance.** The dark `solid` aims at **45.25**: it is solved
against its own family's `bg-app` but shipped against the page's neutral, and that tint moves the
measured contrast by up to 0.016 Lc. A quarter clears that; a full Lc walks `progress`'s dark
solid into its own `border-strong`. The light `text-muted` aims at **78** to narrow the ramp's
one structural gap, the 24 Lc jump to `text-strong` that light polarity compresses into a quarter
of the lightness range (0.2684 against a 0.26 ceiling).

**Solids deliberately leave the ramp.** A solid is picked to read as a brand or a state, and that
is what lets semantic families separate in lightness. Gates for monotonicity, adjacent-step
distance and cross-family agreement skip `Palette::shiftable_roles`: they look for accidental
drift, not intent.

**`on-{slot}` on `{slot}` is gated at 30, not 45, because it is arithmetic.** The engine solved
the solid to sit `45 + contrast_shift` Lc from the family's lightest step in dark mode, so the
families with a negative shift land between Lc 31 and 43. A status fill is not a text background;
`fg` on `{slot}-bg` is, gated at 90.

**Lc is printed at four decimals, everywhere** — the palette's quantization — so nothing
displayed rounds across a threshold it did not cross; at one decimal, a pair 0.0003 short prints
as meeting its target beside a failure mark. The site measures the *painted pixel*, so its last
decimal can differ from the compiler's, and the page says so.

**Semantic colours cannot be made dichromacy-safe by colour alone.** The best worst-case
separation any six-family set reaches is **0.0163**, under one JND, and the spec declares more
families than that. So the gate warns with margins and fails only on identical colours, one
finding per pair naming the worst palette; `labelled = true` collapses a categorical set to one
note per deficiency, and every pair is still measured. Never "fix" this by raising the shifts or
lowering the target: consumers carry a non-colour signal (`NOCTUA.md §8.8`).

**`OPPOSED` is curated; `margins` measures everything.** The gate lists only pairs whose
confusion changes what the interface *said* — "it worked" for "it broke", waiting for running —
while `margins` returns every pair and `system/reports/colour-vision.md` publishes them, so a
collision stays findable without burying a dangerous one. Extend `OPPOSED` for a meaning, never
for tidiness.

**`Severity::Note` is a measured limit, not a quiet warning.** A warning says a different choice
would fix it, so it is fixed; a note says no palette can. Only `cvd` emits notes, and only above
its floor — identical colours still fail. Never reclassify a warning to quiet it: a permanently
yellow build teaches people to skip yellow lines. The target is no failures and no warnings.

**The semantic layer and the pair table come from two shape tables** in
`crates/noctua-emit/src/tokens.rs`, driven by `mode.semantic`, where every family is a slot of its
own name. Never hand-write a row: a hand-written table fails by a missing row, which nothing
reports. `contrast::pairs` groups rows by family, so the aliases of `danger` report a failure
once.

**Three deliberate duplications, each compared by a test.** The site's sample of the pair table in
`crates/noctua-docs/src/sections.rs` (`noctua-docs` never depends on the gates):
`the_site_and_the_gate_agree_on_every_pair`. `contrast::semantic_view`, which re-derives the
contract instead of importing `tokens::semantic_tokens` so the gate cannot agree by construction:
`the_gate_and_the_emitter_resolve_every_token_alike`. The palette stylesheet's href in the
bootstrap in `crates/noctua-docs/src/page.rs` and in `docs-site/js/site.js`:
`the_bootstrap_and_the_script_build_the_same_stylesheet_path`. Independence nobody compares is
drift.

### Palettes, families and scales

**Palettes are a grid, expanded before anything else sees them.**
`crates/noctua-spec/src/expand.rs` turns `[accents]` × `[saturations]` into ordinary
`[themes.*]`, so nothing downstream knows about axes, and a generated theme is exactly a
hand-written one.

**The hue wheel is full.** Every gap 14° clear of both neighbours is taken, and 158–172° is
unusable. `waiting` (117.5°) and `active` (203°) sit 12.5° and 9° from the nearest *accent* on
purpose: whoever installs a palette chooses its accent and nobody chooses a status colour, so the
accent gives way; separation between *families* is what holds. **There is no room for another
family** (invariant 8).

**158–172° is unusable for an accent.** Swept in 2.5° steps, it collapses onto `danger` under
protanopia — 0.0018 apart at 165° — and 130–158° is already too close to `success`, so every
green above lime is out. Re-measure before adding a hue in that arc.

**The dense ramp is shared; the neutral family is per theme.** `--nc-gray-*` is emitted once and
shared by every theme, so it takes the base tint hue and can follow no accent. Each theme's
`neutral` family takes `[neutral] tint_hue` when the spec sets one, and its own accent's hue
otherwise (`neutral_tint_hue` in `crates/noctua-engine/src/palette.rs`); the shipped spec sets
none, so each palette's neutral leans toward its own accent. **A chart set with no `hue_start`
starts from that same hue** — so every shipped set pins one, because letting the charts rotate
with the accent drops two palettes' series below the protanopia floor. The `cool` and `warm` hues are fixed in
`[neutral]`, where naming a hue is what asks for the variant. All three share step lightnesses,
because `neutral::place` depends only on `steps` and `density`, so `gray-7` swaps for
`gray-cool-7` without moving any contrast.

**A temperature is made of chroma, not hue.** Two tints are at most about the sum of their chromas
apart, so at the base's absolute chroma of 0.004 no hue placement separates anything. The variants
carry 0.16 and 0.165 — warm more, because the base tint is itself warm — and
`crates/noctua-check/src/spacing.rs` gates the peak, failing under a JND.

**A scale says which kind it is; never recognise one by its name.** A categorical set spreads hues
around the wheel and is checked pairwise; an ordered one walks a hue path read in order (mistaking
`level-2` for `level-7` loses precision, not meaning) and is checked for separable neighbours,
opposed ends and monotone simulated lightness. `ResolvedScale::kind` carries the kind into the
JSON, and both kinds live in `ResolvedMode::scales`, so an emitter loops. **Hue and lightness are
placed separately** — `ordinal::place` by arc length, lightness by stop index — because one
combined measure bunches stops where a dichromat sees least (0.0192 apart under protanopia).
**`lightness_spread` is signed, and the sign is worth 2×**: the shipped ordered scales descend
into red, where protanopia's darkening adds to each step (0.0447) instead of cancelling it
(0.0185).

### The generated outputs

**The semantic layer is emitted once, not per theme.** `--nc-color-rejected: var(--nc-danger-solid)`
is the same sentence in every palette, so it lives in `system/css/contexts.css` and at the top of
`system/json/palette.json`, and a theme carries only what its `[themes.<name>.semantic]`
overrides; per theme it would be most of `system/`. The shared block is **`:where(:root)`**, at
zero specificity, so a theme's override wins whatever the link order. The split is **derived from
the resolved palette** — a slot is shared because every theme resolved it alike — and
`tokens::semantic_layer` owns it; every emitter asks it.

**A `var()` resolves where it is declared, so the shared layer does not follow a subtree.** The
contract, the numbered steps and the alpha ladder are declared once, on the root. A `data-theme`,
a `.light` or `.dark` class, or a `data-palette` on a smaller element re-themes only the
`--nc-<family>-<role>` tokens inside it, and every `--nc-color-*` there keeps the root's value: a
`.dark` box in a light page paints `var(--nc-color-surface)` light. Mode and palette switch fully
on the root. `README.md` tells consumers so, and `noctua-design` re-declares the contract per
scope in its own `../noctua-design/dist/scope.css`.

**A theme file holds one palette's values and nothing shared.** `system/css/ochre-balanced.css`
(the default, on `:root`) and each `system/css/theme-<name>.css` (on `[data-palette="<name>"]`)
carry only `--nc-<family>-*`; the dense ramps are in `system/css/ramp.css` and the contract in
`system/css/contexts.css`. Linked alone, a theme file leaves `--nc-gray-4` and
`--nc-color-success` undefined, and CSS drops an undefined custom property without a word; the
`references` gate catches that here, and nothing catches it in a consumer. **The answer is
`system/css/palette/<name>.css`**: self-contained, and bound to `:root` whichever theme it carries,
because a scoped rule copied into a standalone file defines nothing; a test asserts it.
`system/css/index.css` imports everything, for switching at runtime.

**Only the Tailwind target emits `--color-*`.** Tailwind v4 owns that namespace and cannot rename
it, and its entry points import the plain CSS, so everything else keeps the `--nc-` prefix.
`system/tailwind/bridge.css` maps `--color-surface: var(--nc-color-surface)` under
**`@theme inline`**, because a plain `@theme` bakes values at build time and dark mode never
applies. Its `@custom-variant dark` matches the `.dark` class and `data-theme="dark"`, not the
system preference.

**Qt parses hex and nothing else.** The QML singletons in `system/qml/`, one per theme and mode,
carry no `oklch()` and no custom properties. Qt's eight-digit hex is **ARGB**, so
`value::hex_argb` and `hex_rgba` are separate functions: the same digits read as two plausible
colours, never as an error. A reserved word as a property name takes the whole singleton down, so
`name::qml_property` escapes against `QML_RESERVED`, **the full keyword set** — `readonly` and
`required` are QML keywords no JavaScript list has, and both are shipped contexts.
`every_slot_survives_every_target` checks the emitted names.

**The alpha ladder is real alpha, and no gate can audit it.** In CSS a stop is
`color-mix(in oklab, <token> N%, transparent)` — the token at that opacity over any backdrop — and
eight-digit hex elsewhere; deliberately not a hex solved against one fixed backdrop, as Radix
does. An alpha token has no contrast until composited, so `contrast::pairs` cannot include one; do
not add a gate that pretends otherwise. The ladder exists for the neutrals and the accent only: a
`danger` wash would be a worse `danger-bg`, which is solved and checked.

**DTCG output follows Format Module 2025.10.** A colour `$value` is an object — `colorSpace:
"oklch"` and `components`, plus the six-digit `hex` fallback — which Style Dictionary v5 reads by
default. Translucency goes in `alpha`, because `hex` must be six digits, and
`colors.noctua.relativeChroma` is the only thing in `$extensions`. The shape is public API under
`TOKEN-POLICY.md`.

**The generated Rust crate has its own `[workspace]` and one feature per theme.** Without the
`[workspace]`, `cargo build` inside `system/rust` fails and vendoring absorbs it into the
consumer's workspace. The features keep `.rmeta` small — every consumer loads the metadata of
whatever is compiled in — rather than compile time. The default feature is the first theme, the
one the CSS binds to `:root`, and the sync check skips the `target/` and `Cargo.lock` that
compiling the crate leaves in `system/rust`.

### The npm package

**`sideEffects` is the array `["**/*.css", "**/*.scss"]`, never `false`.** `false` lets a bundler
tree-shake every stylesheet nobody imported a binding from, and the consumer gets an unstyled
application with no error. `@radix-ui/colors` ships `false` correctly, because it ships only
JavaScript; do not copy it. `packaging::side_effects_cover_css` rejects `false` and an array
without CSS.

**npm does not auto-include `LICENSE-MIT`.** Its `LICENSE*` rule misses the hyphenated
dual-licence names, so both files are listed in `files`. `npm pack --dry-run --json` is the only
way to see what ships.

### The documentation site

**The palette page renders one palette and builds the rest in the browser**, because every
palette server-rendered would make a page of megabytes. `renderRamps` in `docs-site/js/site.js`
mirrors `ramp_table` and `swatch` in `crates/noctua-docs/src/sections.rs`, and
`the_two_ramp_renderers_agree` fails if the Rust grows a `data-` attribute the script does not
write. In each palette's JSON, `semantic` maps a token to a step and `slots` maps a slot to a
family; neither derives from the other.

**Restoring `data-palette` is not restoring the palette.** The inline bootstrap in
`crates/noctua-docs/src/page.rs` sets the attribute before the first paint and injects the sheet
that defines it, with **`blocking="render"`** — a stylesheet inserted by script does not block
rendering otherwise, as `PerformanceResourceTiming.renderBlockingStatus` shows. It reads the
`noctua-palette` key the script writes on every change, and
`the_script_writes_the_palette_key_the_bootstrap_reads` holds the two halves together. It also
starts the palette JSON fetch and parks it on `window.__noctuaThemeFetch`, since the script must
first resolve the palette from the axes JSON.

**Nothing may flash on reload.** Swatches paint from `var(--nc-<stem>)`, and the JSON only
refreshes their numbers. Mode-dependent blocks are shown by `[data-mode]` in
`docs-site/css/site.css`, never by script, so the right one paints first, and with script off.
Only elements below the fold get the `.reveal` fade.

**Webfonts use `font-display: optional`, and all three faces are preloaded** by
`page::font_preloads`. `swap` would reflow every column of a page of hex tables when a face lands;
`optional` never swaps, so a face not requested before the first paint falls back for that load,
differently on each reload.

**The site is built once per language.** Strings are inline `t(locale, "English", "Português")`
calls, so both exist or neither compiles, and the Portuguese page is a sibling suffixed `.pt`,
because asset paths are relative. Only the unsuffixed page may redirect to a stored preference: a
URL that names its language is a shared link, and it wins. Text a script writes comes from a
`data-` attribute.

**`getComputedStyle(el).color` keeps the authored space; it does not return `rgb()`.** An
`oklch()` token computes to that string, and its numbers read as 0–255 channels measure every pair
as Lc 0. Paint into a 1×1 canvas and read the pixel.

**Two wasm-bindgen traps, both silent.** `wasm-bindgen-cli-support` is required at a full patch
version in `Cargo.toml`: a bare `"0.2"` resolves to 0.2.1, a 2018 release that does not compile.
And `init()` with no argument calls `WebAssembly.instantiate(undefined)`: pass
`{ module_or_path: new URL("...", import.meta.url) }`.

### Tooling

**A gate can be quietly useless in two ways.** It can inspect its own output: `check` calls
`build::palette`, which touches no disk, never `build::run`, or the hand-edit guard compares
`system/` with itself. And it can be wired to nothing: assert that a new gate's `checked` count is
non-zero.

**`Path::starts_with` does not understand `..`**, so `<root>/../noctua-design` looks inside
`<root>` and `export`'s "outside this repository" notice would never fire. `export::normalize`
resolves `..` and `.` lexically first — not with `fs::canonicalize`, which fails on a consumer
directory that does not exist before the first export.

**Your own tooling can lie.** `cargo fmt` can move an `allow-literal:` marker off the line it
excuses: bind fixtures to one-line `let`s. A stale `cargo xtask dev` keeps rewriting
`target/system/` with an old binary, so an edit seems to do nothing: find it with
`pgrep -af xtask` and stop it by PID, never with `pkill -f` and a pattern your own shell's command
line contains, which kills that shell. And shells differ: zsh does not word-split an unquoted
`$var` and POSIX `sh` does, so quote every expansion.

**Two workspace facts.** `clippy::float_cmp` is allowed in test modules only, where exact
comparison against a sentinel a function returns verbatim *is* the assertion. And this is a
virtual workspace with no root package, so a root `tests/` of test code would never run:
integration tests live in the crate they exercise.

## Conventions

- **Naming.** Test names are sentences stating the property checked
  (`everything_below_max_chroma_is_in_gamut`), not `test_foo`. Public items carry doc comments;
  `missing_docs` warns, and `check` denies warnings.
- **Comments explain why, never what** — a decision, a measurement, or a trap.
- **Errors.** `thiserror` for library types, `miette` at the CLI edge. A diagnostic points at the
  exact span and ends with an actionable fix.
- **Tests.** Unit tests inline in `#[cfg(test)] mod tests`; property tests in
  `crates/<crate>/tests/properties.rs`. A test that found a real bug says so in a comment.
- **Floats.** `f64` throughout, quantized once at palette construction, so what the gates
  checked is byte-for-byte what ships.
- **Commits.** Conventional commits, in English, signed, made whenever the work is ready without
  asking, and nothing a commit says credits an agent (`NOCTUA.md §2.6`).

## Dependencies

Each third-party crate has one job, and adding another is a conversation with the developer
first (`NOCTUA.md §2.3`). Versions live in `Cargo.toml`, pinned by `Cargo.lock`.

| Crate | For |
|---|---|
| `indexmap` | maps whose insertion order is the contract (invariant 4) |
| `serde`, `serde_json`, `toml` | reading the spec and writing JSON; `preserve_order` keeps authored order |
| `blake3` | the content hashes in `system/MANIFEST.json` and `system/json/axes.json` |
| `thiserror`, `miette` | library error types; span-pointing diagnostics at the CLI edge |
| `maud` | the site's HTML (`noctua-docs`) |
| `wasm-bindgen`, `wasm-bindgen-cli-support` | the playground's bindings, generated in process so nothing needs installing |
| `clap`, `anstream`, `anstyle` | `xtask`'s command line and terminal output |
| `notify`, `notify-debouncer-full`, `tiny_http` | `dev`'s file watcher and local server |
| `proptest` | property tests |

What this repository publishes depends on nothing: the generated crate has no dependencies, and
`xtask/src/packaging.rs` refuses any dependency or script in `package.json`.

## Vendored assets

**`docs-site/assets/fonts/` is committed by hand and is not build output.** Nothing regenerates
it, and it must never be deleted as a stale artifact; it is the only vendored binary here. It
holds three faces of Noctua Iosevka, **subset** to what the site renders: Bold is not optional,
and Italic is the true italic, not the slanted roman. The SIL OFL 1.1 travels with the files in
`docs-site/assets/fonts/OFL.md`, and `docs-site/assets/fonts/ATTRIBUTION.md` holds the
`pyftsubset` invocation and the reasoning. The typeface comes from the `noctua-fonts` repository
(`NOCTUA.md §6`).

**`.TEMP/` is the landing area for future hand-vendored inputs.** Its contents are gitignored;
only `.TEMP/.gitkeep` is tracked (`NOCTUA.md §3.7`).

## Publishing

**Remote:** `git@github.com:noctua-world/noctua-colors.git`, public. `system/` is generated
**and committed**, which is what lets every channel work with no build step:

| Channel | Artifact | Published by |
|---|---|---|
| npm `@noctua-world/colors` | the **curated** set: css, tokens, scss, tailwind, `system/json/axes.json`, the manifest | `.github/workflows/release-npm.yml` |
| crates.io `noctua-colors-tokens` | the generated crate from `system/rust` | `.github/workflows/release-crate.yml` |
| crates.io `noctua-colors` | the facade crate that holds the name | `.github/workflows/release-crate.yml` |
| GitHub Release | the **complete** `system/`: tarball, zip and `SHA256SUMS`, attested | `.github/workflows/release-github.yml` |
| jsDelivr, unpkg | npm-backed and `gh`-backed | nothing — automatic |
| Cargo git dependency, Nix flake, submodule, copy | the complete `system/` | nothing — the tag is enough |
| GitHub Pages | the documentation site | `.github/workflows/pages.yml`, on push to `main` |

**Cutting a release.** `cargo xtask release <version>` runs the full gate, writes the version into
the spec's `[system]` table **and `package.json`**, rebuilds so `system/MANIFEST.json` and the
generated crate's manifest follow, and warns if `CHANGELOG.md` has no entry. `--tool <version>`
bumps the compiler's version in `Cargo.toml` instead, and publishes nothing. The verb stops
there and prints the rest, to run once the release has been reviewed:

```sh
git commit -am "release: <version>"
git tag -a v<version> -m "noctua-colors <version>"
git push --follow-tags
```

**Pushing the tag is what publishes.** All three release workflows trigger on `v*.*.*`, and the
npm and crates workflows refuse a tag that differs from `package.json`'s version. Nothing
publishes on a push to `main`.

**There are no publishing secrets, and there must never be any.** npm and crates.io both use
**OIDC trusted publishing**: the workflow's own identity is exchanged for a credential that lives
minutes. Three consequences:

- **The workflow filenames are configuration.** npm's trusted publisher is registered against
  `.github/workflows/release-npm.yml` and crates.io's against
  `.github/workflows/release-crate.yml`, **by filename**. Renaming either
  breaks publishing, and the error does not mention the filename.
- **Only GitHub-hosted runners.** Trusted publishing and npm provenance both refuse self-hosted
  runners.
- **The first publish of a new package name is manual.** Neither registry has a pending
  publisher, and both need the package to exist before a trusted publisher can be configured, so
  that release's automated run fails. Re-running it does not help: a re-run uses the workflow file
  at the **tag's** commit, not `main`'s.

**A publish trusts the registry's answer to the publish, never a lookup before it.** The read
path lags the write path — `npm view` can return 404 while a PUT is rejected for already existing
— so a pre-flight lookup is only a fast path, and both release workflows treat "already
published", reported by the publish itself, as success.

**`npm pack --json` changes shape across npm versions**: an array of one entry before npm 12, an
object keyed by package name from 12. The npm workflow installs `npm@latest`, because trusted
publishing needs at least 11.5.1, so CI runs a newer npm than a developer's machine; anything that
parses npm's JSON accepts both shapes.

**`package.json` is the only hand-written manifest**, and it points at generated files, so
`cargo xtask check` verifies its claims in Rust, without Node: every `exports` target and `files`
entry exists, `sideEffects` covers CSS, the version equals the spec's, there are no dependencies
or scripts, and every relative `@import` resolves inside the tarball (`xtask/src/packaging.rs`).

## Maintaining this file

`NOCTUA.md §0.5` governs it. Update it **on your own initiative**, in the same change as the work
that invalidates or extends it.

- Record only what changes future behaviour — invariants, gotchas, decisions, recipes — never
  task history or anything a command answers. **Record the command, not the value.**
- **Write the present.** A finished piece of work leaves a rule here and its story in its commit
  message.
- **Compaction is part of maintenance.** Before adding, edit or merge an existing entry if one
  fits, and delete what no longer applies. Where a module doc carries the argument, keep only the
  rule and its measurement here. There is no line budget: an entry belongs if removing it would
  make a future agent err.
- **Code cites this file.** `xtask/src/release.rs` names **Publishing**, two `Cargo.toml` files
  name **Invariants**, and `crates/noctua-emit/src/tokens.rs` names invariant 4. Never rename a
  cited heading or renumber an invariant.
- **`NOCTUA.md §8` is this repository seen from outside.** A change that alters what a consumer
  sees — a name, a subpath, a file, a versioning rule — fixes §8 in the same change.
