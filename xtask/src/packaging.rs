//! The npm package's contract, checked without Node.
//!
//! `package.json` is the one manifest in this repository that is hand-written
//! rather than generated, and it points at generated files. So it is the one
//! place where a rename inside `system/` can break a consumer while every other
//! check stays green: `exports` is a whitelist, and a path in it that no longer
//! exists fails at the consumer's `@import`, in their project, weeks later.
//!
//! This runs in Rust on purpose. `npm pack --dry-run` is the authoritative
//! check and CI runs it, but it needs Node, and `cargo xtask check` is what a
//! developer runs on a machine that has only rustup. A gate that only fires in
//! CI is a gate that tells you after you pushed.

use std::path::Path;

use serde_json::Value;

use crate::ui;

/// Verifies `package.json` against what is actually on disk.
///
/// Returns one string per failure; an empty vector means the package is
/// coherent.
///
/// # Errors
///
/// Only when `package.json` cannot be read or is not valid JSON. A broken
/// *claim* is a returned failure, not an error, so every claim gets checked in
/// one pass rather than stopping at the first.
pub(crate) fn check(root: &Path, system_version: &str) -> Result<Vec<String>, String> {
    let path = root.join("package.json");
    if !path.exists() {
        // Not every repository in this fleet publishes to npm. Absence is a
        // valid state, not a failure.
        return Ok(Vec::new());
    }

    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read package.json: {error}"))?;
    let manifest: Value = serde_json::from_str(&text)
        .map_err(|error| format!("package.json is not valid JSON: {error}"))?;

    let mut failures = Vec::new();
    let mut checked = 0usize;

    exports_resolve(root, &manifest, &mut failures, &mut checked);
    files_exist(root, &manifest, &mut failures, &mut checked);
    side_effects_cover_css(&manifest, &mut failures, &mut checked);
    version_tracks_the_colour_system(&manifest, system_version, &mut failures, &mut checked);
    nothing_is_depended_on(&manifest, &mut failures);

    // 6. The trap this check exists for. `system/tailwind/theme.css` opens with a
    //    *relative* `@import "../css/index.css"`, which never touches the
    //    `exports` map — it is resolved on the filesystem, inside the tarball.
    //    So moving either file breaks Tailwind consumers while passing every
    //    other check, in-repo and in `exports`.
    failures.extend(relative_imports_resolve(root)?);
    checked += 1;

    if failures.is_empty() {
        ui::ok(&format!(
            "package.json is coherent ({checked} claims checked)"
        ));
    } else {
        for failure in &failures {
            ui::failure(failure);
        }
    }

    Ok(failures)
}

/// Every `exports` target must exist. This is the whitelist that seals the
/// package: a path not listed is unreachable, and a path listed but missing is a
/// runtime error in someone else's build.
fn exports_resolve(root: &Path, manifest: &Value, failures: &mut Vec<String>, checked: &mut usize) {
    let Some(exports) = manifest.get("exports").and_then(Value::as_object) else {
        failures.push("package.json has no exports map".to_owned());
        return;
    };

    for (subpath, target) in exports {
        let Some(target) = target.as_str() else {
            failures.push(format!(
                "exports[\"{subpath}\"] is not a string; only plain-string subpaths \
                 are used here, because a condition object introduces a \
                 resolver-ordering question this package does not need"
            ));
            continue;
        };
        *checked += 1;
        // A star pattern names a directory rather than a file, so the directory
        // is what has to exist.
        let probe = match target.split_once('*') {
            Some((prefix, _)) => prefix.trim_end_matches('/').to_owned(),
            None => target.to_owned(),
        };
        if !root.join(probe.trim_start_matches("./")).exists() {
            failures.push(format!(
                "exports[\"{subpath}\"] points at {target}, which does not exist"
            ));
        }
    }
}

/// Every `files` entry must exist, or the tarball silently ships less than it
/// claims.
fn files_exist(root: &Path, manifest: &Value, failures: &mut Vec<String>, checked: &mut usize) {
    let Some(files) = manifest.get("files").and_then(Value::as_array) else {
        failures.push(
            "package.json has no files allowlist; without one npm ships the whole \
             repository, which here is 79 MB"
                .to_owned(),
        );
        return;
    };

    for entry in files {
        let Some(entry) = entry.as_str() else {
            continue;
        };
        *checked += 1;
        if !root.join(entry).exists() {
            failures.push(format!("files[] lists {entry}, which does not exist"));
        }
    }
}

/// `sideEffects: false` on a package whose entire purpose is CSS makes bundlers
/// drop the CSS **with no error**, and the consumer sees an unstyled
/// application. The array form is the only correct answer.
fn side_effects_cover_css(manifest: &Value, failures: &mut Vec<String>, checked: &mut usize) {
    match manifest.get("sideEffects") {
        Some(Value::Array(patterns)) => {
            *checked += 1;
            if !patterns
                .iter()
                .any(|p| p.as_str().is_some_and(|p| p.contains(".css")))
            {
                failures.push(
                    "sideEffects does not cover .css; a bundler will tree-shake the \
                     stylesheets away"
                        .to_owned(),
                );
            }
        }
        Some(Value::Bool(false)) => failures.push(
            "sideEffects is false. This package ships CSS, so that tells bundlers \
             they may delete it — silently. Use [\"**/*.css\", \"**/*.scss\"]"
                .to_owned(),
        ),
        _ => failures.push("sideEffects is missing; it must be an array covering CSS".to_owned()),
    }
}

/// The npm version has to track **the colour system's**, not the compiler's.
///
/// `package.json` is the one artifact whose version is hand-written, and it and
/// the generated crate are published from the same tag. If they disagree, one
/// registry ships a release the other has never heard of — and npm publishes
/// are permanent, so this has to fail before the tag exists rather than after.
///
/// It used to compare against the workspace version. That was right when there
/// was one number; now the workspace holds the compiler's, and comparing
/// against it would demand that the npm package be versioned by a number no
/// consumer of it can see.
fn version_tracks_the_colour_system(
    manifest: &Value,
    system_version: &str,
    failures: &mut Vec<String>,
    checked: &mut usize,
) {
    match manifest.get("version").and_then(Value::as_str) {
        Some(version) if version == system_version => *checked += 1,
        Some(version) => failures.push(format!(
            "package.json is at {version} and the colour system is at {system_version}. \
             The spec's [system] table is the authority; `cargo xtask release` writes both"
        )),
        None => failures.push("package.json has no version".to_owned()),
    }
}

/// Nothing here may grow a dependency or an install script. npm 12 blocks
/// lifecycle scripts by default, so a `postinstall` would be broken-by-default
/// for current consumers — and a data package needs neither.
fn nothing_is_depended_on(manifest: &Value, failures: &mut Vec<String>) {
    for forbidden in ["dependencies", "peerDependencies", "scripts"] {
        if manifest
            .get(forbidden)
            .and_then(Value::as_object)
            .is_some_and(|object| !object.is_empty())
        {
            failures.push(format!(
                "package.json declares {forbidden}; this package is data and must have none"
            ));
        }
    }
}

/// Every `.css` file under `system/`, in a stable order.
///
/// Collected rather than listed. A hardcoded list was right when two files had
/// imports; at forty-two it is a blind spot that grows every time a palette is
/// added, and the thing it would miss — an import that resolves in-repo but not
/// in the tarball — is exactly what this check exists to catch.
fn stylesheets(directory: &Path, into: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    // A directory that is not there is a finding, not a crash: the caller
    // reports "ships no stylesheets at all", which is the useful sentence.
    // Returning Err here instead would abort the whole packaging check on a
    // tree that simply has not been built yet.
    if !directory.is_dir() {
        return Ok(());
    }
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("could not read {}: {error}", directory.display()))?;
    let mut found: Vec<std::path::PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| format!("could not read an entry: {error}"))?;
        found.push(entry.path());
    }
    found.sort();

    for path in found {
        if path.is_dir() {
            stylesheets(&path, into)?;
        } else if path.extension().is_some_and(|e| e == "css") {
            into.push(path);
        }
    }
    Ok(())
}

/// Resolves every relative `@import` in the generated CSS against the
/// filesystem, the way a bundler inside the tarball will.
fn relative_imports_resolve(root: &Path) -> Result<Vec<String>, String> {
    let mut failures = Vec::new();
    let system = root.join("system");

    let mut files = Vec::new();
    for directory in ["css", "tailwind"] {
        stylesheets(&system.join(directory), &mut files)?;
    }
    if files.is_empty() {
        failures.push("system/ ships no stylesheets at all".to_owned());
    }

    for file in files {
        let relative = file
            .strip_prefix(&system)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&file)
            .map_err(|error| format!("could not read system/{relative}: {error}"))?;
        let parent = file.parent().unwrap_or(&system);

        for line in text.lines() {
            let Some(rest) = line.trim().strip_prefix("@import") else {
                continue;
            };
            let Some(target) = rest.split('"').nth(1) else {
                continue;
            };
            if !target.starts_with('.') {
                continue;
            }
            if !parent.join(target).exists() {
                failures.push(format!(
                    "system/{relative} imports {target}, which does not resolve. This \
                     breaks inside the published tarball while still working in-repo"
                ));
            }
        }
    }

    Ok(failures)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository_root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask has a parent")
            .to_path_buf()
    }

    fn workspace_version() -> String {
        let manifest =
            std::fs::read_to_string(repository_root().join("Cargo.toml")).expect("read Cargo.toml");
        manifest
            .split("[workspace.package]")
            .nth(1)
            .and_then(|section| section.lines().find(|l| l.starts_with("version = ")))
            .and_then(|line| line.split('"').nth(1))
            .expect("a workspace version")
            .to_owned()
    }

    /// The version passed here is the **colour system's**, because that is what
    /// `check` compares `package.json` against and what the tag publishes.
    /// Passing the compiler's made this fail the moment the two diverged, which
    /// is exactly what it should have done.
    #[test]
    fn the_shipped_package_is_coherent() {
        let failures = check(&repository_root(), &system_version()).expect("readable");
        assert!(failures.is_empty(), "{failures:#?}");
    }

    /// The whole reason this module is not a `npm pack` invocation: it has to
    /// fire on a machine with no Node, and it has to fire for the right reason.
    #[test]
    fn a_missing_export_target_is_caught() {
        let dir = tempdir("missing-export");
        std::fs::write(
            dir.join("package.json"),
            r#"{"version":"0.0.0","sideEffects":["**/*.css"],
                "exports":{"./css":"./system/css/nope.css"},"files":[]}"#,
        )
        .expect("write");

        let failures = check(&dir, "0.0.0").expect("readable");
        assert!(
            failures.iter().any(|f| f.contains("nope.css")),
            "{failures:#?}"
        );
    }

    /// `sideEffects: false` is the highest-consequence mistake available in this
    /// file: it is silent, and the symptom is an unstyled application in
    /// somebody else's project.
    #[test]
    fn side_effects_false_is_rejected() {
        let dir = tempdir("side-effects");
        std::fs::write(
            dir.join("package.json"),
            r#"{"version":"0.0.0","sideEffects":false,"exports":{},"files":[]}"#,
        )
        .expect("write");

        let failures = check(&dir, "0.0.0").expect("readable");
        assert!(
            failures.iter().any(|f| f.contains("tree-shake")
                || f.contains("silently")
                || f.contains("delete it")),
            "{failures:#?}"
        );
    }

    #[test]
    fn a_version_that_drifted_from_the_workspace_is_caught() {
        let dir = tempdir("version-drift");
        std::fs::write(
            dir.join("package.json"),
            r#"{"version":"9.9.9","sideEffects":["**/*.css"],"exports":{},"files":[]}"#,
        )
        .expect("write");

        let failures = check(&dir, "0.1.0").expect("readable");
        assert!(
            failures.iter().any(|f| f.contains("9.9.9")),
            "{failures:#?}"
        );
    }

    /// The colour system's version, from the spec — the authority.
    fn system_version() -> String {
        let spec = noctua_spec::load(repository_root().join("specs/noctua.toml"))
            .expect("the shipped spec");
        spec.system.version
    }

    /// Every artifact a consumer sees must carry **the colour system's**
    /// version, and there are four places it lands.
    ///
    /// `cargo xtask release` writes two directly — the spec and `package.json`
    /// — and regenerates the other two. The regeneration happens in a *freshly
    /// compiled* subprocess; see the comment on `rebuild_with_the_new_version`
    /// for why that is still necessary now that the system version travels on
    /// the palette.
    ///
    /// This test caught a real bug: an in-process regeneration stamped the
    /// artifacts with the version that had just been replaced.
    #[test]
    fn every_artifact_carries_the_same_version() {
        let root = repository_root();
        let system = system_version();

        let package_json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("package.json")).expect("read"),
        )
        .expect("valid JSON");
        assert_eq!(
            package_json["version"].as_str().expect("a version"),
            system,
            "package.json disagrees with the spec's [system] version"
        );

        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("system/MANIFEST.json")).expect("read"),
        )
        .expect("valid JSON");
        assert_eq!(
            manifest["systemVersion"].as_str().expect("a version"),
            system,
            "system/MANIFEST.json disagrees — regenerate with `cargo xtask build --system`"
        );

        for file in ["system/rust/Cargo.toml", "system/rust/README.md"] {
            let text = std::fs::read_to_string(root.join(file)).expect("read");
            assert!(
                text.contains(&format!("version = \"{system}\"")),
                "{file} does not carry {system}"
            );
        }
    }

    /// The two versions are genuinely separate, and the manifest records both.
    ///
    /// Without this, the split could silently collapse back into one number —
    /// a later refactor reintroducing `env!("CARGO_PKG_VERSION")` on the
    /// publishing path would pass every other test here, because the two are
    /// allowed to be *equal*; what they are not allowed to be is the same
    /// field.
    #[test]
    fn the_manifest_records_the_compiler_version_separately() {
        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(repository_root().join("system/MANIFEST.json")).expect("read"),
        )
        .expect("valid JSON");

        assert_eq!(
            manifest["version"].as_str().expect("a version"),
            workspace_version(),
            "MANIFEST.json's `version` must stay the compiler's — TOKEN-POLICY.md \
             tells consumers to diff this file"
        );
        assert!(
            manifest["systemVersion"].is_string(),
            "the colour system's version is missing from the manifest"
        );
    }

    #[test]
    fn a_repository_without_a_package_json_is_not_a_failure() {
        let dir = tempdir("no-package-json");
        assert!(check(&dir, "0.1.0").expect("readable").is_empty());
    }

    /// The trap this whole module exists for, and the only one that is
    /// invisible from inside the repository: `theme.css` reaches its sibling by
    /// a *relative* path, which bypasses `exports` entirely and is resolved on
    /// the filesystem — inside the consumer's tarball. Move either file and
    /// Tailwind consumers break while every other check stays green.
    #[test]
    fn a_relative_import_that_does_not_resolve_is_caught() {
        let dir = tempdir("relative-import");
        std::fs::create_dir_all(dir.join("system/tailwind")).expect("mkdir");
        std::fs::create_dir_all(dir.join("system/css")).expect("mkdir");
        std::fs::write(dir.join("system/css/index.css"), "/* the sibling */\n").expect("write");
        // Points one directory further up than the real file does, which is
        // exactly what a move would produce.
        std::fs::write(
            dir.join("system/tailwind/theme.css"),
            "@import \"../../css/index.css\";\n",
        )
        .expect("write");
        std::fs::write(
            dir.join("package.json"),
            r#"{"version":"0.0.0","sideEffects":["**/*.css"],"exports":{},"files":[]}"#,
        )
        .expect("write");

        let failures = check(&dir, "0.0.0").expect("readable");
        assert!(
            failures
                .iter()
                .any(|f| f.contains("does not resolve") && f.contains("tarball")),
            "{failures:#?}"
        );
    }

    /// Scoped to this test module rather than a dependency: three lines against
    /// a crate, for a directory that is deleted by the OS anyway.
    fn tempdir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("noctua-colors-packaging-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
