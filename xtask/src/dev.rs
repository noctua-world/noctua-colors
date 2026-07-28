//! `cargo xtask dev` — watch the spec and rebuild.
//!
//! The inner loop for tuning colors: change a hue, see every artifact and
//! every gate result a moment later, without running anything.
//!
//! Gates run on each rebuild rather than only on request, because the
//! feedback that matters while tuning is "did that break a contrast target",
//! and finding out at commit time is finding out too late.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::new_debouncer;

use crate::{build, serve, site, ui};

/// Editors write files in bursts — truncate, write, rename — and a naive
/// watcher rebuilds three times for one save.
const SETTLE: Duration = Duration::from_millis(250);

/// Watches the spec until interrupted.
///
/// # Errors
///
/// A port already in use, or a watcher that cannot be established. A failing
/// *build* is reported and the loop continues, because a spec is broken half
/// the time you are editing it and exiting would make the tool useless — but
/// a port that cannot be claimed is not recoverable, and pretending otherwise
/// leaves a watcher running with nothing to browse.
pub(crate) fn run(root: &Path, spec_path: &Path, port: u16) -> Result<(), String> {
    // Claimed before the build, so the failure that cannot be worked around
    // arrives first rather than after a screen of successful output.
    let server = serve::bind(port)?;

    rebuild(root, spec_path);

    // The server owns its own threads and never returns, so the watcher keeps
    // this one.
    let directory = root.join(site::OUTPUT);
    std::thread::spawn(move || serve::serve(server, directory, port));

    let (sender, receiver) = mpsc::channel();
    let mut debouncer = new_debouncer(SETTLE, None, sender)
        .map_err(|error| format!("could not start the file watcher: {error}"))?;

    debouncer
        .watch(spec_path, RecursiveMode::NonRecursive)
        .map_err(|error| format!("could not watch {}: {error}", spec_path.display()))?;

    ui::gap();
    ui::heading(&format!("watching {}", build::relative_to(root, spec_path)));
    ui::detail("the spec, the site sources, and the stylesheets");
    ui::detail("press Ctrl-C to stop");

    for result in receiver {
        match result {
            Ok(_) => {
                ui::gap();
                rebuild(root, spec_path);
            }
            Err(errors) => {
                for error in errors {
                    ui::warn(&format!("watcher: {error}"));
                }
            }
        }
    }

    Ok(())
}

/// Rebuilds and reports, swallowing failures so the loop survives them.
fn rebuild(root: &Path, spec_path: &Path) {
    // Never publishes. `dev` is the tightest loop there is — rebuilding on
    // every keystroke into the committed tree is exactly the accident the
    // scratch directory exists to prevent.
    match build::all(root, spec_path, true, false) {
        Ok(palette) => {
            serve::GENERATION.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let mut report = noctua_check::run(&palette);
            report.absorb(noctua_check::source::check(root));

            for finding in report.failures() {
                ui::failure(&finding.to_string());
            }
            let warnings = report.warnings().len();
            if report.is_ok() {
                ui::ok(&format!(
                    "{} checks passed, {warnings} warning(s)",
                    report.checked
                ));
            }
        }
        Err(error) => ui::failure(&error),
    }
}
