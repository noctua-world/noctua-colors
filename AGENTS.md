# noctua-colors — Agent Guidelines

> ## Everything you write to disk is in English.
>
> Code, comments, identifiers, documentation, CLI help, error messages, test
> names, commit messages, the docs site — all of it, always, whatever language the
> developer writes to you in. You may *reply* in Portuguese; you may not *produce
> artifacts* in it. This diverges from the rest of the `noctua-*` fleet, which is
> pt-BR. The divergence is deliberate; do not "correct" it.

## What this is

A **color system compiler**. A small declarative spec goes in; every artifact
other projects consume comes out — CSS custom properties, a Tailwind v4 theme,
Rust constants, DTCG tokens, JSON/TypeScript, SCSS, and a QML singleton.

It is not a palette and not a UI library. **The repository versions the curves,
not the colors.** No hand-picked hex exists in the source. Two ideas carry it:

- **Relative chroma.** Chroma is a fraction `cr ∈ [0,1]` of what the target gamut
  can show at that lightness and hue, so one definition renders correctly on sRGB
  and more vividly on Display P3, and "sober ↔ vivid" is one multiplier.
- **Contrast-anchored steps.** A step's lightness is *solved* from a contrast
  target against a declared reference, not authored as a ramp.

## Status

Complete, end to end. **Nothing is committed yet** — the tree is untracked,
awaiting the first commit.

## Repository map

```
crates/noctua-core/   colour math: space, gamut + cubic (analytic chroma
                      boundary), map (CSS Color 4, hex), contrast (APCA + WCAG
                      reporting), diff, cvd (Brettel 1997), matrix
crates/noctua-spec/   TOML model, defaults, validation, miette diagnostics
crates/noctua-engine/ curves, contrast-anchored solving, palette construction
crates/noctua-emit/   the eight output targets, plus dist/ write and sync
crates/noctua-check/  gates: contrast, colour vision, spacing, source literals,
                      consumer token references
crates/noctua-docs/   site generator (maud), two locales. Reads dist/ only
crates/noctua-wasm/   browser bindings over the engine, for the playground
xtask/                the six verbs, plus the wasm bundle build
specs/noctua.toml     the spec — the only file a developer edits
dist/                 generated artifacts. COMMITTED. Never hand-edited.
docs-site/            site sources. public/ is build output and gitignored
examples/             consumer-rust and consumer-web, outside the workspace
tests/golden/         snapshot of the built palette
.TEMP/                hand-vendored inputs. Kept, contents gitignored. ```

**Dependency rule.** `noctua-core` has no workspace dependencies and never
will; it knows nothing of the spec format or the output targets. `noctua-emit`
performs **no color math** — it formats resolved, quantized colors. An emitter
that computes one makes gates and output disagree about what shipped.

## Invariants

Breaking any of these is a defect, not a tradeoff.

1. **No hardcoded color values outside the spec** — not in the engine, tests,
   docs site or examples. Construct them: `Oklch { l: 0.55, c: 0.15, h: 25.0 }`,
   or a fraction of `max_chroma` at a chosen hue, which covers the space
   deliberately rather than arbitrarily. Three narrow carve-outs: **hex string
   fixtures** where the *format* is under test (strings, not colors); **black
   and white**, the only colors with no free parameters; and **published
   reference constants** cited in a comment. Enforced by `noctua_check::source`,
   not by remembering — a literal needs an `// allow-literal: <reason>` marker on
   its line, which puts every exception in front of a reviewer. Golden files and
   `dist/` are exempt and both are tool-generated.
2. **Never clip per channel to fit a gamut** — it shifts hue silently. Use
   `map::map_into_gamut`. The one exception is `cvd::simulate`; see Gotchas.
3. **APCA is the design criterion. WCAG 2.x is reporting only.** No solver and
   no gate may target a WCAG ratio.
4. **Determinism.** Same spec plus same version produces byte-identical output:
   no timestamps, no random ordering, no `HashMap` iteration order reaching a
   generated file. `BTreeMap`, or `IndexMap` where insertion order is the
   contract.
5. **Generated files are never hand-edited.** Every one says so in a header.
6. **Stay in OKLCH.** CAM16 is not implemented and no stub exists for it: it
   needs viewing-condition parameters this project does not have, and its
   advantage for UI work is marginal. The `UniformSpace` trait keeps the door
   open; leave it shut until a *measured* failure justifies otherwise.
7. **Out of scope, permanently:** spectral data, chromatic adaptation,
   viewing-condition models, tone mapping. No stubs for them either.
8. **A context is an alias until proven otherwise.** A new `[families.*]` costs
   a hue the wheel has not got and 3.5 MiB; a `[semantic]` line costs 15 KiB.

## Commands

```bash
cargo xtask build     # compile the spec into every target under dist/
cargo xtask check     # the single gate: spec, gates, dist sync, fmt, lints, tests
cargo xtask dev       # watch the spec, rebuild, serve with live reload
cargo xtask export    # copy dist/ into every registered consumer
cargo xtask import    # fit an existing palette back to spec parameters
cargo xtask release   # prepare a version ```

`cargo xtask check` is what CI runs — the same command, not a reimplementation in
YAML. `--colors-only` skips fmt, lints and tests for a fast loop; CI never passes
it. `just` offers one-word aliases and is never the only path. New capability goes
behind an existing verb or it does not ship.

`import` reads color literals out of CSS, SCSS, QML, JSON or plain text, groups
them into ramps by name, and reports how closely the curve model expresses each.
`--gamut` measures `cr` against a gamut other than the spec's, which matters: a
palette authored for P3 has colors sRGB cannot show, and `cr` cannot exceed 1.

## Recipes

**Add a context.** One line in `[semantic]` mapping it to an existing family, and
it reaches every target. Read the hue-wheel entry below before adding a
`[families.*]` instead — the answer is almost always an alias.

**Add a family.** One block with a `hue`; the rest defaults. Rebuild `dist/` and
the golden, then read the new CVD margins.

**Add a scale.** One `[[scales]]` block. `stops` is a count or a list of names;
reversing the hue knots reverses direction, and `lightness_spread` may be negative.

**Add an accent or a saturation.** One line in `[accents]` or `[saturations]`, plus
a label in `controls.rs` or the picker shows the raw identifier. An accent in the
158-172 arc needs re-measuring first. A one-off palette not on the grid is still
`[themes.<name>]`.

**Add an emitter.** One file in `crates/noctua-emit/src/` implementing `Emitter`,
plus a line in `registry()` and a `pub mod`. Never a refactor. Use `tokens::` for
what to emit and `value::` for how to spell a color, so the new target cannot
drift from the others.

**Retune contrast.** Edit role targets in `crates/noctua-spec/src/defaults.rs`
or override `[[scale.roles]]`. Light and dark are not mirror images — see below.

**Add a quality gate.** A module in `crates/noctua-check/src/` returning a
`Report` of `Finding`s, plus a line in `run()`. Every finding needs a location, a
message saying what to do, and a margin where one is meaningful — a verdict alone
tells nobody how close they were.

**Add a page to the site.** A module in `crates/noctua-docs/src/` plus an
`Output` per locale in `render()`; the tests read `render()` itself.

**Change a `#[wasm_bindgen]` signature.** `cargo xtask build` checks that every
name `docs-site/js/playground.js` imports is one the bindings export. The JSON
*shape* is checked in `noctua-wasm`'s own tests — that contract broke once.

## Gotchas

Every entry below cost real debugging time in this repository.

**The sRGB gamut is not convex in Oklab; bisecting chroma gets it wrong.** At
lightness 0.4525 and hue 264.1 the red channel crosses zero *three* times, so a
bisection converges to whichever crossing its midpoints bracket and maximum chroma
jumps 19% between hue 264.0 and 264.1. `max_chroma` therefore *solves* six cubics
and takes the smallest positive root — also the **first** crossing, which relative
chroma requires. Never replace it with a search;
`the_srgb_gamut_is_non_convex_near_the_blue_primary` pins it, and `gamut.rs`
carries the derivation.

**Display P3 is not a subset of Rec.2020.** P3's red primary is at xy (0.680,
0.320) and Rec.2020's red-green edge passes through y = 0.31828 there, so a sliver
of orange-red is in P3 and not Rec.2020 — sRGB is inside both. "Wider gamut" is
not a total ordering.

**Two constants that look wrong and are not.** `screen_luminance` uses a plain 2.4
power with no linear toe, because APCA's constants were fitted against that curve;
`wcag_luminance` below it uses the real one, and they must differ. And the
published Oklab matrices land D65 white on 0.999_998_8 — fatal to a compiler
anchoring steps on lightness — so `space.rs` conditions both at compile time.
Never paste the raw matrices back in.

**`cvd::simulate` clamps channels — the one place per-channel clamping is right.**
The dichromatic surface is not inside the sRGB cube (a saturated blue for a
protanope returns red ≈ −0.32), and mapping cannot rescue a colour with no
meaningful hue to preserve. The cost: simulation is only *near*-idempotent, so
the tests scope that law to projections landing in gamut.

**Two ways a gate can be quietly useless.** `check` must never write `dist/`
before inspecting it — it once compared the artifacts against themselves, so the
hand-edit guard said "in sync" right after a hand edit (`build::palette` compiles
without touching disk; `build::run` writes). A gate never *called* checks nothing:
the colour-vision simulator was complete, tested and wired to nothing for three
milestones, then found 175 real failures the hour it was connected — assert a new
gate's `checked` count is non-zero.

**Semantic colors cannot be made dichromacy-safe by color alone.** Measured by
search over every shift combination: the best worst-case separation a six-family
set reaches is **0.0163**, under one JND, and there are now ten families. Hue is
the axis dichromacy removes and there is not enough lightness left to replace
it. The gate reports margins and warns, failing only on identical colors — and
reports **one finding per pair**, naming the worst palette, or thirty-six
palettes would bury every other gate. Never "fix" this by raising the shifts or
lowering the target.

**`OPPOSED` is curated; `margins` measures everything.** Forty-five pairs gated
would be a hundred warnings restating the entry above, so the gate lists only
pairs whose confusion changes what the interface *said* — "it worked" for "it
broke", waiting for running. `margins` returns every pair and
`dist/reports/colour-vision.md` publishes it, so an awkward collision stays
findable without burying a dangerous one. Extend `OPPOSED` for a meaning, never
for tidiness.

**The hue wheel is full.** Sixteen hues were already taken by five families and
twelve accents — 22.5 degrees of average spacing — and 158-172 is unusable, so
only three gaps sat 14 degrees clear of both neighbours: 45, 74.5, 264. `waiting`
(117.5) and `active` (203) are 12.5 and 9 degrees from the nearest *accent*
deliberately: an accent is chosen by whoever installs the palette and a status
colour is not, so the accent gives way, and separation from another *family* is
the constraint that holds. **There is no room for an eleventh** — see invariant 8.

**Lc is printed at four decimals, everywhere.** At one, a pair short of its
target by 0.0003 printed as `Lc 45.0, needs 45` beside a failure mark — the
number said it met the target and the mark said it did not. Four is what the
palette is quantized at, so nothing displayed can round across a threshold it did
not cross. The site's figure comes from the *painted pixel* through a canvas, so
its last decimal differs from the compiler's, which measures its own values;
that is the honest quantity for a page claiming to measure what it paints, and
the page says so.

**Two role targets carry an allowance, and both are measured rather than
chosen.** The dark `solid` aims at **45.25**: a solid is solved against its own
family's `bg-app` but shipped against the page, which is the neutral's, and that
hair of tint chroma moves the measured contrast by up to 0.016 Lc — enough to
leave six accent hues a ten-thousandth under the gate. The quarter clears it
fifteen times over and no more: a full Lc was tried and walked `progress`'s dark
solid into its own `border-strong`, because that family's shift is large enough
to put its solid *below* the border rather than above, so a rising target closes
the gap. The light `text-muted` aims at **78** to shrink the ramp's one
structural gap — `text-muted` to `text-strong` is a 24 Lc jump that light
polarity compresses into a quarter of the lightness range, measured at 0.2684
against a 0.26 ceiling while every other pair in the same ramp sat under 0.15.

**Solids deliberately leave the ramp.** A solid is picked to be recognised as a
brand or a state, not to occupy a rung — which is what lets semantic families
separate in lightness. Gates checking monotonicity, adjacent-step distance or
cross-family agreement skip `Palette::shiftable_roles`: accidental drift, not
intent.

**Gamut mapping must report coordinates that are in gamut,** and both obvious
choices are wrong: the bisection's last candidate is not (re-mapping eroded chroma
by 7e-5 a pass) and the clipped color's coordinates shift hue by up to 15 degrees.
Keep lightness and hue, clamp chroma to `max_chroma`. The property test asserts
*exact* equality — a tolerance would have hidden the bug.

**Colour vision deficiency can push two colors *further* apart.** The projection
contracts distances in linear RGB, but ΔE is measured in Oklab and the two
half-planes apply *different* matrices — 1.34x expansion under protanopia,
measured over 10648 colors. Never assume contraction.

**A gamut lookup table was built, measured, and deleted.** Measure before adding
one back: 329 ns analytic against 44 ns table, but 43 ms to build — break-even at
~152,000 lookups, and its worst error (0.014 chroma) landed at the blue primary.

**Light and dark are not mirror images; copying values across breaks the ramp.**
APCA's reverse polarity is far less sensitive — a separation of 0.265 from the app
background reads **Lc 46 in light and Lc 15 in dark**, so dark needs roughly
**1.45x**. Dark *solid* targets are lower too: equal Lc gives a paler brand, since
Lc 58 against a dark ground means lightness 0.75. Text targets stay near-equal.

**Two targets have a quirk that bites once.** Qt parses hex and nothing else, so
`Theme.qml` has no `oklch()` and no custom properties — and eight-digit hex in Qt
is **ARGB**, not RGBA: alpha leads, which is why `value::hex_argb` and
`hex_rgba` are two functions rather than one with a flag. The same eight digits
read as two different colours, so a mistake there produces a plausible result
instead of an error. Qt also rejects a JavaScript reserved word as a property
name and takes the whole singleton down with it — `new` is a real context in the
shipped spec, so `name::qml_property` escapes it. Tailwind's `@theme` must be
`@theme inline`, because a plain `@theme` bakes the value at build time and the
dark-mode override never applies; the utilities then look right in light mode and
are frozen there.

**The alpha ladder is real alpha, and no gate can audit it.** Stops are
`color-mix(in oklab, <token> N%, transparent)` in CSS — premultiplied, so that
*is* the token at that opacity over any backdrop — and eight-digit hex elsewhere.
Deliberately **not** Radix's model of solving a hex that composites to a target
over one fixed backdrop. The cost has to stay documented: contrast is a property
of two opaque colours, so an alpha token has none until composited and
`contrast::pairs` cannot include one. Do not add a gate that pretends otherwise.
Emitted for the neutrals and the accent only — a `danger` wash would be a
worse-specified `danger-bg`, which is a solved step with checked contrast.

**`on-{slot}` on `{slot}` is gated at 30, not 45, because it is arithmetic.**
`on-danger` is that family's lightest step and `danger` its solid, which the
engine solved to sit exactly `45 + contrast_shift` Lc from it in dark mode — so
the pair lands between Lc 31 and 43 for the five families with a negative shift,
and no edit here could raise it. **A status fill is not a text background;** `fg`
on `{slot}-bg` is, gated at 90.

**The semantic layer and the pair table are generated from two shape tables in
`tokens.rs`,** driven by `mode.semantic`, where every family is a slot of its own
name. Hand-written they were readable at six contexts and would be four hundred
lines at thirty, and their defect mode is a *missing* row — invisible, because
nothing reports a pair nobody thought of. Four of twenty-three tokens were
ungated exactly that way (every `*-border`). `contrast::pairs` groups rows by
family, so four aliases of `danger` do not report one failure four times.

**A scale is not a chart, and three facts about ordered scales cost measurement.**
`[chart]` spreads hues *around the wheel* for a legend and is checked pairwise;
`[[scales]]` walks a hue *path* to be read in order, where pairwise is the wrong
property (confusing `level-2` with `level-7` loses precision, not meaning) and the
checks are instead neighbours separable, ends opposed, simulated lightness
monotone. Both live in one `ResolvedMode::scales` map keyed by stem, so an emitter
loops. **Hue and lightness are placed separately** — `ordinal::place` by arc
length on one lightness slice, lightness by *stop index* — because one combined
measure lets them trade and bunches stops exactly where hue does the most work and
a dichromat has the least: 0.0192 apart under protanopia, inside a JND. And
**`lightness_spread` is signed and the sign is worth 2x**: both shipped scales
descend into red, because protanopia darkens reds sharply, so descending has the
deficiency *adding* to each step (0.0447) where ascending has it cancelling
(0.0185).

**The generated Rust crate: its own `[workspace]`, one feature per theme, and its
build output skipped.** Without the `[workspace]`, `cargo build` inside
`dist/rust` fails and vendoring it absorbs it into the consumer's workspace. The
features are not about compile time — half a second for the lot — but `.rmeta`,
which every consumer naming the crate loads for whatever is compiled in:
megabytes at thirty-six palettes, where `examples/consumer-rust` uses four. The
default is the first theme, the one the CSS binds to `:root`. And compiling it
leaves `target/` and `Cargo.lock` inside `dist/` — byproducts of *using* the
artifacts — so the sync check skips both, or one `cargo build` reports hundreds
of stale files.

**Two workspace facts.** `clippy::float_cmp` is allowed in test modules only,
where assertions compare against literal sentinels the functions return verbatim
and exact comparison *is* the assertion. And this is a *virtual* workspace with no
root package, so a root `tests/` directory would silently never run — integration
tests live inside the crate they exercise.

**Relative chroma cannot exceed 1, so a palette authored for a wider gamut cannot
be expressed in a narrower one.** Fitting Tailwind v4 against sRGB left amber,
orange and yellow outside a JND — not because the model is too weak, but because
those ramps sit up to 13% beyond what sRGB shows. Against P3 the worst residual
halves. Check the gamut before changing the model: `cargo xtask import <file>
--gamut display-p3`.

**Two wasm-bindgen traps, both silent.** `wasm-bindgen-cli-support` must be
pinned to the *exact* version — `"0.2"` resolves to 0.2.1, a 2018 release that does
not compile. And `init()` with no argument reaches
`WebAssembly.instantiate(undefined)`: pass
`{ module_or_path: new URL("...", import.meta.url) }`.

**`getComputedStyle(el).color` does not return `rgb()`.** A browser preserves the
authored space, so an `oklch()` token computes to that *string* — and its first
three numbers, fed to something expecting 0-255 channels, made every pair in the
site's contrast matrix measure Lc 0.0 and report as failing with nothing saying
so. Paint into a 1x1 canvas and read the pixel; never parse the string.

**A build with no failures and no warnings is the target, and `Severity::Note`
is what makes it honest.** Three levels, and the lower two mean different
things: a **warning** says *a different choice would fix this*, so it should be
fixed rather than shipped; a **note** says *this is the measured limit*. The
twenty-nine colour-vision findings are notes because no palette can clear them —
`cvd` is the only gate that emits one, and only above its floor, since two
colours that are literally the same is still a defect. Do not reclassify a
warning to quiet it: the reason the split exists is that a permanently yellow
build teaches people to skip the yellow lines, which is where a real regression
would appear.

**Three deliberate duplications of the contract, each guarded by an xtask or
docs test.**
`sections.rs` holds a readable *sample* of the pair table because `noctua-docs`
never depends on the gates, and `contrast::semantic_view` re-derives the contract
rather than importing `tokens::semantic_tokens`, because a gate that imported the
emitter's view would agree by construction. Both are compared, by
`the_site_and_the_gate_agree_on_every_pair` and
`the_gate_and_the_emitter_resolve_every_token_alike`. The third is the palette
stylesheet's href, spelled in both `page.rs`'s inline bootstrap and `site.js`
and compared by `the_bootstrap_and_the_script_build_the_same_stylesheet_path`.
Independence nobody compares is just drift.

**Three ways your own tooling lies to you.** `cargo fmt` can separate an
`allow-literal:` marker from the literal it excuses, so the source gate fails a
file that was passing — bind fixtures to one-line `let`s. A stale `cargo xtask
dev` rewrites `dist/` with an old binary while you debug why your edit "did
nothing"; `pkill -f xtask` (not `xtask dev`, which misses a `setsid`-detached one)
before trusting a build. And the shell here is **zsh**, which does not word-split
an unquoted `$var` — a sweep script that relied on it silently wrote a malformed
spec and every reading came back identical.

**Palettes are a grid, expanded in `noctua-spec` before anything else sees them.**
`[accents]` x `[saturations]` becomes ordinary `[themes.*]` in `expand.rs`, so the
engine, emitters and gates never learned about axes, and a generated theme is
exactly what a hand-written one would have been — a shorthand, not a second model.

**One accent band is unusable, and not the obvious one.** Swept in 2.5-degree
steps: an accent between roughly 158 and 172 degrees collapses onto `danger` under
protanopia, worst at 165 where the two sit **0.0018** apart. With 130-158 already
too close to `success`, every green above lime is out. Re-measure before adding a
hue in that arc.

**The neutral *family* follows its palette's accent; the dense ramp does not.**
`--nc-gray-*` is emitted once and shared, so it cannot lean toward any one accent,
while the per-theme `neutral` family does — which is what makes a blue palette's
surfaces read blue-grey. `cool` and `warm` follow neither: their hues are fixed in
`[neutral]`, where naming a hue is what asks for the variant. All three share the
same step lightnesses, because `neutral::place` depends only on `steps` and
`density`, which lets `gray-7` be swapped for `gray-cool-7` without moving any
contrast.

**A temperature is made of chroma, not hue: two tints are at most about the sum
of their chromas apart.** At the base's 3% — absolute chroma 0.004 — no hue
placement separates anything, which is how all three ramps shipped
indistinguishable, `gray-warm` byte-identical to `gray` and `gray-cool` 0.0098
away despite sitting 191 degrees off. The variants carry 0.16 and 0.165, and warm
needs *more* than cool because the base tint is itself warm. `spacing.rs` has the
argument and now gates the peak, failing under a JND.

**The docs page renders one palette and builds the rest in the browser.** All
thirty-six would be a 2.4 MB page of seventeen thousand nodes. `sections.rs` emits
the default; `renderRamps` in `site.js` mirrors `ramp_table`/`swatch` for the
others, from `tokens/json/themes/<name>.json`. Two renderers is the cost, and
`the_two_ramp_renderers_agree` fails the build if the Rust grows a `data-`
attribute the script does not write. In that JSON `semantic` is *token* to step
and `slots` is *slot* to family; neither derives from the other.

**Only the Tailwind target may emit `--color-*`;** everything else this project
emits carries the prefix, including the semantic layer, which is
`--nc-color-surface`. Tailwind v4 fixes its theme namespace at `--color-*` and
nothing can rename it, and `tailwind/theme.css` imports `css/index.css` — so a bare
layer in the plain CSS would claim 150-odd names in Tailwind's namespace for a
consumer who never installed it. Tailwind maps
`--color-surface: var(--nc-color-surface)`, so the contract is defined once.

**A theme file is one palette, and renaming a theme renames it.**
`ochre-balanced.css` carries the semantic layer plus every `--nc-<family>-*`; the
dense ramps (`--nc-gray*-*`) are in `ramp.css` and the other palettes in files of
their own, and `index.css` imports all thirty-six, which is two megabytes. Asking
for `--nc-gray-4` having linked only a theme file fails *silently*, because CSS
drops an undefined custom property — the `references` gate catches that in-repo.
And a rename moves the file, so `README.md`, `examples/`, the integration snippet
in `sections.rs` and `noctua_docs::token_files` must move with it.
`dist/css/index.css` is the one name that never does.

**Restoring `data-palette` is not restoring the palette.** The inline bootstrap
sets the attribute before the first paint, but the sheet that *defines*
`[data-palette="umber-balanced"]` is a separate file — so for two milestones the
page painted in the default theme and snapped over when `site.js` fetched the
right one, on every reload. The bootstrap now injects that sheet itself, and
**`blocking="render"` on it is load-bearing**: a stylesheet inserted by script is
*not* render-blocking by default, and Chrome says so through
`PerformanceResourceTiming.renderBlockingStatus` — "non-blocking" without the
attribute, "blocking" with it. Without it the injection changes nothing visible.

**Webfonts are `font-display: optional`, not `swap`.** Both avoid blocking on the
download; only `optional` avoids the reflow afterwards, and on a page that is
mostly tables of hex values a swap moves every column. The faces are preloaded
and total 34 KB, so the browser has them in time on every visit but the first
with a cold cache — and that one renders in a platform monospace, which is what
the fallback stack is for.

**The site is built once per language, not translated at runtime.** Strings are
inline `t(locale, "English", "Português")` calls, so both exist or neither
compiles, and pages are siblings (`index.pt.html`) because asset paths are
relative. Only the unsuffixed page may redirect to a stored preference — a URL
naming its language is a shared link and has to win. Anything a *script* writes
takes its wording from a `data-` attribute.

## Conventions

- **Naming.** Test names are sentences stating the property checked
  (`everything_below_max_chroma_is_in_gamut`), not `test_foo`. Public items carry
  doc comments; `missing_docs` warns and `check` denies warnings.
- **Comments explain why, never what** — a decision, a measurement, or a trap.
- **Errors.** `thiserror` for library types, `miette` at the CLI edge.
  Diagnostics point at the exact span and end with an actionable fix.
- **Tests.** Unit tests inline in `#[cfg(test)] mod tests`; property tests in
  `tests/properties.rs`. A test that found a real bug says so in a comment.
- **Commits.** Conventional commits, in English. Commits on this machine need a
  physical YubiKey touch — never commit unprompted.
- **Floats.** `f64` throughout, quantized once at palette construction, so what
  the gates checked is byte-for-byte what ships.

## Vendored assets

**`docs-site/assets/fonts/` is committed by hand and is not build output.**
Nothing regenerates it and it must never be deleted as a stale artifact — the only
vendored binary here. Three faces of Noctua Iosevka, **subset** from the 135-face
upstream build (4,998,164 bytes to 34,548); Bold is not optional and Italic is the
true italic, not the slanted roman. `SIL OFL 1.1` must travel with the files and
does, in `OFL.md`. The `pyftsubset` invocation and the reasoning are in
`ATTRIBUTION.md`; source is `noctua-font/noctua-iosevka/`. `.TEMP/` is the landing
area for future hand-vendored inputs — contents gitignored, `.gitkeep` tracked.

## Maintaining this file

Update it **on your own initiative** whenever a task invalidates or extends it.

- Record only what changes future behaviour: invariants, gotchas, decisions,
  recipes. Never task history, changelogs, or anything obvious from the code.
- **Compaction is part of maintenance.** Before adding, check whether an entry
  should be edited or merged instead, and delete what no longer applies.
- **Denser over time, not longer.** Where the reasoning already lives in a module
  doc, keep the rule and the measurement here and let the code carry the
  argument. There is deliberately **no line budget** — one lived here for two
  sessions and cost more than it saved, because it turned every addition into a
  round of compression that traded content for line count.
- Every entry earns its place: if removing it would not cause a future agent to
  err, it does not belong. Length is not the test; that is.
