//! The committed `dist/` must match what the spec produces right now.
//!
//! Generated artifacts are committed so consumers need no build step. That
//! only holds if they are actually generated, so this test regenerates
//! everything and byte-compares.
//!
//! To accept an intentional change:
//!
//! ```text
//! UPDATE_DIST=1 cargo test -p noctua-emit --test dist_sync
//! ```
//!
//! Milestone 4 moves this behind `cargo xtask build` and `cargo xtask check`;
//! the functions it calls are already the ones those commands will use.

use std::path::PathBuf;

use noctua_emit::dist;

fn repository_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn dist_matches_the_specification() {
    let root = repository_root();
    let spec_path = root.join("specs/noctua.toml");
    let spec_text = std::fs::read_to_string(&spec_path).expect("the shipped spec");
    let palette =
        noctua_engine::build(&noctua_spec::load(&spec_path).expect("valid")).expect("builds");

    let dist_root = root.join("dist");

    if std::env::var_os("UPDATE_DIST").is_some() {
        let written =
            dist::write(&dist_root, &palette, "specs/noctua.toml", &spec_text).expect("write dist");
        println!("wrote {} files", written.len());
        return;
    }

    let drift =
        dist::check(&dist_root, &palette, "specs/noctua.toml", &spec_text).expect("read dist");
    assert!(drift.is_empty(), "{}", dist::explain(&drift));
}

/// The names promised in `README.md`'s integration guides must exist.
///
/// A README describing an API that no longer exists is worse than no README,
/// and these are exactly the strings a reader will copy verbatim.
#[test]
fn the_readme_integration_examples_still_resolve() {
    let dist = repository_root().join("dist");
    let read = |relative: &str| {
        std::fs::read_to_string(dist.join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
    };

    // Plain CSS. The semantic contract lives in `contexts.css` — every theme
    // resolved it identically, so it is written once — and the theme file
    // carries the values it points at. The README says to link both.
    let contexts = read("css/contexts.css");
    for token in [
        "--nc-color-surface-raised:",
        "--nc-color-fg:",
        "--nc-color-border:",
        "--nc-color-ring:",
    ] {
        assert!(contexts.contains(token), "README promises {token}");
    }
    let css = read("css/ochre-balanced.css");
    assert!(
        css.contains("--nc-neutral-bg-element:"),
        "the theme file must carry the values the contract points at"
    );

    // Tailwind.
    let tailwind = read("tailwind/theme.css");
    for utility in [
        "--color-surface:",
        "--color-accent-hover:",
        "--color-chart-3:",
    ] {
        assert!(tailwind.contains(utility), "README promises {utility}");
    }
    assert!(
        tailwind.contains("--color-gray-18:"),
        "README promises bg-gray-18"
    );

    // Rust.
    let rust = read("rust/src/lib.rs");
    assert!(rust.contains("pub mod ochre_balanced {"));
    assert!(rust.contains("pub const SOLID: Color"));
    assert!(rust.contains("pub const fn packed(self) -> u32"));

    // SCSS: both the variable and the flat-map key the README shows.
    let scss = read("scss/_noctua.scss");
    assert!(scss.contains("$nc-ochre-balanced-light-neutral-bg-app:"));
    assert!(scss.contains(r#"  "ochre-balanced-light-accent-solid": "#));

    // QML.
    assert!(read("qml/qmldir").contains("singleton OchreBalancedDark 1.0 OchreBalancedDark.qml"));
    let qml = read("qml/OchreBalancedDark.qml");
    for property in ["surface:", "border:", "fg:", "accent:"] {
        assert!(
            qml.contains(&format!("readonly property color {property}")),
            "README promises OchreBalancedDark.{property}"
        );
    }

    // TypeScript and JSON.
    let data: serde_json::Value =
        serde_json::from_str(&read("json/palette.json")).expect("valid JSON");
    let step = &data["themes"]["ochre-balanced"]["light"]["families"]["accent"]["steps"][8];
    assert_eq!(
        step["role"], "solid",
        "README indexes steps[8] as the solid"
    );
    assert!(step["renditions"][0]["css"].is_string());
    assert!(step["renditions"][0]["chromaHeadroom"].is_number());
}
