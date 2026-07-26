# Examples

Two consumers, proving the export path end to end. Both are **outside the
workspace**: a sibling project is not a member of this one, and an example that
built only because the workspace resolved something for it would prove nothing.

## `consumer-rust`

One path dependency on `dist/rust`, then `use`. No build script, no macro, no
runtime — every color is a `const`.

```bash
cd examples/consumer-rust && cargo run
```

It is run by `cargo xtask check`, not merely compiled: it asserts the neutral
ramp is monotone in lightness, so running it tests the generated output rather
than only the manifest.

## `consumer-web`

One `<link>` to `dist/css/balanced.css`. Open `index.html` in a browser — there
is nothing to install and nothing to build.

```bash
xdg-open examples/consumer-web/index.html
```

It defines no colors of its own. Every value in it is a token, which is why the
mode toggle restyles the whole page and why the `references` gate can verify
that each of the 24 tokens it names is one the compiler actually ships.

Both are checked by `cargo xtask check`. If a rename in an emitter breaks a
consumer, it breaks here — in this repository, on the next run — rather than in
a sibling project three weeks later.
