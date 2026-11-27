use super::{creation::FolderCreationReply, WindowSources};
use crate::core::{AppError, AppState, Result};
use tauri::State;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StepFolderDraft {
    name: String,
    agent: String,
    prompt: String,
}

#[tauri::command]
pub async fn transfer_step_to_folder(
    draft: StepFolderDraft,
    window: tauri::WebviewWindow,
    sources: State<'_, WindowSources>,
    state: State<'_, AppState>,
) -> Result<Option<FolderCreationReply>> {
    let StepFolderDraft {
        name,
        agent,
        prompt,
    } = draft;
    #[cfg(all(
        feature = "native-worker-client",
        any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        )
    ))]
    {
        use super::pick_folder;
        use crate::project_repository::sqlite_folder::api::{
            ModelRuntime, SelectedStepFile, StepProjectIntent,
        };
        use tauri::Manager;
        use tauri_plugin_dialog::DialogExt;

        let scope = sources.for_window(window.label())?;
        let app = window.app_handle().clone();
        let selected = tokio::task::spawn_blocking(move || {
            let Some(file) = app
                .dialog()
                .file()
                .set_title("Forma CAD — STEP")
                .add_filter("STEP", &["step", "stp"])
                .blocking_pick_file()
            else {
                return Ok::<_, AppError>(None);
            };
            let path = file
                .into_path()
                .map_err(|error| AppError::Invalid(error.to_string()))?;
            let source = SelectedStepFile::open(&path)?;
            let original = source.capture()?;
            Ok(Some((source, original)))
        })
        .await
        .map_err(|error| AppError::Invalid(error.to_string()))??;
        let Some((source, original)) = selected else {
            return Ok(None);
        };
        let Some(destination) = pick_folder(&window, None).await? else {
            return Ok(None);
        };
        let result = scope
            .transfer_selected_step(
                &source,
                &original,
                &destination,
                StepProjectIntent {
                    name: name.trim().to_owned(),
                    agent,
                    prompt,
                },
                ModelRuntime {
                    tasks: &state.tasks,
                    gate: &state.cad_tasks,
                    runtime_root: &state.root,
                    transient_root: &state.root,
                    executable: None,
                },
            )
            .await?;
        Ok(Some(result.into()))
    }
    #[cfg(not(all(
        feature = "native-worker-client",
        any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        )
    )))]
    {
        let _ = (name, agent, prompt, window, sources, state);
        Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()))
    }
}
