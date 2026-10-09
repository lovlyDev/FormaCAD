use super::{legacy, program::PreparedProgram, workspace::Workspace};
use crate::{
    agents::review_plan::ReviewPlan,
    cad_ir::Document,
    core::{AppError, AppState, Result},
    models::Project,
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

pub(super) struct GeneratedModel {
    pub step: Vec<u8>,
    pub preview: Vec<u8>,
    pub program_base: Option<String>,
}

#[cfg(feature = "native-occt")]
fn native_document(program: &str, prepared: &PreparedProgram) -> Result<Option<Document>> {
    if !prepared.typed {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_str(program)?;
    if value["schemaVersion"] == 2 {
        Ok(Some(serde_json::from_value(value)?))
    } else {
        Ok(None)
    }
}

fn bounded_output(cwd: &Path, name: &str, error: &str) -> Result<Vec<u8>> {
    let path = crate::security::guarded(cwd, Path::new(name))?;
    if std::fs::metadata(&path)?.len() > crate::artifacts::MAX_FILE_BYTES as u64 {
        return Err(AppError::Invalid(error.into()));
    }
    Ok(std::fs::read(path)?)
}

pub(super) async fn build(
    state: &AppState,
    original: &Project,
    program: &str,
    prepared: &PreparedProgram,
    review: Option<(&ReviewPlan, &Document)>,
    cancel: Arc<AtomicBool>,
    executable: Option<&Path>,
) -> Result<GeneratedModel> {
    let workspace = Workspace::create(&state.root, &original.id)?;
    let cwd = &workspace.path;
    let current = original
        .revisions
        .iter()
        .find(|revision| Some(&revision.id) == original.current_revision.as_ref());
    let source = current
        .filter(|_| !prepared.typed)
        .and_then(|revision| revision.program_base.as_ref().or(revision.source.as_ref()))
        .filter(|name| {
            name.to_lowercase().ends_with(".step") || name.to_lowercase().ends_with(".stp")
        });
    if let Some(name) = source {
        let file = original
            .files
            .iter()
            .find(|file| &file.name == name)
            .ok_or_else(|| AppError::Invalid("Base STEP is missing".into()))?;
        let bytes = crate::artifacts::read(state, &original.id, file)?;
        crate::artifacts::immutable_write(cwd, Path::new("base.step"), &bytes)?;
    }
    #[cfg(feature = "native-occt")]
    let document = native_document(program, prepared)?;
    #[cfg(feature = "native-occt")]
    if let Some(document) = &document {
        crate::native::assets::stage_project_assets(state, original, document, cwd)?;
    }
    #[cfg(feature = "native-occt")]
    let native = if document.is_none() {
        false
    } else if let Some(executable) = executable {
        crate::native::worker::build_with_executable(program, cwd, cancel.clone(), executable)
            .await?
    } else {
        crate::native::worker::build_if_ir_v2(program, cwd, cancel.clone()).await?
    };
    #[cfg(not(feature = "native-occt"))]
    let native = {
        let _ = executable;
        false
    };
    if let Some((review, document)) = review {
        if !native {
            return Err(AppError::Invalid("AI review plan is invalid.".into()));
        }
        let path = crate::security::guarded(cwd, Path::new("result.json"))?;
        if std::fs::metadata(&path)?.len() > 65536 {
            return Err(AppError::Invalid("CAD worker response is too large".into()));
        }
        crate::agents::review_plan::check_metrics(
            review,
            document,
            &serde_json::from_slice(&std::fs::read(path)?)?,
        )?;
    }
    let used_base = if native {
        false
    } else {
        legacy::build(
            &state.root,
            cwd,
            program,
            prepared.compiled_legacy.as_ref(),
            source.is_some(),
            cancel,
        )
        .await?
    };
    let preview = bounded_output(cwd, "preview.glb", "Converted preview exceeds 40 MB")?;
    let step = bounded_output(cwd, "model.step", "Model STEP exceeds 40 MB")?;
    Ok(GeneratedModel {
        step,
        preview,
        program_base: if used_base { source.cloned() } else { None },
    })
}

#[cfg(all(test, feature = "native-occt"))]
mod tests {
    use super::*;
    #[test]
    fn typed_v1_routes_to_compiled_legacy_without_native_asset_deserialization() {
        let source = r#"{"version":1,"features":[{"id":"profile","operation":{"type":"rectangle","width":40,"depth":20}},{"id":"pad","operation":{"type":"extrude","sketch":"profile","distance":10}}],"output":"pad"}"#;
        let prepared = super::super::program::prepare(source).unwrap();
        assert!(prepared.typed);
        assert!(prepared
            .compiled_legacy
            .as_ref()
            .unwrap()
            .source
            .contains("rect(40,20).extrude(10)"));
        assert!(native_document(source, &prepared).unwrap().is_none());
        let native = serde_json::json!({"schemaVersion":2,"revisionId":"candidate","parameters":[],"features":[
            {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40},"depth":{"kind":"literal","mm":20}}},
            {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
        ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]}).to_string();
        let prepared = super::super::program::prepare(&native).unwrap();
        assert!(prepared.compiled_legacy.is_none());
        assert!(native_document(&native, &prepared).unwrap().is_some());
    }
}
