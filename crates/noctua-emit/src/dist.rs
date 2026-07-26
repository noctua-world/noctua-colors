//! Writing `dist/`, and verifying it is still what the spec says.
//!
//! Generated artifacts are **committed**. That is a deliberate choice: it
//! makes every consumption path — git submodule, subtree, sparse checkout,
//! plain file copy, a raw URL — work with no build step on the consumer's
//! side. The cost is that a committed generated file can be edited by hand,
//! and a hand-edit that survives is a lie that outlives whoever made it.
//!
//! So the sync check is not a nicety. It regenerates everything in memory and
//! compares byte for byte, and it reports three separate failures — changed,
//! missing, and stale — because the fix differs for each.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

use noctua_engine::Palette;

use crate::{EmittedFile, emit_all, manifest};

/// Everything that belongs in `dist/`, including the manifest.
#[must_use]
pub fn artifacts(palette: &Palette, spec_path: &str, spec_text: &str) -> Vec<EmittedFile> {
    let mut files = emit_all(palette);
    // The manifest hashes the others, so it is built last and listed first.
    let manifest = manifest(spec_path, spec_text, &files);
    files.insert(0, manifest);
    files
}

/// Writes every artifact under `root`, removing anything stale.
///
/// # Errors
///
/// Propagates any filesystem error.
pub fn write(
    root: &Path,
    palette: &Palette,
    spec_path: &str,
    spec_text: &str,
) -> io::Result<Vec<PathBuf>> {
    let files = artifacts(palette, spec_path, spec_text);
    let expected: BTreeSet<String> = files.iter().map(|f| f.path.clone()).collect();

    let mut written = Vec::with_capacity(files.len());
    for file in &files {
        let path = root.join(&file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Only touch the file if the bytes actually differ, so an unchanged
        // build leaves modification times — and any watcher — alone.
        let unchanged =
            std::fs::read_to_string(&path).is_ok_and(|existing| existing == file.contents);
        if !unchanged {
            std::fs::write(&path, &file.contents)?;
        }
        written.push(path);
    }

    for stale in stale_files(root, &expected)? {
        std::fs::remove_file(root.join(&stale))?;
    }
    prune_empty_directories(root)?;

    Ok(written)
}

/// A way in which `dist/` disagrees with the spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drift {
    /// The file exists but its contents differ. Almost always a hand-edit.
    Changed(String),
    /// The file should exist and does not.
    Missing(String),
    /// The file exists and nothing generates it any more.
    Stale(String),
}

impl std::fmt::Display for Drift {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Changed(path) => write!(f, "dist/{path} was edited by hand"),
            Self::Missing(path) => write!(f, "dist/{path} is missing"),
            Self::Stale(path) => write!(f, "dist/{path} is no longer generated"),
        }
    }
}

/// Checks `dist/` against what the spec would produce right now.
///
/// # Errors
///
/// Propagates any filesystem error other than a missing file, which is
/// reported as [`Drift::Missing`].
pub fn check(
    root: &Path,
    palette: &Palette,
    spec_path: &str,
    spec_text: &str,
) -> io::Result<Vec<Drift>> {
    let files = artifacts(palette, spec_path, spec_text);
    let expected: BTreeSet<String> = files.iter().map(|f| f.path.clone()).collect();
    let mut drift = Vec::new();

    for file in &files {
        match std::fs::read_to_string(root.join(&file.path)) {
            Ok(existing) if existing == file.contents => {}
            Ok(_) => drift.push(Drift::Changed(file.path.clone())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                drift.push(Drift::Missing(file.path.clone()));
            }
            Err(error) => return Err(error),
        }
    }

    drift.extend(stale_files(root, &expected)?.into_iter().map(Drift::Stale));
    Ok(drift)
}

/// Byproducts of *using* what is in `dist/`, rather than of generating it.
///
/// The Rust target emits a real crate, so anyone who compiles it leaves a
/// `target/` directory and a lockfile behind. Those are not stale artifacts
/// and deleting them on every build would be both rude and slow, so the sync
/// check steps over them.
fn is_build_byproduct(relative: &str) -> bool {
    relative.split('/').any(|component| component == "target") || relative.ends_with("Cargo.lock")
}

/// Files present under `root` that nothing generates.
fn stale_files(root: &Path, expected: &BTreeSet<String>) -> io::Result<Vec<String>> {
    let mut stale = Vec::new();
    if !root.exists() {
        return Ok(stale);
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            let relative = path
                .strip_prefix(root)
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();

            if path.is_dir() {
                if !is_build_byproduct(&relative) {
                    stack.push(path);
                }
            } else if !expected.contains(&relative) && !is_build_byproduct(&relative) {
                stale.push(relative);
            }
        }
    }
    stale.sort();
    Ok(stale)
}

/// Removes directories left empty after stale files were deleted.
fn prune_empty_directories(root: &Path) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    let mut directories = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                directories.push(path.clone());
                stack.push(path);
            }
        }
    }
    // Deepest first, so a directory whose only contents were empty
    // directories is itself removed.
    directories.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for directory in directories {
        if std::fs::read_dir(&directory)?.next().is_none() {
            std::fs::remove_dir(&directory)?;
        }
    }
    Ok(())
}

/// The sentence a failing sync check should print.
#[must_use]
pub fn explain(drift: &[Drift]) -> String {
    use std::fmt::Write as _;

    let mut out = format!(
        "dist/ is out of sync with the spec ({} file{}):\n",
        drift.len(),
        if drift.len() == 1 { "" } else { "s" }
    );
    for item in drift {
        writeln!(out, "  {item}").expect("string write");
    }
    out.push_str(
        "\nGenerated files are never edited by hand. Change specs/noctua.toml \
         and run `cargo xtask build`.",
    );
    out
}

#[cfg(test)]
mod tests {
    use noctua_engine::build;

    use super::*;

    fn fixture() -> (Palette, String) {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        let text = std::fs::read_to_string(path).expect("the shipped spec");
        let palette = build(&noctua_spec::load(path).expect("valid")).expect("builds");
        (palette, text)
    }

    /// A scratch directory that cleans up after itself.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("noctua-dist-{name}"));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("create scratch");
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_fresh_write_is_immediately_in_sync() {
        let (palette, text) = fixture();
        let scratch = Scratch::new("fresh");

        write(&scratch.0, &palette, "specs/noctua.toml", &text).expect("write");
        let drift = check(&scratch.0, &palette, "specs/noctua.toml", &text).expect("check");
        assert!(drift.is_empty(), "{}", explain(&drift));
    }

    #[test]
    fn an_empty_directory_reports_everything_missing() {
        let (palette, text) = fixture();
        let scratch = Scratch::new("empty");

        let drift = check(&scratch.0, &palette, "specs/noctua.toml", &text).expect("check");
        assert!(!drift.is_empty());
        assert!(drift.iter().all(|d| matches!(d, Drift::Missing(_))));
    }

    /// The failure this whole module exists to catch.
    #[test]
    fn a_hand_edited_file_is_caught() {
        let (palette, text) = fixture();
        let scratch = Scratch::new("edited");
        write(&scratch.0, &palette, "specs/noctua.toml", &text).expect("write");

        let victim = scratch.0.join("css/ochre-balanced.css");
        let mut contents = std::fs::read_to_string(&victim).expect("read");
        contents.push_str("\n:root { --nc-accent-solid: #ff0000; }\n"); // allow-literal: fixture: a hand-edit the sync check must catch
        std::fs::write(&victim, contents).expect("write");

        let drift = check(&scratch.0, &palette, "specs/noctua.toml", &text).expect("check");
        assert_eq!(
            drift,
            vec![Drift::Changed("css/ochre-balanced.css".to_owned())]
        );

        let message = explain(&drift);
        assert!(message.contains("edited by hand"), "{message}");
        assert!(message.contains("cargo xtask build"), "{message}");
    }

    #[test]
    fn a_file_nothing_generates_is_reported_and_then_removed() {
        let (palette, text) = fixture();
        let scratch = Scratch::new("stale");
        write(&scratch.0, &palette, "specs/noctua.toml", &text).expect("write");

        let orphan = scratch.0.join("css/leftover.css");
        std::fs::write(&orphan, "/* from a target that no longer exists */").expect("write");

        let drift = check(&scratch.0, &palette, "specs/noctua.toml", &text).expect("check");
        assert_eq!(drift, vec![Drift::Stale("css/leftover.css".to_owned())]);

        write(&scratch.0, &palette, "specs/noctua.toml", &text).expect("rewrite");
        assert!(!orphan.exists(), "a rebuild must clear stale output");
    }

    /// Compiling the generated crate must not make `dist/` look dirty.
    #[test]
    fn build_output_from_the_generated_crate_is_not_mistaken_for_drift() {
        let (palette, text) = fixture();
        let scratch = Scratch::new("byproducts");
        write(&scratch.0, &palette, "specs/noctua.toml", &text).expect("write");

        // What `cargo build` inside dist/rust leaves behind.
        std::fs::create_dir_all(scratch.0.join("rust/target/debug")).expect("mkdir");
        std::fs::write(scratch.0.join("rust/target/debug/libtokens.rlib"), "binary")
            .expect("write");
        std::fs::write(scratch.0.join("rust/Cargo.lock"), "# lockfile").expect("write");

        let drift = check(&scratch.0, &palette, "specs/noctua.toml", &text).expect("check");
        assert!(drift.is_empty(), "{}", explain(&drift));

        // ...and a rebuild must not delete them either.
        write(&scratch.0, &palette, "specs/noctua.toml", &text).expect("rewrite");
        assert!(scratch.0.join("rust/target/debug/libtokens.rlib").exists());
        assert!(scratch.0.join("rust/Cargo.lock").exists());
    }

    #[test]
    fn writing_twice_produces_identical_bytes() {
        let (palette, text) = fixture();
        let scratch = Scratch::new("twice");

        write(&scratch.0, &palette, "specs/noctua.toml", &text).expect("write");
        let first: Vec<String> = artifacts(&palette, "specs/noctua.toml", &text)
            .iter()
            .map(|f| std::fs::read_to_string(scratch.0.join(&f.path)).expect("read"))
            .collect();

        write(&scratch.0, &palette, "specs/noctua.toml", &text).expect("write again");
        let second: Vec<String> = artifacts(&palette, "specs/noctua.toml", &text)
            .iter()
            .map(|f| std::fs::read_to_string(scratch.0.join(&f.path)).expect("read"))
            .collect();

        assert_eq!(first, second);
    }

    #[test]
    fn the_manifest_is_listed_first_and_covers_the_rest() {
        let (palette, text) = fixture();
        let files = artifacts(&palette, "specs/noctua.toml", &text);
        assert_eq!(files[0].path, "MANIFEST.json");

        let manifest: serde_json::Value =
            serde_json::from_str(&files[0].contents).expect("valid JSON");
        let listed = manifest["files"].as_object().expect("a files map");
        assert_eq!(
            listed.len(),
            files.len() - 1,
            "every artifact but the manifest itself"
        );
    }
}
