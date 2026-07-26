//! `cargo xtask build` — compile the spec into every target.

use std::path::Path;

use noctua_engine::Palette;

use crate::{site, ui};

/// Loads the spec and builds the palette, touching nothing on disk.
///
/// Separate from [`write`] for one important reason: `check` must **not**
/// rewrite `dist/` before inspecting it. When these were one function, running
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

/// Builds the palette and writes `dist/`.
///
/// # Errors
///
/// Any spec problem, unreachable colour target, or filesystem failure.
pub(crate) fn run(root: &Path, spec_path: &Path) -> Result<Palette, String> {
    let relative = relative_to(root, spec_path);
    let palette = palette(root, spec_path)?;

    let spec_text = std::fs::read_to_string(spec_path)
        .map_err(|error| format!("could not read {relative}: {error}"))?;

    let dist = root.join("dist");
    let written = noctua_emit::dist::write(&dist, &palette, &relative, &spec_text)
        .map_err(|error| format!("could not write dist/: {error}"))?;

    ui::ok(&format!("wrote {} files to dist/", written.len()));
    Ok(palette)
}

/// The spec path as written in generated headers.
///
/// Always repository-relative with forward slashes, so `dist/` is identical
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
pub(crate) fn all(root: &Path, spec_path: &Path, with_site: bool) -> Result<Palette, String> {
    let palette = run(root, spec_path)?;
    if with_site {
        let written = site::build(root)?;
        site::report(root, written);
    }
    Ok(palette)
}
