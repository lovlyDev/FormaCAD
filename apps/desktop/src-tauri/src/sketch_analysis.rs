//! Bounded sketch-only analysis: no CAD kernel, database writes or project revision changes.
use crate::{
    cad_ir::{
        sketch::analysis::{analyze_bound, SketchAnalysis},
        Operation, Parameter,
    },
    core::{AppError, Result},
};
use std::collections::HashMap;

pub fn analyze_operation(
    operation: Operation,
    parameters: Vec<Parameter>,
    feature_id: &str,
) -> Result<SketchAnalysis> {
    if parameters.len() > 10000
        || parameters
            .iter()
            .any(|p| !p.value_mm.is_finite() || p.value_mm.abs() > 10000.0)
    {
        return Err(AppError::Invalid("Sketch analysis input is invalid".into()));
    }
    let values: HashMap<_, _> = parameters
        .iter()
        .map(|p| (p.id.as_str(), p.value_mm))
        .collect();
    if values.len() != parameters.len() {
        return Err(AppError::Invalid("Sketch analysis input is invalid".into()));
    }
    match operation {
        Operation::Sketch2d {
            plane,
            origin_mm,
            points,
            lines,
            constraints,
            bindings,
        } => Ok(analyze_bound(
            plane,
            origin_mm,
            &points,
            &lines,
            &constraints,
            &bindings,
            &values,
            feature_id,
        )),
        _ => Err(AppError::Invalid("Sketch analysis input is invalid".into())),
    }
}

#[tauri::command]
pub async fn analyze_sketch(
    operation: Operation,
    parameters: Vec<Parameter>,
    feature_id: String,
) -> Result<SketchAnalysis> {
    tokio::task::spawn_blocking(move || analyze_operation(operation, parameters, &feature_id))
        .await
        .map_err(|_| AppError::Invalid("Sketch analysis failed".into()))?
}
