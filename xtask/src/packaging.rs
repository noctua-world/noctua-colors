//! The npm package's contract, checked without Node.
//!
//! `package.json` is the one manifest in this repository that is hand-written
//! rather than generated, and it points at generated files. So it is the one
//! place where a rename inside `dist/` can break a consumer while every other
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
pub(crate) fn check(root: &Path, workspace_version: &str) -> Result<Vec<String>, String> {
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
    version_tracks_workspace(&manifest, workspace_version, &mut failures, &mut checked);
    nothing_is_depended_on(&manifest, &mut failures);

    // 6. The trap this check exists for. `dist/tailwind/theme.css` opens with a
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

/// The version has to track the workspace, or the npm package and the crate
/// disagree about what release they are.
fn version_tracks_workspace(
    manifest: &Value,
    workspace_version: &str,
    failures: &mut Vec<String>,
    checked: &mut usize,
) {
    match manifest.get("version").and_then(Value::as_str) {
        Some(version) if version == workspace_version => *checked += 1,
        Some(version) => failures.push(format!(
            "package.json is at {version} and the workspace is at {workspace_version}"
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

/// Resolves every relative `@import` in the generated CSS against the
/// filesystem, the way a bundler inside the tarball will.
fn relative_imports_resolve(root: &Path) -> Result<Vec<String>, String> {
    let mut failures = Vec::new();
    let dist = root.join("dist");

    for relative in ["tailwind/theme.css", "css/index.css"] {
        let file = dist.join(relative);
        if !file.exists() {
            failures.push(format!("dist/{relative} is missing"));
            continue;
        }
        let text = std::fs::read_to_string(&file)
            .map_err(|error| format!("could not read dist/{relative}: {error}"))?;
        let parent = file.parent().unwrap_or(&dist);

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
                    "dist/{relative} imports {target}, which does not resolve. This \
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

    #[test]
    fn the_shipped_package_is_coherent() {
        let failures = check(&repository_root(), &workspace_version()).expect("readable");
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
                "exports":{"./css":"./dist/css/nope.css"},"files":[]}"#,
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

    /// Every place the release version lands has to agree, and there are five.
    ///
    /// `cargo xtask release` writes two of them directly and regenerates the
    /// other three — and the regeneration has to happen in a *freshly compiled*
    /// subprocess, because the version reaches `dist/` through
    /// `env!("CARGO_PKG_VERSION")`, which is baked when the binary was built.
    /// Doing it in-process stamped the artifacts with the version that had just
    /// been replaced, and `check` reported three files out of sync. This is the
    /// test that would have caught it.
    #[test]
    fn every_artifact_carries_the_same_version() {
        let root = repository_root();
        let workspace = workspace_version();

        let package_json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("package.json")).expect("read"),
        )
        .expect("valid JSON");
        assert_eq!(
            package_json["version"].as_str().expect("a version"),
            workspace,
            "package.json disagrees with the workspace"
        );

        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("dist/MANIFEST.json")).expect("read"),
        )
        .expect("valid JSON");
        assert_eq!(
            manifest["version"].as_str().expect("a version"),
            workspace,
            "dist/MANIFEST.json disagrees — regenerate with `cargo xtask build`"
        );

        for file in ["dist/rust/Cargo.toml", "dist/rust/README.md"] {
            let text = std::fs::read_to_string(root.join(file)).expect("read");
            assert!(
                text.contains(&format!("version = \"{workspace}\"")),
                "{file} does not carry {workspace}"
            );
        }
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
        std::fs::create_dir_all(dir.join("dist/tailwind")).expect("mkdir");
        std::fs::create_dir_all(dir.join("dist/css")).expect("mkdir");
        std::fs::write(dir.join("dist/css/index.css"), "/* the sibling */\n").expect("write");
        // Points one directory further up than the real file does, which is
        // exactly what a move would produce.
        std::fs::write(
            dir.join("dist/tailwind/theme.css"),
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
