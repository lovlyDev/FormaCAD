//! Shared real CAD execution; durable project storage is supplied by the host.
use super::{build::GeneratedModel, legacy, program::PreparedProgram, workspace::Workspace};
use crate::{
    agents::review_plan::ReviewPlan,
    cad_ir::Document,
    core::{AppError, Result},
    models::{Project, ProjectFile},
};
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

/// Trusted host paths. Neither root nor executable is accepted from IPC.
pub(crate) struct BuildRuntime<'a> {
    pub runtime_root: &'a Path,
    pub transient_root: &'a Path,
    pub executable: Option<&'a Path>,
}

/// A host reader of descriptors from the captured project. Folder hosts must
/// bind this reader to their original snapshot and bounded immutable CAS.
/// Lifecycle failures propagate unchanged, never becoming missing/checksum errors.
pub(crate) trait AssetSource: Send {
    fn read(&mut self, file: &ProjectFile) -> Result<Vec<u8>>;
    #[cfg(feature = "native-worker-client")]
    fn stage_native(&mut self, project: &Project, document: &Document, cwd: &Path) -> Result<()> {
        crate::native::assets::stage_assets_with(project, document, cwd, |file| self.read(file))
    }
}

impl<F> AssetSource for F
where
    F: FnMut(&ProjectFile) -> Result<Vec<u8>> + Send,
{
    fn read(&mut self, file: &ProjectFile) -> Result<Vec<u8>> {
        self(file)
    }
}

fn stage_base(
    original: &Project,
    prepared: &PreparedProgram,
    cwd: &Path,
    assets: &mut dyn AssetSource,
) -> Result<Option<String>> {
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
        let bytes = assets.read(file)?;
        crate::artifacts::immutable_write(cwd, Path::new("base.step"), &bytes)?;
    }
    Ok(source.cloned())
}

#[cfg(feature = "native-worker-client")]
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

pub(crate) async fn build(
    runtime: BuildRuntime<'_>,
    assets: &mut dyn AssetSource,
    original: &Project,
    program: &str,
    prepared: &PreparedProgram,
    review: Option<(&ReviewPlan, &Document)>,
    cancel: Arc<AtomicBool>,
) -> Result<GeneratedModel> {
    let workspace = Workspace::create(runtime.transient_root, &original.id)?;
    let cwd = &workspace.path;
    let source = stage_base(original, prepared, cwd, assets)?;
    #[cfg(feature = "native-worker-client")]
    let document = native_document(program, prepared)?;
    #[cfg(feature = "native-worker-client")]
    if let Some(document) = &document {
        assets.stage_native(original, document, cwd)?;
    }
    #[cfg(feature = "native-worker-client")]
    let native = if document.is_none() {
        false
    } else if let Some(executable) = runtime.executable {
        crate::native::worker::build_with_executable(program, cwd, cancel.clone(), executable)
            .await?
    } else {
        crate::native::worker::build_if_ir_v2(program, cwd, cancel.clone()).await?
    };
    #[cfg(not(feature = "native-worker-client"))]
    let native = {
        let _ = runtime.executable;
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
            runtime.runtime_root,
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
        program_base: if used_base { source } else { None },
    })
}

#[cfg(all(test, feature = "native-worker-client"))]
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

#[cfg(test)]
#[path = "execution/source_tests.rs"]
mod source_tests;
