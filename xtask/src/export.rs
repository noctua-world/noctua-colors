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

use std::path::Path;

use crate::{build, ui};

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
        let outside = !destination.starts_with(root);

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
