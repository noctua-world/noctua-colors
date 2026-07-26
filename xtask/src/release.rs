//! `cargo xtask release` — cut a version.
//!
//! Deliberately conservative. It verifies, writes the version and the
//! changelog entry, and stops. It does **not** commit, tag, push or publish.
//!
//! That is not timidity, it is this machine: every commit here requires a
//! physical hardware-key touch, so a tool that tried to commit would hang
//! waiting for a finger that may not be there. It also means a release is
//! reviewable before it becomes a fact — the printed next steps are three
//! commands a human runs deliberately.

use std::path::Path;

use crate::{check, ui};

/// Prepares a release.
///
/// # Errors
///
/// A malformed version, a failing check, or a filesystem problem.
pub(crate) fn run(
    root: &Path,
    spec_path: &Path,
    version: &str,
    dry_run: bool,
) -> Result<(), String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.parse::<u32>().is_err()) {
        return Err(format!(
            "`{version}` is not a version. Expected three numbers, such as 0.2.0"
        ));
    }

    ui::heading(&format!("preparing {version}"));
    ui::gap();

    // Nothing ships that does not pass the gate.
    check::run(root, spec_path, false)?;

    let manifest_path = root.join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .map_err(|error| format!("could not read Cargo.toml: {error}"))?;

    let current = manifest
        .split("[workspace.package]")
        .nth(1)
        .and_then(|section| section.lines().find(|line| line.starts_with("version = ")))
        .and_then(|line| line.split('"').nth(1))
        .ok_or("could not find the workspace version in Cargo.toml")?
        .to_owned();

    if current == version {
        return Err(format!("the workspace is already at {version}"));
    }

    ui::gap();
    ui::heading("version");
    ui::ok(&format!("{current} -> {version}"));

    if dry_run {
        ui::gap();
        ui::detail("nothing was changed; drop --dry-run to write the version");
        return Ok(());
    }

    let updated = manifest.replacen(
        &format!("version = \"{current}\""),
        &format!("version = \"{version}\""),
        1,
    );
    std::fs::write(&manifest_path, updated)
        .map_err(|error| format!("could not write Cargo.toml: {error}"))?;

    // The version lives in dist/MANIFEST.json too, so the artifacts have to
    // be rebuilt or `check` would immediately report them out of sync.
    crate::build::run(root, spec_path)?;

    ui::gap();
    ui::heading("next steps");
    ui::detail("these are left to a human, because every commit here needs a key touch:");
    ui::detail(&format!("    git commit -am \"release: {version}\""));
    ui::detail(&format!(
        "    git tag -a v{version} -m \"noctua-colors {version}\""
    ));
    ui::detail("    git push --follow-tags   # once a remote exists");
    Ok(())
}
