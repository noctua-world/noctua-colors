//! Design Tokens Community Group JSON.
//!
//! The point of this target is that an existing Style Dictionary pipeline
//! consumes it unchanged. That constrains the shape more than the standard
//! does: `$value` is a plain hex string, because that is what every tool in
//! the ecosystem actually reads today. The richer object form for color in the
//! draft is not yet widely supported, so the OKLCH coordinates ride along in
//! `$extensions` instead — present for anything that wants them, invisible to
//! anything that does not.

use noctua_engine::{Palette, ResolvedStep};
use serde_json::{Map, Value, json};

use crate::{CommentStyle, EmittedFile, Emitter, header, value};

/// The DTCG token target.
#[derive(Debug, Clone, Copy)]
pub struct Dtcg;

impl Emitter for Dtcg {
    fn id(&self) -> &'static str {
        "dtcg"
    }

    fn describe(&self) -> &'static str {
        "DTCG JSON tokens, consumable by Style Dictionary unchanged"
    }

    fn emit(&self, palette: &Palette) -> Vec<EmittedFile> {
        let mut files = Vec::new();

        for theme in &palette.themes {
            for mode in &theme.modes {
                let mut root = Map::new();
                root.insert(
                    "$description".into(),
                    description(&theme.name, mode.mode.id()),
                );

                for (family, resolved) in &mode.families {
                    let mut group = Map::new();
                    group.insert("$type".into(), "color".into());
                    for step in &resolved.steps {
                        group.insert(step.role.clone(), token(step));
                    }
                    root.insert(family.clone(), Value::Object(group));
                }

                for (scale, steps) in &mode.scales {
                    let mut group = Map::new();
                    group.insert("$type".into(), "color".into());
                    for step in steps {
                        group.insert(step.role.clone(), token(step));
                    }
                    root.insert(scale.clone(), Value::Object(group));
                }

                let mut alpha = Map::new();
                alpha.insert("$type".into(), "color".into());
                for stop in crate::tokens::alpha_tokens(palette, mode) {
                    alpha.insert(
                        stop.stem(),
                        json!({
                            "$value": crate::value::hex_rgba(stop.step.primary(), stop.percentage),
                        }),
                    );
                }
                root.insert("alpha".into(), Value::Object(alpha));

                for (ramp, steps) in &palette.neutral_ramps {
                    let mut group = Map::new();
                    group.insert("$type".into(), "color".into());
                    for step in steps {
                        group.insert(step.index.to_string(), token(step));
                    }
                    root.insert(ramp.clone(), Value::Object(group));
                }

                files.push(EmittedFile::new(
                    format!("tokens/{}-{}.json", theme.name, mode.mode.id()),
                    render(&Value::Object(root)),
                ));
            }
        }

        files
    }
}

/// JSON has no comments, so the generated-file warning goes in a field the
/// format already defines for prose.
fn description(theme: &str, mode: &str) -> Value {
    let banner = header("specs/noctua.toml", CommentStyle::Line(""));
    let banner = banner.replace('\n', " ").trim().to_owned();
    json!(format!("Theme {theme}, {mode} mode. {banner}"))
}

fn token(step: &ResolvedStep) -> Value {
    let color = step.primary();
    json!({
        "$type": "color",
        "$value": value::hex(color),
        "$extensions": {
            "colors.noctua.oklch": {
                "l": round(color.oklch.l, 4),
                "c": round(color.oklch.c, 4),
                "h": round(color.oklch.h, 2),
            },
            "colors.noctua.relativeChroma": round(color.achieved_relative_chroma, 4),
        }
    })
}

fn round(value: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    (value * scale).round() / scale
}

fn render(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("tokens always serialize");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use noctua_engine::build;

    use super::*;

    fn shipped() -> Palette {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../specs/noctua.toml");
        build(&noctua_spec::load(path).expect("valid")).expect("builds")
    }

    fn parsed(name: &str) -> Value {
        let contents = Dtcg
            .emit(&shipped())
            .into_iter()
            .find(|f| f.path == name)
            .unwrap_or_else(|| panic!("{name} should be emitted"))
            .contents;
        serde_json::from_str(&contents).expect("valid JSON")
    }

    #[test]
    fn one_file_per_theme_and_mode() {
        let palette = shipped();
        let files = Dtcg.emit(&palette);
        assert_eq!(files.len(), palette.themes.len() * 2);
        assert!(
            files
                .iter()
                .any(|f| f.path == "tokens/ochre-balanced-light.json")
        );
        assert!(
            files
                .iter()
                .any(|f| f.path == "tokens/ochre-sober-dark.json")
        );
    }

    #[test]
    fn every_file_is_valid_json() {
        for file in Dtcg.emit(&shipped()) {
            serde_json::from_str::<Value>(&file.contents)
                .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", file.path));
        }
    }

    /// The requirement: an existing Style Dictionary pipeline reads this as-is.
    #[test]
    fn tokens_use_the_plain_hex_value_every_tool_understands() {
        let tokens = parsed("tokens/ochre-balanced-light.json");
        let solid = &tokens["accent"]["solid"];
        assert_eq!(solid["$type"], "color");
        let hex = solid["$value"].as_str().expect("a string value");
        assert!(hex.starts_with('#') && hex.len() == 7, "got {hex}");
    }

    #[test]
    fn the_oklch_coordinates_ride_along_without_getting_in_the_way() {
        let tokens = parsed("tokens/ochre-balanced-light.json");
        let extensions = &tokens["accent"]["solid"]["$extensions"];
        assert!(extensions["colors.noctua.oklch"]["l"].is_number());
        assert!(extensions["colors.noctua.oklch"]["c"].is_number());
        assert!(extensions["colors.noctua.oklch"]["h"].is_number());
        assert!(extensions["colors.noctua.relativeChroma"].is_number());
    }

    #[test]
    fn the_generated_warning_survives_a_format_without_comments() {
        let tokens = parsed("tokens/ochre-balanced-light.json");
        let description = tokens["$description"].as_str().expect("a description");
        assert!(description.contains("do not edit"), "{description}");
        assert!(description.contains(crate::REGENERATE), "{description}");
    }

    #[test]
    fn groups_cover_families_the_chart_and_the_ramp() {
        let tokens = parsed("tokens/ochre-balanced-dark.json");
        assert!(tokens["accent"]["solid"].is_object());
        assert!(tokens["neutral"]["bg-app"].is_object());
        assert!(tokens["chart"]["1"].is_object());
        assert!(tokens["gray"]["24"].is_object());
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(Dtcg.emit(&shipped()), Dtcg.emit(&shipped()));
    }
}
