//! `cargo xtask build` — compile the spec into every target.

use std::path::Path;

use noctua_engine::Palette;

use crate::{site, ui};

/// Loads the spec and builds the palette, touching nothing on disk.
///
/// Separate from [`write`] for one important reason: `check` must **not**
/// rewrite `system/` before inspecting it. When these were one function, running
/// `check` silently regenerated the artifacts and then compared them against
/// themselves, so the hand-edit guard could never fire — the check reported
/// "in sync" immediately after a file had been edited by hand.
///
/// # Errors
///
/// Any spec problem or unreachable colour target.
pub(crate) fn palette(root: &Path, spec_path: &Path) -> Result<Palette, String> {
    let relative = relative_to(root, spec_path);

    let spec = noctua_spec::load(spec_path).map_err(|error| {
        // The spec layer's diagnostics point at the exact byte range and end
        // with a fix; rendering them any other way would throw that away.
        format!("{:?}", miette::Report::new(error))
    })?;

    let palette =
        noctua_engine::build(&spec).map_err(|error| format!("{error}\n\n  {}", error.fix()))?;

    ui::ok(&format!("compiled {relative}"));
    ui::detail(&format!(
        "{} themes, {} families, {} gamuts, {} neutral steps \u{d7} {} ramps, {} scales",
        palette.themes.len(),
        palette.themes[0].modes[0].families.len(),
        palette.gamuts.len(),
        palette.neutral_ramp().len(),
        palette.neutral_ramps.len(),
        palette.themes[0].modes[0].scales.len()
    ));
    Ok(palette)
}

/// Where the **published** colour system lives.
///
/// Committed, and public API: consumers reach it by CDN URL, by Nix `src`, by
/// submodule and by plain copy. Only `build --system` and `release` write here.
pub(crate) const SYSTEM: &str = "system";

/// Where an ordinary build writes.
///
/// Inside `target/`, so it is already gitignored, and so the everyday loop —
/// edit the spec, build, look at the site — **cannot dirty the published
/// system**. That separation is the point: before it existed, testing a hue
/// rewrote the shipped colours, and the only thing between that and a commit
/// was noticing a 250-file diff.
pub(crate) const SCRATCH: &str = "target/system";

/// The directory a build writes to.
#[must_use]
pub(crate) fn destination(root: &Path, publish: bool) -> std::path::PathBuf {
    root.join(if publish { SYSTEM } else { SCRATCH })
}

/// Builds the palette and writes it.
///
/// `publish` chooses between the committed `system/` and the scratch tree. It is
/// deliberately a parameter with no default here — the default belongs to the
/// CLI, where a reader can see it.
///
/// # Errors
///
/// Any spec problem, unreachable colour target, or filesystem failure.
pub(crate) fn run(root: &Path, spec_path: &Path, publish: bool) -> Result<Palette, String> {
    let relative = relative_to(root, spec_path);
    let palette = palette(root, spec_path)?;

    let spec_text = std::fs::read_to_string(spec_path)
        .map_err(|error| format!("could not read {relative}: {error}"))?;

    let destination = destination(root, publish);
    let label = if publish { SYSTEM } else { SCRATCH };
    let written = noctua_emit::output::write(&destination, &palette, &relative, &spec_text)
        .map_err(|error| format!("could not write {label}/: {error}"))?;

    ui::ok(&format!("wrote {} files to {label}/", written.len()));
    if !publish {
        ui::detail("a scratch build; `--system` writes the published colour system");
    }
    Ok(palette)
}

/// The spec path as written in generated headers.
///
/// Always repository-relative with forward slashes, so `system/` is identical
/// no matter where the build ran from or on which platform.
pub(crate) fn relative_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Builds the artifacts and, optionally, the site.
///
/// # Errors
///
/// Any spec, colour or filesystem problem.
pub(crate) fn all(
    root: &Path,
    spec_path: &Path,
    with_site: bool,
    publish: bool,
) -> Result<Palette, String> {
    let palette = run(root, spec_path, publish)?;
    if with_site {
        // The site renders from whatever this build just wrote, so a scratch
        // build previews the scratch colours. That is the whole reason the loop
        // is safe: you can look at a change before deciding to publish it.
        let written = site::build(root, &destination(root, publish))?;
        site::report(root, written);
    }
    Ok(palette)
}
