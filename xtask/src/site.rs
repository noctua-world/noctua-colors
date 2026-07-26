//! Rendering the documentation site.
//!
//! The site is generated from `dist/`, so it can only be built after the spec
//! has been compiled. That ordering is the mechanism which keeps the site
//! honest: it reads the same artifacts every other consumer reads.

use std::path::Path;

use crate::ui;

/// Where the rendered site is written. Gitignored — CI builds and deploys it.
pub(crate) const OUTPUT: &str = "docs-site/public";

/// Renders the site into `docs-site/public/`.
///
/// # Errors
///
/// A missing `dist/`, or any filesystem failure.
pub(crate) fn build(root: &Path) -> Result<usize, String> {
    let dist = root.join("dist");
    let out = root.join(OUTPUT);
    let source = root.join("docs-site");

    let palette = noctua_docs::load(&dist)?;
    let pages = noctua_docs::render(&dist)?;

    // A clean rebuild, so a renamed page never lingers as a stale route.
    if out.exists() {
        std::fs::remove_dir_all(&out)
            .map_err(|error| format!("could not clear {OUTPUT}: {error}"))?;
    }

    let mut written = 0usize;
    for page in &pages {
        write(&out.join(&page.path), page.contents.as_bytes())?;
        written += 1;
    }

    for (from, to) in noctua_docs::assets() {
        let bytes = std::fs::read(source.join(from))
            .map_err(|error| format!("could not read docs-site/{from}: {error}"))?;
        write(&out.join(to), &bytes)?;
        written += 1;
    }

    // The tokens, copied in exactly as a consumer would receive them.
    for file in noctua_docs::token_files(&palette) {
        let bytes = std::fs::read(dist.join(&file))
            .map_err(|error| format!("could not read dist/{file}: {error}"))?;
        write(&out.join("tokens").join(&file), &bytes)?;
        written += 1;
    }

    // The playground last, and optionally: the site is complete without it.
    if crate::wasm::build(root, &out)? {
        written += crate::wasm::OUTPUT_FILES.len();
    }

    Ok(written)
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, bytes)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

/// Reports what was built.
pub(crate) fn report(root: &Path, written: usize) {
    let out = root.join(OUTPUT);
    let bytes: u64 = walk(&out)
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok().map(|m| m.len()))
        .sum();
    ui::ok(&format!("rendered {written} files to {OUTPUT}"));
    ui::detail(&format!("{:.0} KB total", bytes as f64 / 1024.0));
}

fn walk(base: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![base.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found
}
