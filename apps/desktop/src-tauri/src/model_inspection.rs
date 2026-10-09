//! Read exact CAD properties for any saved STEP revision in the project.
#[cfg(feature = "native-occt")]
use crate::core::AppError;
use crate::core::{AppState, Result};
#[cfg(feature = "native-occt")]
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tauri::State;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExactModelProperties {
    pub volume_mm3: f64,
    pub area_mm2: f64,
    pub face_count: i32,
    pub edge_count: i32,
    pub bounds_mm: [f64; 3],
}

#[tauri::command]
pub async fn inspect_model(
    project_id: String,
    revision_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ExactModelProperties>> {
    #[cfg(not(feature = "native-occt"))]
    {
        let _ = (project_id, revision_id, state);
        Ok(None)
    }
    #[cfg(feature = "native-occt")]
    {
        let project = crate::projects::get(&state, &project_id).await?;
        let revision = project
            .revisions
            .iter()
            .find(|entry| entry.id == revision_id)
            .ok_or_else(|| AppError::Invalid("CAD revision is missing".into()))?;
        let source = match revision.source.as_deref() {
            Some(source)
                if Path::new(source)
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("step") || ext.eq_ignore_ascii_case("stp")
                    }) =>
            {
                source
            }
            _ => return Ok(None),
        };
        let file = project
            .files
            .iter()
            .find(|file| file.name == source)
            .ok_or_else(|| AppError::Invalid("Saved STEP model is missing".into()))?;
        let bytes = crate::artifacts::read(&state, &project_id, file)?;
        let cancel = Arc::new(AtomicBool::new(false));
        let _cad = state
            .cad_tasks
            .acquire(&project_id, "inspection", cancel.clone())
            .await?;
        let workspace = crate::project_access::transient::Workspace::create(&state.root)?;
        let cwd = &workspace.path;
        crate::artifacts::immutable_write(cwd, Path::new("source.step"), &bytes)?;
        let result = crate::native::inspection::inspect_step(cwd, cancel).await;
        result.map(|properties| {
            Some(ExactModelProperties {
                volume_mm3: properties.volume_mm3,
                area_mm2: properties.area_mm2,
                face_count: properties.face_count,
                edge_count: properties.edge_count,
                bounds_mm: properties.bounds_mm,
            })
        })
    }
}
