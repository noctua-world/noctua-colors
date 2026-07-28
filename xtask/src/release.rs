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
    tool: bool,
    dry_run: bool,
) -> Result<(), String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.parse::<u32>().is_err()) {
        return Err(format!(
            "`{version}` is not a version. Expected three numbers, such as 0.2.0"
        ));
    }

    let what = if tool {
        "the compiler"
    } else {
        "the colour system"
    };
    ui::heading(&format!("preparing {version} — {what}"));
    ui::gap();

    // Nothing ships that does not pass the gate.
    check::run(root, spec_path, false)?;

    let current = if tool {
        tool_version(root)?
    } else {
        system_version(root, spec_path)?
    };

    if current == version {
        return Err(format!("{what} is already at {version}"));
    }

    ui::gap();
    ui::heading("version");
    ui::ok(&format!("{what}: {current} -> {version}"));
    if tool {
        ui::detail("the colour system's version is unchanged; nothing publishes from this");
    } else {
        ui::detail("stamped on every artifact, both registries, and the tag");
    }

    if dry_run {
        ui::gap();
        ui::detail("nothing was changed; drop --dry-run to write the version");
        return Ok(());
    }

    if tool {
        bump_workspace(root, &current, version)?;
    } else {
        // The spec is the colour system, so its version is declared there and
        // everything generated reads it from the palette.
        bump_spec(spec_path, &current, version)?;
        // The npm package carries its own version, and it is the one artifact
        // whose manifest is hand-written. Left behind, the two registries would
        // disagree about what release they are — and `check` fails on exactly
        // that, so the mistake is loud rather than shipped.
        bump_package_json(root, &current, version)?;
    }

    // Still a subprocess, and the reason has narrowed rather than gone away.
    //
    // The colour system's version now travels on the palette, so it would be
    // correct in-process. The compiler's does not: it reaches MANIFEST.json
    // through `env!("CARGO_PKG_VERSION")`, resolved when *this binary* was
    // compiled, and Cargo cannot recompile a binary that is currently running.
    // A `--tool` release regenerated in-process would stamp the version it had
    // just replaced, and `check` would report the manifest out of sync.
    //
    // Rather than branch on which of the two is being bumped and get it subtly
    // wrong later, both take the subprocess. It costs a compile, which a
    // release can afford, and it means the regenerated tree is always the one
    // a fresh clone would produce.
    rebuild_with_the_new_version(root, spec_path)?;

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

/// Regenerates `system/` from a freshly compiled binary.
///
/// A subprocess rather than a call, for the reason above: the version is a
/// compile-time constant, so the artifacts can only carry the new one if the
/// code that writes them was compiled after the manifest changed.
fn rebuild_with_the_new_version(root: &Path, spec_path: &Path) -> Result<(), String> {
    let spec = spec_path
        .strip_prefix(root)
        .unwrap_or(spec_path)
        .to_string_lossy()
        .into_owned();

    let status = std::process::Command::new(env!("CARGO"))
        .args([
            "run",
            "--quiet",
            "--package",
            "xtask",
            "--",
            "--spec",
            &spec,
            "build",
            "--system",
        ])
        .current_dir(root)
        .status()
        .map_err(|error| format!("could not rebuild: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("the rebuild against the new version failed".to_owned())
    }
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
/// The compiler's version, from `[workspace.package]`.
fn tool_version(root: &Path) -> Result<String, String> {
    let manifest = std::fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|error| format!("could not read Cargo.toml: {error}"))?;
    Ok(manifest
        .split("[workspace.package]")
        .nth(1)
        .and_then(|section| section.lines().find(|line| line.starts_with("version = ")))
        .and_then(|line| line.split('"').nth(1))
        .ok_or("could not find the workspace version in Cargo.toml")?
        .to_owned())
}

/// The colour system's version, from the spec's `[system]` table.
///
/// Read through the parser rather than by scanning text: the spec is the
/// authority here and `[system]` has defaults, so a version that is absent
/// still has a value and text-scanning would report it as missing.
fn system_version(root: &Path, spec_path: &Path) -> Result<String, String> {
    let _ = root;
    let spec = noctua_spec::load(spec_path)
        .map_err(|error| format!("{:?}", miette::Report::new(error)))?;
    Ok(spec.system.version)
}

/// Writes the compiler's version into `[workspace.package]`.
fn bump_workspace(root: &Path, current: &str, version: &str) -> Result<(), String> {
    let path = root.join("Cargo.toml");
    let manifest = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read Cargo.toml: {error}"))?;
    let updated = manifest.replacen(
        &format!("version = \"{current}\""),
        &format!("version = \"{version}\""),
        1,
    );
    std::fs::write(&path, updated)
        .map_err(|error| format!("could not write Cargo.toml: {error}"))?;
    ui::ok(&format!("Cargo.toml {current} -> {version}"));
    Ok(())
}

/// Writes the colour system's version into the spec's `[system]` table.
///
/// Anchored on `[system]` rather than replacing the first `version = "…"` in
/// the file, because the spec is a long document and a bare replacement would
/// happily rewrite an unrelated line that happened to match.
fn bump_spec(spec_path: &Path, current: &str, version: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(spec_path)
        .map_err(|error| format!("could not read the spec: {error}"))?;

    let (before, after) = text
        .split_once("[system]")
        .ok_or("the spec has no [system] table; add one with a `version` line")?;

    let needle = format!("version = \"{current}\"");
    if !after.contains(&needle) {
        return Err(format!(
            "the spec's [system] table does not contain {needle}; check its formatting"
        ));
    }

    let updated = format!(
        "{before}[system]{}",
        after.replacen(&needle, &format!("version = \"{version}\""), 1)
    );
    std::fs::write(spec_path, updated)
        .map_err(|error| format!("could not write the spec: {error}"))?;
    ui::ok(&format!("specs/noctua.toml {current} -> {version}"));
    Ok(())
}

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
