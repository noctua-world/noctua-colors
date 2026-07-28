//! `cargo xtask release` — cut a version.
//!
//! Deliberately conservative. It verifies, writes the version everywhere it
//! appears, checks that the changelog has an entry for it, and stops. It does
//! **not** commit, tag, push or publish.
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

    // The npm package carries its own version, and it is the one artifact whose
    // manifest is hand-written. Left behind, the two registries would disagree
    // about what release they are — and `check` fails on exactly that, so the
    // mistake is loud rather than shipped.
    bump_package_json(root, &current, version)?;

    // The version lives in dist/MANIFEST.json and in the generated crate's
    // manifest too, so the artifacts have to be rebuilt or `check` would
    // immediately report them out of sync.
    crate::build::run(root, spec_path)?;

    // A changelog entry is the one part of a release a tool cannot write. For a
    // colour system, "which colour changed" is the only question a consumer
    // actually has, and commit-derived release notes do not answer it.
    if let Some(warning) = changelog_gap(root, version)? {
        ui::gap();
        ui::warn(&warning);
    }

    next_steps(version);
    Ok(())
}

/// The three commands a human runs to turn a prepared version into a release.
///
/// Printed rather than executed: every commit on this machine needs a physical
/// hardware-key touch, so a tool that committed would hang waiting for a finger
/// that may not be there — and a release ought to be reviewable before it
/// becomes a fact.
fn next_steps(version: &str) {
    ui::gap();
    ui::heading("next steps");
    ui::detail("these are left to a human, because every commit here needs a key touch:");
    ui::detail(&format!("    git commit -am \"release: {version}\""));
    ui::detail(&format!(
        "    git tag -a v{version} -m \"noctua-colors {version}\""
    ));
    ui::detail("    git push --follow-tags");
    ui::gap();
    ui::detail("pushing the tag publishes to npm, crates.io and GitHub Releases.");
    ui::detail("see AGENTS.md (\"Publishing\") for what each workflow does.");
}

/// Moves `package.json`'s version, matching only the top-level `"version"` key.
///
/// A blind string replacement would also rewrite a version that appeared in a
/// dependency range or a URL. There are none today, which is exactly why a
/// future one would go unnoticed.
fn bump_package_json(root: &Path, current: &str, version: &str) -> Result<(), String> {
    let path = root.join("package.json");
    if !path.exists() {
        return Ok(());
    }

    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read package.json: {error}"))?;
    let needle = format!("\"version\": \"{current}\"");
    if !text.contains(&needle) {
        return Err(format!(
            "package.json does not contain {needle}. It is hand-written, so the \
             version line has to be findable; check its formatting."
        ));
    }

    let updated = text.replacen(&needle, &format!("\"version\": \"{version}\""), 1);
    std::fs::write(&path, updated)
        .map_err(|error| format!("could not write package.json: {error}"))?;
    ui::ok(&format!("package.json {current} -> {version}"));
    Ok(())
}

/// Reports whether the changelog is missing an entry for this version.
///
/// A warning rather than a failure: a release with an undocumented change is a
/// bad release, but a tool that refuses to proceed over prose is a tool people
/// route around.
fn changelog_gap(root: &Path, version: &str) -> Result<Option<String>, String> {
    let path = root.join("CHANGELOG.md");
    if !path.exists() {
        return Ok(Some(
            "there is no CHANGELOG.md. A consumer of a colour system wants to know \
             which colour changed, and no commit log answers that."
                .to_owned(),
        ));
    }

    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read CHANGELOG.md: {error}"))?;
    if text.contains(&format!("[{version}]")) || text.contains(&format!("## {version}")) {
        ui::ok(&format!("CHANGELOG.md has an entry for {version}"));
        return Ok(None);
    }

    Ok(Some(format!(
        "CHANGELOG.md has no entry for {version}. Add one before tagging — the tag \
         is what publishes."
    )))
}
