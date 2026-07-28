# Contributing

[← back to the README](README.md)

If you want to *use* the colours, you do not need this file — the
[README](README.md) covers every installation route. This is for working on the
compiler.

---

## Before you clone

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
project; this repository's [`AGENTS.md`](AGENTS.md) is its own operating manual
and is the file to read before changing anything here.

---

## Quick start

Requires only a Rust toolchain. There is nothing to install first — the
`cargo xtask` alias ships in `.cargo/config.toml`, and the WebAssembly target
the playground needs is declared in `rust-toolchain.toml` and installed by
`xtask` itself.

```sh
git clone https://github.com/noctua-world/noctua-colors
cd noctua-colors
cargo xtask check
```

That validates the spec, runs every quality gate, verifies `system/` matches,
and runs formatting, lints and tests. It is the same command CI runs — not a
reimplementation of it in YAML — so "passes locally, fails in CI" is not a thing
that happens here.

---

## The loop

Change a hue in `specs/noctua.toml`, then:

```sh
cargo xtask build     # writes target/system/ — a gitignored scratch tree
cargo xtask dev       # ...and serves it, with live reload
```

**An ordinary `build` cannot touch the published colour system.** It writes
`target/system/`, so trying something out never dirties the repository and can
never be committed by accident. When you actually mean it:

```sh
cargo xtask build --system
```

The guard is symmetric: `cargo xtask check` verifies `system/` against the spec,
so the opposite mistake — meaning a change and forgetting to publish it — fails
locally and in CI. Neither direction is left to memory.

---

## The rules

These are not style preferences. Breaking one is a defect.

1. **No hardcoded colour values outside the spec** — not in the engine, tests,
   documentation site or examples. There is a source gate that fails the build
   on a hex literal. If you genuinely need one (a fixture, a published anchor
   value), it takes an `// allow-literal: <reason>` marker on the same line, and
   the reason has to be a real one.
2. **`noctua-core` has no workspace dependencies** and never will. It knows
   nothing of the spec format or the output targets.
3. **`noctua-emit` performs no colour math.** It formats colours that arrive
   resolved and quantized. An emitter that computes one makes the gates and the
   output disagree about what shipped.
4. **Generated files are never edited by hand.** Change the spec and rebuild.
5. **The command surface is six verbs.** New capability goes behind an existing
   one or it does not ship.
6. **Everything is in English** — code, comments, documentation, commit
   messages — regardless of the language a discussion happened in. User-facing
   strings are i18n resources and need an English and a pt-BR pair, which the
   site's `t(locale, en, pt)` helper enforces at compile time.

[`AGENTS.md`](AGENTS.md) has the full list, the repository map, and a section of
gotchas that have each already cost real debugging time.

---

## Tests

```sh
cargo xtask check                 # everything
cargo xtask check --colors-only   # skip fmt, lints and tests, for tuning colours
cargo test --workspace            # just the tests
```

Property tests run 2048 cases each. To reproduce a specific failure, proptest
writes the input to `crates/noctua-core/tests/*.proptest-regressions` — commit
that file so the case is replayed forever.

A change to the colours will move `tests/golden/palette.txt`. That is expected;
what is not expected is moving it *without meaning to*, which is why it is
committed.

---

## Releasing

```sh
cargo xtask release 0.3.0 --dry-run   # see what would happen
cargo xtask release 0.3.0             # write it
```

This bumps the **colour system's** version — the one in `specs/noctua.toml`'s
`[system]` table, which is what a tag means and what both registries publish.
`--tool 0.3.0` bumps the compiler's version instead, which publishes nothing.

`release` deliberately does **not** commit, tag, push or publish. It verifies,
writes the version everywhere it appears, checks the changelog has an entry, and
prints the three commands a human then runs deliberately.

A changelog entry is the one part a tool cannot write. For a colour system,
*which colour changed and does my interface still meet contrast* is the only
question a consumer actually has, and commit-derived release notes do not answer
it.

---

## Licence

By contributing you agree that your contributions are licensed under MIT OR
Apache-2.0, matching the project.
