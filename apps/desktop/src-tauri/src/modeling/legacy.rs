//! Compatibility execution path for existing CadQuery programs and CAD JSON v1.
use crate::{
    cad_document::CompiledDocument,
    core::{AppError, Result},
    processes::{self, CommandSpec},
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

pub async fn build(
    root: &Path,
    cwd: &Path,
    program: &str,
    document: Option<&CompiledDocument>,
    has_base: bool,
    cancel: Arc<AtomicBool>,
) -> Result<bool> {
    let python = processes::python(root)
        .ok_or_else(|| AppError::Invalid("Install the local CadQuery environment first".into()))?;
    let build_output = processes::run(
        &CommandSpec {
            executable: python.clone(),
            args: vec![
                "-I".into(),
                "-c".into(),
                include_str!("../../scripts/model_program.py").into(),
            ],
            cwd: cwd.to_path_buf(),
        },
        &serde_json::json!({
            "program": document.map(|d| d.source.as_str()).unwrap_or(program),
            "features": document.map(|d| &d.lines),
            "hasBase": has_base && document.is_none()
        })
        .to_string(),
        cancel.clone(),
        Duration::from_secs(120),
    )
    .await?;
    let used_base =
        serde_json::from_str::<serde_json::Value>(build_output.trim())?["usesBase"] == true;
    processes::run(
        &CommandSpec {
            executable: python,
            args: vec![
                "-I".into(),
                "-c".into(),
                include_str!("../../scripts/convert_step.py").into(),
            ],
            cwd: cwd.to_path_buf(),
        },
        r#"{"source":"model.step","output":"preview.glb","parts":true}"#,
        cancel,
        Duration::from_secs(120),
    )
    .await?;
    Ok(used_base)
}
