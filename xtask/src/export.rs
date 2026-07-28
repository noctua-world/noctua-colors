//! `cargo xtask export` — copy `dist/` into every registered consumer.
//!
//! A consumer is three lines in the spec: a name, a path, and which targets it
//! wants. Nothing else needs configuring, and adding a project never means
//! touching build files on either side.
//!
//! Paths outside the repository are allowed and expected — that is the whole
//! point — but they are reported before anything is written, because writing
//! into somebody else's checkout unannounced is not a thing a build tool
//! should do quietly.

use std::path::{Component, Path, PathBuf};

use crate::{build, ui};

/// Resolves `..` and `.` lexically, without touching the filesystem.
///
/// `Path::starts_with` compares components and does not understand `..`, so
/// `<root>/../noctua-design` *starts with* `<root>` as far as it is concerned —
/// which silently disabled the "outside this repository" notice for exactly the
/// sibling paths it exists to announce. The first `../` consumer registered in
/// the spec is what surfaced it.
///
/// Lexical rather than `fs::canonicalize`, deliberately: a consumer's directory
/// legitimately does not exist yet on the first export, and `canonicalize` fails
/// on a path that is not there. The cost is that a symlink is not followed, so a
/// symlinked consumer inside the repository would be reported as outside — the
/// safe direction for a notice whose only job is to over-announce.
fn normalize(path: &Path) -> PathBuf {
    path.components()
        .fold(PathBuf::new(), |mut acc, component| {
            match component {
                Component::ParentDir => {
                    acc.pop();
                }
                Component::CurDir => {}
                other => acc.push(other),
            }
            acc
        })
}

/// Copies each consumer's requested targets into its path.
///
/// # Errors
///
/// An unknown target name, or a filesystem failure.
pub(crate) fn run(root: &Path, spec_path: &Path, dry_run: bool) -> Result<(), String> {
    let spec = noctua_spec::load(spec_path)
        .map_err(|error| format!("{:?}", miette::Report::new(error)))?;
    let palette = build::run(root, spec_path)?;

    if spec.consumers.is_empty() {
        ui::gap();
        ui::warn("no consumers are registered, so there is nowhere to export to");
        ui::detail("add one to the spec:");
        ui::detail("    [[consumers]]");
        ui::detail("    name    = \"noctua-hub\"");
        ui::detail("    path    = \"../noctua-hub/tokens\"");
        ui::detail("    targets = [\"qml\"]");
        return Ok(());
    }

    let known = noctua_emit::ids();
    let mut wrote = 0usize;

    ui::gap();
    ui::heading(if dry_run {
        "export (dry run)"
    } else {
        "export"
    });

    for consumer in &spec.consumers {
        for target in &consumer.targets {
            if !known.contains(&target.as_str()) {
                return Err(format!(
                    "consumer `{}` asks for target `{target}`, which does not exist.\n  \
                     Available targets: {}",
                    consumer.name,
                    known.join(", ")
                ));
            }
        }

        let destination = root.join(&consumer.path);
        let outside = !normalize(&destination).starts_with(normalize(root));

        let emitter_files: Vec<_> = consumer
            .targets
            .iter()
            .filter_map(|id| noctua_emit::by_id(id))
            .flat_map(|emitter| emitter.emit(&palette))
            .collect();

        ui::ok(&format!(
            "{} <- {} ({} file(s))",
            consumer.path,
            consumer.targets.join(", "),
            emitter_files.len()
        ));
        if outside {
            ui::detail("outside this repository");
        }

        if dry_run {
            continue;
        }

        for file in &emitter_files {
            let path = destination.join(&file.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
            }
            // Only write when the bytes differ, so an unchanged export leaves
            // the consumer's modification times — and their watcher — alone.
            let unchanged =
                std::fs::read_to_string(&path).is_ok_and(|existing| existing == file.contents);
            if !unchanged {
                std::fs::write(&path, &file.contents)
                    .map_err(|error| format!("could not write {}: {error}", path.display()))?;
                wrote += 1;
            }
        }
    }

    ui::gap();
    if dry_run {
        ui::detail("nothing was written; drop --dry-run to export");
    } else {
        ui::ok(&format!("{wrote} file(s) changed"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::normalize;
    use std::path::Path;

    /// The "outside this repository" notice exists so that writing into somebody
    /// else's checkout is never quiet. It was silently disabled for every
    /// `../sibling` consumer, because `Path::starts_with` is lexical and does not
    /// understand `..` — so `<root>/../noctua-design` *started with* `<root>`.
    ///
    /// This test found that bug, on the day the first sibling consumer was
    /// registered in the spec. It compares the way the code does, so it fails
    /// again if the normalization is ever dropped.
    #[test]
    fn a_sibling_consumer_is_recognised_as_outside_the_repository() {
        let root = Path::new("/w/repos/noctua-colors");
        let sibling = root.join("../noctua-design/packages/tokens/vendor");

        assert!(
            sibling.starts_with(root),
            "the lexical comparison still sees a sibling as inside — \
             if this ever fails, std changed and the workaround can go"
        );
        assert!(
            !normalize(&sibling).starts_with(normalize(root)),
            "a sibling consumer must be reported as outside the repository"
        );
    }

    /// The notice must not cry wolf: an in-repository consumer, which is the
    /// common case, has to stay silent — including when its path is written with
    /// redundant `./` or a `..` that cancels out.
    #[test]
    fn an_in_repository_consumer_is_not_reported_as_outside() {
        let root = Path::new("/w/repos/noctua-colors");
        for path in [
            "docs-site/vendor/tokens",
            "./docs-site/vendor/tokens",
            "docs-site/../docs-site/vendor/tokens",
        ] {
            let destination = normalize(&root.join(path));
            assert!(
                destination.starts_with(normalize(root)),
                "{path} is inside the repository and must not be announced"
            );
        }
    }

    /// `..` past the root must not underflow into an empty path that then
    /// "starts with" nothing — a consumer pointing above the filesystem root is
    /// nonsense, and it has to be reported rather than silently accepted.
    #[test]
    fn a_path_climbing_past_the_root_is_still_outside() {
        let root = Path::new("/w/repos/noctua-colors");
        let escaped = root.join("../../../../../../elsewhere");
        assert!(!normalize(&escaped).starts_with(normalize(root)));
    }
}
