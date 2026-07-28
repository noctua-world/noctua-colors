//! The committed `system/` must match what the spec produces right now.
//!
//! Generated artifacts are committed so consumers need no build step. That
//! only holds if they are actually generated, so this test regenerates
//! everything and byte-compares.
//!
//! This is the check that makes the accident uncommittable. An ordinary
//! `cargo xtask build` writes the scratch tree, so a hue you were only trying
//! out never reaches `system/` — but the reverse matters just as much: once you
//! *do* mean it, `system/` must be rewritten or this test fails and CI with it.
//!
//! The ordinary way to accept an intentional change is `cargo xtask build
//! --system`. This escape hatch exists for the same job without a full build:
//!
//! ```text
//! UPDATE_SYSTEM=1 cargo test -p noctua-emit --test output_sync
//! ```

use std::path::PathBuf;

use noctua_emit::output;

fn repository_root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn the_committed_system_matches_the_specification() {
    let root = repository_root();
    let spec_path = root.join("specs/noctua.toml");
    let spec_text = std::fs::read_to_string(&spec_path).expect("the shipped spec");
    let palette =
        noctua_engine::build(&noctua_spec::load(&spec_path).expect("valid")).expect("builds");

    let system_root = root.join("system");

    if std::env::var_os("UPDATE_SYSTEM").is_some() {
        let written = output::write(&system_root, &palette, "specs/noctua.toml", &spec_text)
            .expect("write the system");
        println!("wrote {} files", written.len());
        return;
    }

    let drift = output::check(&system_root, &palette, "specs/noctua.toml", &spec_text)
        .expect("read the system");
    assert!(drift.is_empty(), "{}", output::explain(&drift));
}

/// The names promised in `README.md`'s integration guides must exist.
///
/// A README describing an API that no longer exists is worse than no README,
/// and these are exactly the strings a reader will copy verbatim.
#[test]
fn the_readme_integration_examples_still_resolve() {
    let system = repository_root().join("system");
    let read = |relative: &str| {
        std::fs::read_to_string(system.join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
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

    // Tailwind. The entry point is imports only and the utility names live in
    // the bridge it pulls in, so resolve the import rather than reading one
    // file — that is what a bundler does, and what the README's reader gets.
    let entry = read("tailwind/theme.css");
    assert!(
        entry.contains(r#"@import "./bridge.css";"#),
        "the entry no longer reaches the mapping"
    );
    let tailwind = read("tailwind/bridge.css");
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

    // The per-palette route the README leads with. Both halves must be
    // reachable: the palette's own values, and the utilities that name them.
    let small = read("css/palette/ochre-vivid.css");
    assert!(small.contains("--nc-color-surface:"), "the contract");
    assert!(small.contains("--nc-gray-18:"), "the neutral ramp");
    assert!(small.contains("--nc-accent-solid:"), "the palette");
    let small_tailwind = read("tailwind/palette/ochre-vivid.css");
    assert!(small_tailwind.contains(r#"@import "../../css/palette/ochre-vivid.css";"#));
    assert!(small_tailwind.contains(r#"@import "../bridge.css";"#));

    // Every context name the README spells out must exist. These are the
    // strings a beginner copies verbatim out of the front page, and one of
    // them being wrong is worse than the section not being there.
    for token in [
        "--nc-color-surface-raised:",
        "--nc-color-fg-muted:",
        "--nc-color-border-strong:",
        "--nc-color-accent:",
        "--nc-color-accent-hover:",
        "--nc-color-on-accent:",
        "--nc-color-success:",
        "--nc-color-on-success:",
        "--nc-color-info-bg:",
        "--nc-color-info-border:",
        "--nc-color-danger:",
        "--nc-color-danger-bg:",
        "--nc-color-danger-border:",
        "--nc-color-on-danger:",
        "--nc-color-overdue:",
        "--nc-color-on-overdue:",
    ] {
        assert!(contexts.contains(token), "README promises {token}");
    }
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

/// `on-<name>` goes on the **solid**, never on the tinted `-bg`.
///
/// Both are valid colours, so pairing them wrongly produces text that is
/// nearly invisible with no warning from anything — CSS is happy, the linter
/// is happy, and the page just looks broken. It happened in a draft of the
/// README's copy-paste example, where `success-bg` and `on-success` landed
/// 0.03 apart in lightness.
///
/// So the rule the README teaches is asserted here rather than trusted:
/// `on-X` must be far from `X-bg` and close to nothing about it.
#[test]
fn the_on_token_belongs_to_the_solid_not_the_tinted_background() {
    let palette = {
        let path = repository_root().join("specs/noctua.toml");
        noctua_engine::build(&noctua_spec::load(&path).expect("valid")).expect("builds")
    };

    // Resolve the three tokens through the palette rather than by parsing CSS,
    // so this measures colours rather than strings.
    let mode = &palette.themes[0].modes[0];
    let family = mode
        .families
        .get("success")
        .expect("the success family exists");

    let lightness = |role: &str| {
        family
            .steps
            .iter()
            .find(|s| s.role == role)
            .unwrap_or_else(|| panic!("no {role} step"))
            .primary()
            .oklch
            .l
    };

    let solid = lightness("solid");
    let on_solid = lightness("bg-app"); // what `on-success` points at
    let tinted = lightness("bg-subtle"); // what `success-bg` points at

    let against_solid = (on_solid - solid).abs();
    let against_tinted = (on_solid - tinted).abs();

    assert!(
        against_solid > 0.3,
        "on-success must contrast with the solid it is for: {against_solid:.3} apart"
    );
    assert!(
        against_tinted < 0.1,
        "on-success sits {against_tinted:.3} from the tinted background — if this \
         grows, the two are no longer the trap the README warns about and the \
         warning should be revisited"
    );
}
