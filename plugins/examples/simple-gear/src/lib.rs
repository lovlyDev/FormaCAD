//! Example FormaCAD plugin: a cog built only from regular CAD IR operations
//! (circle, rectangle, extrude, rotate, boolean union, hole). Because the
//! result is ordinary CAD IR, the body stays visible, measurable and
//! exportable even without the plugin.
wit_bindgen::generate!({
    path: ["../../../docs/plugin-api/wit", "wit"],
    world: "example:simple-gear/simple-gear",
    generate_all,
});

use exports::formacad::plugin::feature::Guest;
use formacad::plugin::{assets, log, notifications, types::{Level, PluginError}};
use serde_json::{json, Value};

struct Plugin;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Params {
    teeth: u32,
    root_radius: f64,
    tooth_length: f64,
    tooth_width: f64,
    thickness: f64,
    bore: f64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Context {
    locale: String,
    id_prefix: String,
}

fn error(code: &str, message: &str) -> PluginError {
    PluginError { code: code.into(), message: message.into() }
}

/// Plugin strings from the package's own `locales/<lang>.json`.
fn strings(locale: &str) -> Value {
    let file = if locale == "en" { "locales/en.json" } else { "locales/ru.json" };
    assets::read(file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(Value::Null)
}

fn literal(mm: f64) -> Value {
    json!({ "kind": "literal", "mm": mm })
}

/// Pure geometry: returns Command API commands for the gear.
fn gear_commands(p: &Params, prefix: &str, text: &Value) -> Result<Vec<Value>, PluginError> {
    let label = |key: &str| text[key].as_str().unwrap_or(key).to_owned();
    if !(6..=48).contains(&p.teeth) {
        return Err(error("TEETH_RANGE", "Teeth must be between 6 and 48"));
    }
    if p.bore > 0.0 && p.bore / 2.0 >= p.root_radius * 0.9 {
        return Err(error("BORE_TOO_LARGE", "The bore must be smaller than the gear root"));
    }
    // Teeth overlap the disc a little so every union is a proper solid union.
    let overlap = (p.root_radius / 4.0).min(1.5);
    let radial = p.tooth_length + overlap;
    let id = |name: &str| format!("{prefix}{name}");
    let feature = |name: &str, title: String, operation: Value| {
        json!({ "command": "add_feature", "feature": { "id": id(name), "name": title, "operation": operation } })
    };
    let mut commands = vec![
        feature("disc", label("disc"), json!({ "type": "circle", "radius": literal(p.root_radius) })),
        feature("disc_pad", label("discPad"), json!({ "type": "extrude", "sketchId": id("disc"), "distance": literal(p.thickness) })),
        feature("tooth_profile", label("tooth"), json!({ "type": "rectangle", "width": literal(radial), "depth": literal(p.tooth_width) })),
        feature("tooth_pad", label("tooth"), json!({ "type": "extrude", "sketchId": id("tooth_profile"), "distance": literal(p.thickness) })),
        feature("tooth", label("tooth"), json!({
            "type": "translate", "bodyFeatureId": id("tooth_pad"),
            "offsetMm": [p.root_radius + p.tooth_length - radial / 2.0, 0.0, 0.0]
        })),
    ];
    let mut previous = id("disc_pad");
    for i in 0..p.teeth {
        let (rotated, fused) = (format!("tooth_{i}"), format!("fuse_{i}"));
        commands.push(feature(&rotated, format!("{} {}", label("tooth"), i + 1), json!({
            "type": "rotate", "bodyFeatureId": id("tooth"), "axisOriginMm": [0.0, 0.0, 0.0],
            "axisDirection": [0.0, 0.0, 1.0], "angleDeg": f64::from(i) * 360.0 / f64::from(p.teeth)
        })));
        commands.push(feature(&fused, label("union"), json!({
            "type": "boolean", "leftFeatureId": previous, "rightFeatureId": id(&rotated), "mode": "union"
        })));
        previous = id(&fused);
    }
    if p.bore > 0.0 {
        commands.push(feature("bore", label("hole"), json!({
            "type": "hole", "bodyFeatureId": previous, "centerMm": [0.0, 0.0], "startZMm": 0.0,
            "radius": literal(p.bore / 2.0), "depth": literal(p.thickness)
        })));
        previous = id("bore");
    }
    commands.push(json!({ "command": "add_body", "body": { "id": id("body"), "name": label("body"), "sourceFeatureId": previous } }));
    Ok(commands)
}

impl Guest for Plugin {
    fn build(feature_id: String, params_json: String, context_json: String) -> Result<String, PluginError> {
        if feature_id != "simpleGear" {
            return Err(error("UNKNOWN_FEATURE", "Unknown feature"));
        }
        let params: Params = serde_json::from_str(&params_json).map_err(|e| error("BAD_PARAMS", &e.to_string()))?;
        let context: Context = serde_json::from_str(&context_json).map_err(|e| error("BAD_CONTEXT", &e.to_string()))?;
        let text = strings(&context.locale);
        let commands = gear_commands(&params, &context.id_prefix, &text)?;
        log::write(Level::Info, &format!("{} commands for {} teeth", commands.len(), params.teeth));
        let built = text["built"].as_str().unwrap_or("Gear added").replace("{teeth}", &params.teeth.to_string());
        notifications::notify(Level::Info, &built);
        serde_json::to_string(&commands).map_err(|e| error("ENCODE", &e.to_string()))
    }
}

export!(Plugin);
