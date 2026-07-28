# Convenience aliases. Every one of these is a `cargo xtask` verb underneath —
# `just` is never the only path, so a fresh clone with nothing but rustup can
# still do everything.

# List the available commands.
default:
    @just --list

# Compile the spec into every target, under the scratch tree target/system/.
build:
    cargo xtask build

# The same, but writing the published colour system in system/.
system:
    cargo xtask build --system

# Validate the spec, run every gate, and verify system/ is in sync.
check:
    cargo xtask check

# The same, skipping formatting, lints and tests. For tuning colors.
colors:
    cargo xtask check --colors-only

# Watch the spec and the site, rebuild, and serve with live reload.
dev port="8080":
    cargo xtask dev --port {{port}}

# Copy system/ into every consumer registered in the spec.
export:
    cargo xtask export

# Fit an existing palette back to spec parameters.
import source:
    cargo xtask import {{source}}

# Prepare a release. Writes the version; committing is left to a human.
release version:
    cargo xtask release {{version}}
