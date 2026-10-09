use super::protocol::public_event;
use super::selection::SelectionContext;
use super::PlanResult;
#[cfg(not(feature = "native-occt"))]
use super::{check_cad, parse_output};
use crate::{
    core::{AppError, AppState, Result},
    processes,
};
#[cfg(not(feature = "native-occt"))]
use std::time::Duration;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::{Emitter, State};
#[tauri::command]
pub async fn plan_model(
    project_id: String,
    prompt: String,
    attachment_names: Option<Vec<String>>,
    selection: Option<SelectionContext>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<PlanResult> {
    if prompt.trim().is_empty() || prompt.len() > 16000 {
        return Err(AppError::Invalid(
            "Prompt must contain between 1 and 16000 characters".into(),
        ));
    }
    let project = crate::projects::get(&state, &project_id).await?;
    if let Some(selected) = &selection {
        selected.validate(project.current_revision.as_deref())?;
    }
    let image_names = attachment_names.unwrap_or_default();
    if image_names.len() > 4 {
        return Err(AppError::Invalid(
            "Прикрепите не более четырёх изображений".into(),
        ));
    }
    if !image_names.is_empty() && project.agent != "codex" {
        return Err(AppError::Invalid(
            "Для моделирования по изображению выберите OpenAI Codex".into(),
        ));
    }
    let mut images = Vec::new();
    for name in &image_names {
        let file = project
            .files
            .iter()
            .find(|f| &f.name == name)
            .ok_or_else(|| AppError::Invalid("Изображение не сохранено в проекте".into()))?;
        let ext = Path::new(name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if file.kind != "image"
            || !["png", "jpg", "jpeg", "webp"].contains(&ext.as_str())
            || file.size > 20 * 1024 * 1024
        {
            return Err(AppError::Invalid("Используйте PNG, JPEG или WebP до 20 МБ. PDF/DXF нужно сначала сохранить как изображение.".into()));
        }
        images.push((ext, crate::artifacts::read(&state, &project_id, file)?));
    }
    #[cfg(feature = "native-occt")]
    super::health::check_native_cad()?;
    #[cfg(not(feature = "native-occt"))]
    check_cad(&state.root).await?;
    crate::permissions::consume(&state, &project_id, "run_agent").await?;
    let _access = crate::project_access::ensure_write(&state, &project_id)?;
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut tasks = state.tasks.lock().await;
        if tasks.contains_key(&project_id) {
            return Err(AppError::Invalid(
                "An agent task is already running in this project".into(),
            ));
        }
        tasks.insert(project_id.clone(), cancel.clone());
    }
    let session = uuid::Uuid::new_v4().to_string();
    let result=async {
 if let Some(selected) = &selection { super::selection::trusted::validate(&state, &project, selected, cancel.clone()).await?; }
 sqlx::query("INSERT INTO agent_sessions(id,project_id,status,created_at) VALUES(?,?,'running',?)").bind(&session).bind(&project_id).bind(chrono::Utc::now().to_rfc3339()).execute(&state.pool).await?;
 let _=app.emit("agent://started",serde_json::json!({"projectId":project_id,"sessionId":session}));
 let cwd=crate::security::guarded(&state.root,Path::new(&format!("{project_id}/cache/planner/{session}")))?;std::fs::create_dir_all(&cwd)?;
 let custom=if project.agent=="custom" { Some(super::custom::load(&state.pool).await?.ok_or_else(||AppError::Invalid("Custom CLI is not configured".into()))?) } else { None };
 let spec = super::cli_spec::prepare(&project.agent, custom.as_ref(), &cwd, &images)?;
 let context = super::context::project_context(&project, &prompt, &image_names, selection.as_ref());
 #[cfg(feature = "native-occt")]
 let instructions=include_str!("../../scripts/native_protocol.txt");
 #[cfg(not(feature = "native-occt"))]
 let instructions=concat!(include_str!("../../scripts/cad_protocol.txt"), "\n", include_str!("../../scripts/model_protocol.txt"));
 let event_app=app.clone();let event_project=project_id.clone();let event_session=session.clone();
 let listener:processes::OutputListener=Arc::new(move |line|{if let Some((kind,text))=public_event(line){let _=event_app.emit("agent://progress",serde_json::json!({"projectId":event_project,"sessionId":event_session,"kind":kind,"text":text}));}});
 let input=format!("{instructions}{context}");
 if let Some(selected) = &selection { super::selection::trusted::recheck(&state, &project, selected).await?; }
 #[cfg(feature = "native-occt")]
 { let current = context["currentProgram"].as_str().and_then(|source| serde_json::from_str::<crate::cad_ir::Document>(source).ok());
   let current = current.filter(|document| Some(document.revision_id.as_str()) == project.current_revision.as_deref());
   super::repair::plan_with_two_repairs(&project.agent,&spec,&input,cancel,listener,super::repair::RepairSnapshot {current:current.as_ref(),state:&state,captured:&project}).await }
 #[cfg(not(feature = "native-occt"))]
 { let output=processes::run_stream(&spec,&input,cancel,Duration::from_secs(180),Some(listener)).await?; parse_output(&project.agent,&output) }
 }.await;
    state.tasks.lock().await.remove(&project_id);
    let status = if result.is_ok() {
        "completed"
    } else {
        "failed"
    };
    let _ = sqlx::query("UPDATE agent_sessions SET status=?,completed_at=? WHERE id=?")
        .bind(status)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&session)
        .execute(&state.pool)
        .await;
    let _ = app.emit(
        "agent://finished",
        serde_json::json!({"projectId":project_id,"status":status}),
    );
    result
}
#[tauri::command]
pub async fn cancel_task(project_id: String, state: State<'_, AppState>) -> Result<()> {
    state.cad_tasks.cancel_project(&project_id)?;
    if let Some(cancel) = state.tasks.lock().await.get(&project_id) {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}
