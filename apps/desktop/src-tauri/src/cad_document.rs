//! Forma's versioned modeling language. No viewport or kernel objects cross this boundary.
use crate::core::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CadDocument {
    pub version: u32,
    pub features: Vec<Feature>,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Feature {
    pub id: String,
    pub operation: Operation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Operation {
    Rectangle {
        width: f64,
        depth: f64,
    },
    Circle {
        radius: f64,
    },
    Extrude {
        sketch: String,
        distance: f64,
    },
    Translate {
        body: String,
        offset: [f64; 3],
    },
    Boolean {
        left: String,
        right: String,
        mode: BooleanMode,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BooleanMode {
    Union,
    Cut,
    Intersect,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureLine {
    pub feature_id: String,
    pub operation: String,
    pub solid: bool,
}

pub struct CompiledDocument {
    pub source: String,
    pub lines: Vec<FeatureLine>,
}

fn error(id: &str, code: &str, message: &str) -> AppError {
    AppError::Invalid(
        serde_json::json!({
            "featureId": id, "code": code, "message": message
        })
        .to_string(),
    )
}

impl CadDocument {
    pub fn parse(source: &str) -> Result<Self> {
        serde_json::from_str(source).map_err(|e| error("", "INVALID_DOCUMENT", &e.to_string()))
    }

    pub fn compile(&self) -> Result<CompiledDocument> {
        if self.version != 1 || self.features.is_empty() || self.features.len() > 128 {
            return Err(error(
                "",
                "INVALID_DOCUMENT",
                "Expected version 1 and 1..128 features",
            ));
        }
        // Sketch expressions are replayed for each consumer: CadQuery pending wires are mutable.
        let mut values: HashMap<&str, (String, bool)> = HashMap::new();
        let mut source = String::new();
        let mut lines = Vec::new();
        for (index, feature) in self.features.iter().enumerate() {
            let id = feature.id.as_str();
            if id.is_empty()
                || id.len() > 80
                || !id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
                || values.contains_key(id)
            {
                return Err(error(
                    id,
                    "INVALID_FEATURE_ID",
                    "Feature IDs must be unique ASCII names",
                ));
            }
            let dimension = |v: f64| -> Result<()> {
                if !v.is_finite() || v <= 0.0 || v > 10000.0 {
                    return Err(error(
                        id,
                        "INVALID_PARAMETER",
                        "Dimensions must be finite, positive and at most 10000 mm",
                    ));
                }
                Ok(())
            };
            let reference = |name: &str, solid: bool| -> Result<String> {
                match values.get(name) {
                    Some((value, kind)) if *kind == solid => Ok(value.clone()),
                    Some(_) => Err(error(id, "REFERENCE_TYPE_MISMATCH", name)),
                    None => Err(error(id, "BROKEN_REFERENCE", "References must name an earlier feature; cycles and forward references are not supported")),
                }
            };
            let (expression, solid, operation) = match &feature.operation {
                Operation::Rectangle { width, depth } => {
                    dimension(*width)?;
                    dimension(*depth)?;
                    (
                        format!("cq.Workplane('XY').rect({width},{depth})"),
                        false,
                        "rectangle",
                    )
                }
                Operation::Circle { radius } => {
                    dimension(*radius)?;
                    (
                        format!("cq.Workplane('XY').circle({radius})"),
                        false,
                        "circle",
                    )
                }
                Operation::Extrude { sketch, distance } => {
                    dimension(distance.abs())?;
                    (
                        format!("{}.extrude({distance})", reference(sketch, false)?),
                        true,
                        "extrude",
                    )
                }
                Operation::Translate { body, offset } => {
                    if offset.iter().any(|v| !v.is_finite() || v.abs() > 10000.0) {
                        return Err(error(
                            id,
                            "INVALID_PARAMETER",
                            "Translation exceeds supported bounds",
                        ));
                    }
                    (
                        format!(
                            "{}.translate(({},{},{}))",
                            reference(body, true)?,
                            offset[0],
                            offset[1],
                            offset[2]
                        ),
                        true,
                        "translate",
                    )
                }
                Operation::Boolean { left, right, mode } => {
                    let method = match mode {
                        BooleanMode::Union => "union",
                        BooleanMode::Cut => "cut",
                        BooleanMode::Intersect => "intersect",
                    };
                    (
                        format!(
                            "{}.{method}({})",
                            reference(left, true)?,
                            reference(right, true)?
                        ),
                        true,
                        "boolean",
                    )
                }
            };
            let variable = format!("feature_{index}");
            source.push_str(&format!("{variable} = {expression}\n"));
            values.insert(id, (if solid { variable } else { expression }, solid));
            lines.push(FeatureLine {
                feature_id: id.into(),
                operation: operation.into(),
                solid,
            });
        }
        let Some((output, true)) = values.get(self.output.as_str()) else {
            return Err(error(
                &self.output,
                "INVALID_OUTPUT",
                "Output must reference a solid feature",
            ));
        };
        source.push_str(&format!("result = {output}\n"));
        if source.len() > 60000 {
            return Err(error(
                "",
                "DOCUMENT_TOO_LARGE",
                "Compiled geometry exceeds worker limit",
            ));
        }
        Ok(CompiledDocument { source, lines })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn document() -> CadDocument {
        CadDocument::parse(include_str!(
            "../../../../docs/fixtures/plate-hole.cad.json"
        ))
        .unwrap()
    }
    #[test]
    fn regeneration_is_deterministic_and_parameters_propagate() {
        let mut doc = document();
        assert_eq!(doc.compile().unwrap().source, doc.compile().unwrap().source);
        doc.features[0].operation = Operation::Rectangle {
            width: 80.0,
            depth: 40.0,
        };
        assert!(doc
            .compile()
            .unwrap()
            .source
            .contains("rect(80,40).extrude(10)"));
    }
    #[test]
    fn rejects_broken_references_duplicate_ids_and_invalid_outputs() {
        let mut doc = document();
        doc.features.swap(0, 1);
        assert!(doc
            .compile()
            .err()
            .unwrap()
            .to_string()
            .contains("BROKEN_REFERENCE"));
        let mut doc = document();
        doc.features[1].id = doc.features[0].id.clone();
        assert!(doc
            .compile()
            .err()
            .unwrap()
            .to_string()
            .contains("INVALID_FEATURE_ID"));
        let mut doc = document();
        doc.output = doc.features[0].id.clone();
        assert!(doc
            .compile()
            .err()
            .unwrap()
            .to_string()
            .contains("INVALID_OUTPUT"));
    }
    #[test]
    fn rejects_unknown_fields_and_nonfinite_dimensions() {
        assert!(
            CadDocument::parse(r#"{"version":1,"features":[],"output":"x","python":"bad"}"#)
                .is_err()
        );
        let mut doc = document();
        doc.features[0].operation = Operation::Circle { radius: f64::NAN };
        assert!(doc.compile().is_err());
    }
    #[test]
    fn compiled_document_builds_real_geometry() {
        // Opt-in kernel contract test, also runs the production interpreter and STEP export.
        let Ok(python) = std::env::var("FORMA_TEST_PYTHON") else {
            return;
        };
        use std::{
            io::Write,
            process::{Command, Stdio},
        };
        let compiled = document().compile().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let mut child = Command::new(python)
            .args(["-I", "-c", include_str!("../scripts/model_program.py")])
            .current_dir(dir.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(
                serde_json::json!({"program":compiled.source,"features":compiled.lines})
                    .to_string()
                    .as_bytes(),
            )
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let expected = 60.0 * 40.0 * 10.0 - std::f64::consts::PI * 25.0 * 10.0;
        assert!((report["volume"].as_f64().unwrap() - expected).abs() < 0.001);
        assert!(dir.path().join("model.step").is_file());
    }
}
