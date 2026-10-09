//! Host verification runs before any selected geometry is transmitted to an AI CLI.
use super::{topology, SelectionContext};
use crate::{
    core::{AppError, AppState, Result},
    models::{Project, ProjectFile, Revision},
};
#[cfg(feature = "native-occt")]
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc};

fn revision<'a>(project: &'a Project, selection: &SelectionContext) -> Result<&'a Revision> {
    selection.validate(project.current_revision.as_deref())?;
    project
        .revisions
        .iter()
        .find(|entry| entry.id == selection.revision_id)
        .ok_or_else(topology::invalid)
}

fn sealed_bytes(state: &AppState, project: &Project, name: Option<&str>) -> Result<Vec<u8>> {
    let file = project
        .files
        .iter()
        .find(|file| Some(file.name.as_str()) == name)
        .ok_or_else(topology::invalid)?;
    if file.size == 0 || file.size > crate::artifacts::MAX_FILE_BYTES as u64 {
        return Err(topology::invalid());
    }
    if file.data.is_none() {
        let hash = file
            .sha256
            .as_deref()
            .filter(|hash| crate::artifacts::valid_digest(hash))
            .ok_or_else(topology::invalid)?;
        let extension = std::path::Path::new(&file.name)
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(topology::invalid)?
            .to_ascii_lowercase();
        let path = crate::security::guarded(
            &state.root,
            std::path::Path::new(&format!("{}/attachments/{hash}.{extension}", project.id)),
        )?;
        if std::fs::metadata(path)?.len() != file.size {
            return Err(topology::invalid());
        }
    }
    let bytes = crate::artifacts::read(state, &project.id, file)?;
    verify_seal(file, &bytes)?;
    Ok(bytes)
}

fn verify_seal(file: &ProjectFile, bytes: &[u8]) -> Result<()> {
    if bytes.is_empty()
        || bytes.len() > crate::artifacts::MAX_FILE_BYTES
        || file.size != bytes.len() as u64
        || file.sha256.as_deref() != Some(crate::artifacts::digest(bytes).as_str())
    {
        return Err(topology::invalid());
    }
    Ok(())
}

pub async fn recheck(
    state: &AppState,
    captured: &Project,
    selected: &SelectionContext,
) -> Result<()> {
    let latest = crate::projects::get(state, &captured.id).await?;
    if revision(&latest, selected)? != revision(captured, selected)? {
        return Err(topology::invalid());
    }
    let rev = revision(captured, selected)?;
    for name in [rev.source.as_deref(), rev.preview.as_deref()] {
        if sealed_bytes(state, &latest, name)? != sealed_bytes(state, captured, name)? {
            return Err(topology::invalid());
        }
    }
    if let Some(program) = rev.program.as_deref() {
        if let Ok(document) = serde_json::from_str::<crate::cad_ir::Document>(program) {
            for (_, hash) in
                crate::cad_ir::assets::references(&document).map_err(|_| topology::invalid())?
            {
                let file = latest
                    .files
                    .iter()
                    .find(|file| file.sha256.as_deref() == Some(hash))
                    .ok_or_else(topology::invalid)?;
                sealed_bytes(state, &latest, Some(&file.name))?;
            }
        }
    }
    Ok(())
}

pub async fn validate(
    state: &AppState,
    project: &Project,
    selected: &SelectionContext,
    cancel: Arc<AtomicBool>,
) -> Result<()> {
    #[cfg(not(feature = "native-occt"))]
    {
        let _ = (state, project, selected, cancel);
        Err(AppError::Invalid("SELECTION_CONTEXT_UNSUPPORTED".into()))
    }
    #[cfg(feature = "native-occt")]
    {
        let executable = crate::native::worker::worker_executable()?;
        validate_with_executable(state, project, selected, cancel, &executable).await
    }
}

/// Injectable isolated worker path for native integration tests. Never supplied by IPC.
#[cfg(feature = "native-occt")]
pub async fn validate_with_executable(
    state: &AppState,
    project: &Project,
    selected: &SelectionContext,
    cancel: Arc<AtomicBool>,
    executable: &std::path::Path,
) -> Result<()> {
    let rev = revision(project, selected)?;
    if !rev.source.as_deref().is_some_and(|name| {
        std::path::Path::new(name)
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("step") || ext.eq_ignore_ascii_case("stp"))
    }) {
        return Err(topology::invalid());
    }
    let source = rev
        .program
        .as_deref()
        .ok_or_else(|| AppError::Invalid("SELECTION_CONTEXT_UNSUPPORTED".into()))?;
    let document: crate::cad_ir::Document = serde_json::from_str(source)
        .map_err(|_| AppError::Invalid("SELECTION_CONTEXT_UNSUPPORTED".into()))?;
    document.validate().map_err(|_| topology::invalid())?;
    if document.revision_id != selected.revision_id
        || !document
            .bodies
            .iter()
            .any(|body| body.id == selected.body_id)
    {
        return Err(topology::invalid());
    }
    sealed_bytes(state, project, rev.source.as_deref())?;
    let saved = topology::body_topology(
        &sealed_bytes(state, project, rev.preview.as_deref())?,
        &selected.body_id,
    )?;
    let _slot = state
        .cad_tasks
        .acquire(&project.id, "selection_validation", cancel.clone())
        .await?;
    recheck(state, project, selected).await?;
    let workspace = crate::project_access::transient::Workspace::create(&state.root)?;
    crate::native::assets::stage_project_assets(state, project, &document, &workspace.path)?;
    if !crate::native::worker::build_selected_with_executable(
        source,
        &selected.body_id,
        &workspace.path,
        cancel.clone(),
        executable,
    )
    .await?
    {
        return Err(topology::invalid());
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Invalid("Task cancelled".into()));
    }
    let fresh = topology::body_topology(
        &std::fs::read(workspace.path.join("preview.glb"))?,
        &selected.body_id,
    )?;
    topology::verify(selected, &fresh, &saved)?;
    recheck(state, project, selected).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_and_disk_assets_require_the_exact_committed_checksum_and_size() {
        let bytes = b"ISO-10303-21; verified source";
        let mut file = ProjectFile {
            name: "model.step".into(),
            kind: "model".into(),
            size: bytes.len() as u64,
            data: None,
            sha256: Some(crate::artifacts::digest(bytes)),
        };
        assert!(verify_seal(&file, bytes).is_ok());
        assert!(verify_seal(&file, b"ISO-10303-21; tampered source").is_err());
        file.size += 1;
        assert!(verify_seal(&file, bytes).is_err());
        file.size -= 1;
        file.sha256 = None;
        assert!(verify_seal(&file, bytes).is_err());
    }
}
