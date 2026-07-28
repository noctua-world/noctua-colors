//! The compiler, in a browser.
//!
//! Text in, artifacts out. This is a thin facade: it parses a spec, builds a
//! palette, runs the gates, and renders whichever target was asked for — all
//! by calling the same crates the command line calls. Nothing about the color
//! model lives here, which is what keeps the playground honest. A palette
//! produced on the page is the palette `cargo xtask build` would write.
//!
//! # Why the boundary is strings
//!
//! Every entry point takes a spec as text and returns JSON. Passing rich
//! types across the wasm boundary means maintaining a second description of
//! every type in the system, and that second description is exactly the thing
//! that drifts. A string in and a string out cannot drift, and the cost is
//! one `JSON.parse` on the other side.

use std::fmt::Write as _;

use noctua_core::Gamut;
use wasm_bindgen::prelude::wasm_bindgen;

/// Compiles a spec and returns the palette as JSON.
///
/// The same JSON `cargo xtask build` writes to `system/json/palette.json`, so
/// the playground and the docs site read one format.
///
/// # Errors
///
/// Returns the diagnostic as a string when the spec does not parse, does not
/// validate, or asks for a color the gamut cannot reach.
#[wasm_bindgen]
pub fn compile(spec: &str) -> Result<String, String> {
    let palette = build(spec)?;
    let emitter =
        noctua_emit::by_id("json-ts").ok_or_else(|| "the JSON target is missing".to_owned())?;

    emitter
        .emit(&palette)
        .into_iter()
        .find(|f| f.path.ends_with("palette.json"))
        .map(|f| f.contents)
        .ok_or_else(|| "the JSON emitter produced no palette".to_owned())
}

/// Renders a spec to one named target, returning every file it produces.
///
/// The result is a JSON array of `{ "path": …, "contents": … }`, which is
/// what lets the playground offer the same downloads the command line writes.
///
/// # Errors
///
/// As [`compile`], plus an unknown target id.
#[wasm_bindgen]
pub fn emit(spec: &str, target: &str) -> Result<String, String> {
    let palette = build(spec)?;
    let emitter = noctua_emit::by_id(target).ok_or_else(|| {
        format!(
            "unknown target `{target}`; expected one of: {}",
            noctua_emit::ids().join(", ")
        )
    })?;
    let files = emitter.emit(&palette);

    let mut out = String::from("[");
    for (index, file) in files.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"path\":");
        push_json_string(&mut out, &file.path);
        out.push_str(",\"contents\":");
        push_json_string(&mut out, &file.contents);
        out.push('}');
    }
    out.push(']');
    Ok(out)
}

/// Runs every quality gate and returns the findings as JSON.
///
/// Findings, not a verdict. A playground that only said "pass" or "fail"
/// would be useless for the thing people actually do with it, which is push a
/// value until something breaks and see how far the margin went.
///
/// # Errors
///
/// As [`compile`].
#[wasm_bindgen]
pub fn check(spec: &str) -> Result<String, String> {
    let palette = build(spec)?;
    let report = noctua_check::run(&palette);

    let mut out = format!("{{\"checked\":{},\"findings\":[", report.checked);
    for (index, finding) in report.findings.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"gate\":");
        push_json_string(&mut out, finding.gate);
        out.push_str(",\"severity\":");
        push_json_string(&mut out, &finding.severity.to_string());
        out.push_str(",\"where\":");
        push_json_string(&mut out, &finding.where_);
        out.push_str(",\"message\":");
        push_json_string(&mut out, &finding.message);
        match finding.margin {
            Some(margin) => {
                let _ = write!(out, ",\"margin\":{margin}");
            }
            None => out.push_str(",\"margin\":null"),
        }
        out.push('}');
    }
    out.push_str("]}");
    Ok(out)
}

/// The largest chroma the gamut can show at a lightness and hue.
///
/// Exposed on its own because it is what makes relative chroma legible: the
/// playground draws the gamut boundary with it, so `cr = 0.8` becomes a
/// position on a visible curve rather than a number to be taken on trust.
#[wasm_bindgen]
#[must_use]
pub fn max_chroma(lightness: f64, hue: f64, gamut: &str) -> f64 {
    Gamut::from_id(gamut)
        .unwrap_or(Gamut::Srgb)
        .max_chroma(lightness, hue)
}

/// Every gamut the compiler knows, as a JSON array of ids.
#[wasm_bindgen]
#[must_use]
pub fn gamuts() -> String {
    let ids: Vec<String> = Gamut::all()
        .into_iter()
        .map(|g| format!("\"{}\"", g.id()))
        .collect();
    format!("[{}]", ids.join(","))
}

/// Every emitter target, as a JSON array of ids.
#[wasm_bindgen]
#[must_use]
pub fn targets() -> String {
    let ids: Vec<String> = noctua_emit::ids()
        .iter()
        .map(|id| format!("\"{id}\""))
        .collect();
    format!("[{}]", ids.join(","))
}

/// The spec the compiler ships with, as a starting point to edit.
///
/// Embedded at compile time from the real file, so the playground opens on
/// the palette the repository actually publishes rather than on a copy that
/// has to be kept in step by hand.
#[wasm_bindgen]
#[must_use]
pub fn default_spec() -> String {
    SHIPPED_SPEC.to_owned()
}

/// The repository's own specification.
const SHIPPED_SPEC: &str = include_str!("../../../specs/noctua.toml");

fn build(spec: &str) -> Result<noctua_engine::Palette, String> {
    let parsed = noctua_spec::parse("playground.toml", spec).map_err(|e| diagnostic(&e))?;
    noctua_engine::build(&parsed).map_err(|e| e.to_string())
}

/// Renders a specification error the way the command line would.
///
/// `miette`'s renderer draws a source excerpt with ANSI escapes and box
/// characters, which is right for a terminal and wrong for a textarea. The
/// information is the same either way, and it is the part that matters: a
/// playground that can only say "1 problem in the specification" makes the
/// developer hunt for it, which is exactly the experience this project set out
/// not to have.
fn diagnostic(error: &noctua_spec::SpecError) -> String {
    let source = error.source_text();
    let mut out = String::new();

    for problem in error.problems() {
        if !out.is_empty() {
            out.push('\n');
        }
        match problem.span() {
            Some((offset, _)) => {
                let (line, column) = position(source, offset);
                let _ = write!(out, "line {line}, column {column}: {}", problem.message());
            }
            None => {
                let _ = write!(out, "{}", problem.message());
            }
        }
        if !problem.help().is_empty() {
            let _ = write!(out, "\n  fix: {}", problem.help());
        }
    }

    if out.is_empty() {
        error.to_string()
    } else {
        out
    }
}

/// Turns a byte offset into a one-based line and column.
fn position(source: &str, offset: usize) -> (usize, usize) {
    let before = &source[..offset.min(source.len())];
    let line = before.matches('\n').count() + 1;
    let column = before
        .rsplit_once('\n')
        .map_or(before.len(), |(_, rest)| rest.len())
        + 1;
    (line, column)
}

/// Escapes a string into a JSON literal, quotes included.
///
/// Hand-rolled because this crate ships to a browser and a JSON serializer is
/// most of a megabyte of dependency for one function. Emitted files contain
/// newlines and quotes and nothing more exotic, but control characters are
/// handled anyway — a spec can contain anything a person typed.
fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_spec_compiles() {
        let json = compile(&default_spec()).expect("the shipped spec must compile");
        assert!(
            json.contains("\"themes\""),
            "{}",
            &json[..200.min(json.len())]
        );
    }

    #[test]
    fn a_broken_spec_says_where_and_what_to_do() {
        // "1 problem in the specification" is true and useless. The playground
        // has no terminal to render miette into, so it renders its own — and
        // must not lose the position or the fix in the process.
        let error = compile("[families.accent]\nhue = { base = 210, torsion = \"twenty\" }\n")
            .expect_err("must fail");

        assert!(error.contains("line 2"), "no position in: {error}");
        assert!(
            error.to_lowercase().contains("torsion") || error.contains("fix:"),
            "nothing actionable in: {error}"
        );
    }

    #[test]
    fn a_byte_offset_becomes_a_line_and_column() {
        let text = "alpha\nbeta\ngamma";
        assert_eq!(position(text, 0), (1, 1));
        assert_eq!(position(text, 6), (2, 1));
        assert_eq!(position(text, 8), (2, 3));
        assert_eq!(position(text, 11), (3, 1));
        // Past the end rather than panicking: a span is only ever as
        // trustworthy as the parser that produced it.
        assert_eq!(position(text, 9_999), (3, 6));
    }

    #[test]
    fn every_target_renders() {
        let spec = default_spec();
        for id in noctua_emit::ids() {
            let json = emit(&spec, id).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!(json.starts_with('['), "{id} produced {json:.60}");
            assert!(json.len() > 2, "{id} produced no files");
        }
    }

    #[test]
    fn an_unknown_target_is_an_error_not_a_panic() {
        assert!(emit(&default_spec(), "cobol").is_err());
    }

    /// The shape `docs-site/js/playground.js` reads.
    ///
    /// This is a contract between two languages with nothing but convention
    /// holding it together, and it has already broken once: the playground
    /// read `step.hex`, which does not exist, and every swatch rendered
    /// transparent. A color is under `renditions`, one per emitted gamut,
    /// because the same token is a different color in sRGB and in P3 — which
    /// is the entire point of relative chroma and exactly why the flat field
    /// the playground assumed cannot exist.
    #[test]
    fn the_json_has_the_fields_the_playground_reads() {
        let json = compile(&default_spec()).expect("compiles");

        for field in [
            "\"themes\"",
            "\"families\"",
            "\"steps\"",
            "\"role\"",
            "\"renditions\"",
            "\"hex\"",
            "\"relativeChroma\"",
        ] {
            assert!(json.contains(field), "the playground reads {field}");
        }

        // And the nesting, not just the names: `hex` must be inside a
        // rendition rather than beside `role`.
        let at_role = json.find("\"role\"").expect("a role");
        let after = &json[at_role..];
        let to_renditions = after
            .find("\"renditions\"")
            .expect("renditions follow a role");
        let to_hex = after.find("\"hex\"").expect("a hex follows a role");
        assert!(
            to_renditions < to_hex,
            "hex must sit inside renditions, not beside role"
        );
    }

    #[test]
    fn the_gates_run_and_report_findings() {
        let json = check(&default_spec()).expect("checks run");
        assert!(json.contains("\"checked\""));
        assert!(json.contains("\"findings\""));
    }

    #[test]
    fn the_gamut_boundary_is_reachable() {
        let at_mid = max_chroma(0.6, 264.0, "srgb");
        assert!(at_mid > 0.0, "sRGB shows some chroma at L 0.6");
        assert!(
            max_chroma(0.6, 264.0, "display-p3") > at_mid,
            "a wider gamut must show more"
        );
        // An unknown id falls back rather than failing: this is a drawing
        // helper called from a render loop, not a validation entry point.
        assert!((max_chroma(0.6, 264.0, "nonsense") - at_mid).abs() < 1e-12);
    }

    #[test]
    fn the_listings_are_valid_json_arrays() {
        for listing in [gamuts(), targets()] {
            assert!(
                listing.starts_with('[') && listing.ends_with(']'),
                "{listing}"
            );
            assert!(listing.contains('"'), "{listing}");
        }
    }

    #[test]
    fn strings_survive_the_json_boundary() {
        let mut out = String::new();
        push_json_string(&mut out, "a \"quoted\" \\ path\nwith\ttabs");
        assert_eq!(out, r#""a \"quoted\" \\ path\nwith\ttabs""#);

        // A control character must not be emitted raw: it would produce JSON
        // the browser refuses to parse, and the failure would surface as an
        // empty playground rather than as an error.
        let mut control = String::new();
        push_json_string(&mut control, "\u{1}");
        assert_eq!(control, "\"\\u0001\"");
    }
}
