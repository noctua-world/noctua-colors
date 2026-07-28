//! The developer command surface.
//!
//! Six verbs. The developer edits `specs/noctua.toml` and runs one of these;
//! nothing else needs learning, and no tool needs installing first — the
//! `cargo xtask` alias lives in `.cargo/config.toml`.
//!
//! # Why `check` does everything
//!
//! It validates the spec, runs every quality gate, verifies `dist/` is in
//! sync, and runs formatting, lints and tests. One command, so what runs
//! locally is exactly what runs in CI. "Passes on my machine, fails in CI"
//! is a category of problem this removes rather than manages.

mod build;
mod check;
mod dev;
mod export;
mod import;
mod packaging;
mod release;
mod serve;
mod site;
mod ui;
mod wasm;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use noctua_core::Gamut;

/// Compile a color specification into every artifact other projects consume.
#[derive(Debug, Parser)]
#[command(name = "cargo xtask", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Path to the specification, relative to the repository root.
    #[arg(long, global = true, default_value = "specs/noctua.toml")]
    spec: PathBuf,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Compile the spec into every target under dist/, and render the site.
    Build {
        /// Skip rendering the documentation site.
        #[arg(long)]
        no_site: bool,
    },

    /// Validate the spec, run every quality gate, and verify dist/ is in sync.
    ///
    /// This is the single gate. CI runs exactly this command.
    Check {
        /// Skip formatting, lints and tests, checking only the palette.
        ///
        /// For a fast inner loop while tuning colors. CI never passes this.
        #[arg(long)]
        colors_only: bool,
    },

    /// Watch the spec, rebuild, and serve the site with live reload.
    Dev {
        /// Port for the development server.
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },

    /// Copy dist/ into every consumer registered in the spec.
    Export {
        /// Report what would be written without writing anything.
        #[arg(long)]
        dry_run: bool,
    },

    /// Fit an existing palette back to spec parameters.
    ///
    /// Takes a file, or a bare list of colors. Reads `#rgb`, `#rrggbb`,
    /// `#aarrggbb`, `rgb()` and `oklch()` out of CSS, SCSS, QML, JSON or plain
    /// text, groups them into ramps by name, and reports how closely the
    /// curve model can express each one.
    Import {
        /// A file to read, or a comma-separated list of hex colors.
        source: String,

        /// Name for the fitted family. Defaults to the name in the source.
        #[arg(long)]
        name: Option<String>,

        /// Gamut to measure relative chroma against.
        ///
        /// Defaults to the spec's own. Set this to the gamut the source was
        /// authored for: a palette designed for P3 has colors sRGB cannot
        /// show, and relative chroma above 1 is not a value this model can
        /// hold, so fitting it against sRGB measures the gamut rather than
        /// the palette.
        #[arg(long)]
        gamut: Option<String>,
    },

    /// Tag a version and verify everything is publishable.
    Release {
        /// The version to release, such as `0.2.0`.
        version: String,

        /// Report what would happen without changing anything.
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = repository_root();
    let spec = root.join(&cli.spec);

    let outcome = match cli.command {
        Command::Build { no_site } => build::all(&root, &spec, !no_site).map(|_| ()),
        Command::Check { colors_only } => check::run(&root, &spec, colors_only),
        Command::Dev { port } => dev::run(&root, &spec, port),
        Command::Export { dry_run } => export::run(&root, &spec, dry_run),
        Command::Import {
            source,
            name,
            gamut,
        } => {
            // The spec's own target gamut by default, so relative chroma in
            // the fitted fragment means the same thing it will mean once
            // pasted.
            let resolved = match gamut.as_deref() {
                Some(id) => Gamut::from_id(id).ok_or_else(|| {
                    let known: Vec<&str> = Gamut::all().into_iter().map(Gamut::id).collect();
                    format!(
                        "unknown gamut `{id}`; expected one of: {}",
                        known.join(", ")
                    )
                }),
                None => Ok(noctua_spec::load(&spec).map_or(Gamut::Srgb, |s| s.output.gamut)),
            };

            resolved.and_then(|gamut| import::run(&root, &source, name.as_deref(), gamut))
        }
        Command::Release { version, dry_run } => release::run(&root, &spec, &version, dry_run),
    };

    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            ui::failure(&error);
            ExitCode::FAILURE
        }
    }
}

/// The repository root, found from this crate's location.
///
/// Independent of the working directory, so every verb behaves the same
/// whether it was run from the root or from inside a crate.
fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask always sits one level below the root")
        .to_path_buf()
}
