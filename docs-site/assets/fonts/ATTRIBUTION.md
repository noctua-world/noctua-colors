# Fonts

## What these are

Three faces of **Noctua Iosevka**, a custom build of
[Iosevka](https://typeof.net/Iosevka/) by Renzhi Li (Belleve Invis),
licensed under the SIL Open Font License 1.1. The full licence is in
[`OFL.md`](OFL.md) and must be shipped wherever these files are.

The build plan lives outside this repository, at
`noctua-font/noctua-iosevka/private-build-plans.toml`, together with its
rebuild guide. That is the source of truth for the typeface; this directory
holds only what the documentation site serves.

## Why only three

Iosevka builds 135 faces — five widths, nine weights, three slopes. A
documentation site needs a fraction of that, and every extra file is payload a
visitor downloads and never sees.

| File | Used for |
|---|---|
| `NoctuaIosevka-Regular.woff2` | body, tables, hex and OKLCH columns |
| `NoctuaIosevka-Bold.woff2` | headings and emphasis. Synthesised bold on a monospace face looks broken, so this one is not optional |
| `NoctuaIosevka-Italic.woff2` | captions and inline terminology. The true italic, with its own letterforms — not `Oblique`, which is the roman slanted |

Everything else is dropped: all four alternate widths, the other six weights,
and every `Oblique`.

## Why they are subset

The upstream faces carry Greek, Cyrillic, Armenian, IPA, box drawing, Powerline
glyphs and roughly 1,500 mathematical operators. The site renders about 250
codepoints of that.

```
Regular   1,617,752 B  ->  10,912 B
Bold      1,630,316 B  ->  10,952 B
Italic    1,750,096 B  ->  12,684 B
                          --------
          4,998,164 B  ->  34,548 B     145x smaller
```

Shipping five megabytes of fonts to a site whose stated bar is "genuinely
impressive on a phone" would have dominated every other performance decision
made anywhere in the project.

## How to regenerate them

One command, offline, no permanent dependency. Run from the repository root
with the upstream faces available:

```bash
nix-shell -p python3Packages.fonttools python3Packages.brotli --run '
  pyftsubset <source>/NoctuaIosevka-Regular.woff2 \
    --output-file=docs-site/assets/fonts/NoctuaIosevka-Regular.woff2 \
    --flavor=woff2 \
    --unicodes="U+0020-007E,U+00A0-00FF,U+0394,U+2010-2015,U+2018-201F,U+2020-2022,U+2026,U+202F,U+2030,U+2032-2033,U+2039-203A,U+2044,U+2070,U+2074-2079,U+2080-2089,U+20AC,U+2122,U+2190-2193,U+21D2,U+2206,U+2212,U+2248,U+2260,U+2264-2265,U+25A0,U+25CF,U+2713,U+2717" \
    --layout-features=kern,ccmp,mark,mkmk \
    --desubroutinize --no-hinting --drop-tables+=DSIG
'
```

The repertoire is deliberate: ASCII, Latin-1 for accented names such as Viénot
and Björn, `Δ` because it appears in `ΔE` on nearly every page, dashes and
quotes, arrows for diagrams, sub- and superscripts, the maths relations used in
the measured-bound tables, and `✓ ✗` for the contrast matrix.

## For anyone maintaining this repository

**These files are committed by hand and are not build output.** Nothing
regenerates them, `cargo xtask build` does not touch them, and they must never
be deleted as stale artifacts. That is the one category of binary asset in this
project which is vendored rather than generated.
