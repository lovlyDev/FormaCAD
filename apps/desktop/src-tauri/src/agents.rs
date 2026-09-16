use crate::{
    core::{AppError, AppState, Result},
    processes::{self, CommandSpec},
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{Emitter, State};
#[derive(Serialize)]
pub struct Health {
    pub name: String,
    pub available: bool,
    pub detail: String,
}
#[tauri::command]
pub async fn detect_environment(state: State<'_, AppState>) -> Result<Vec<Health>> {
    let mut result = Vec::new();
    for (name, path) in [
        ("OpenAI Codex", processes::find_codex()),
        ("Claude Code", processes::find_executable("claude")),
        ("Python", processes::python(&state.root)),
    ] {
        let mut available = false;
        let detail = if let Some(exe) = path {
            let spec = CommandSpec {
                executable: exe,
                args: vec!["--version".into()],
                cwd: state.root.clone(),
            };
            match processes::run(
                &spec,
                "",
                Arc::new(AtomicBool::new(false)),
                Duration::from_secs(8),
            )
            .await
            {
                Ok(v) => {
                    available = true;
                    format!("{} · {}", v.trim(), spec.executable.display())
                }
                Err(_) => "Installed, but version check failed".into(),
            }
        } else {
            "Not found. Install this tool and restart Forma.".into()
        };
        result.push(Health {
            name: name.into(),
            available,
            detail,
        });
    }
    let installed = if let Some(exe) = processes::python(&state.root) {
        processes::run(
            &CommandSpec {
                executable: exe,
                args: vec![
                    "-I".into(),
                    "-c".into(),
                    "import cadquery; print(cadquery.__version__)".into(),
                ],
                cwd: state.root.clone(),
            },
            "",
            Arc::new(AtomicBool::new(false)),
            Duration::from_secs(20),
        )
        .await
        .ok()
    } else {
        None
    };
    result.push(Health{name:"CadQuery kernel".into(),available:installed.is_some(),detail:installed.unwrap_or_else(||"Install cadquery in a dedicated Python environment; set FORMA_PYTHON to its executable.".into())});
    for skill in [
        "cad",
        "cad-viewer",
        "dxf",
        "step-parts",
        "dfam-check",
        "gcode",
        "urdf",
        "srdf",
        "sdf",
    ] {
        let p = skill_path(skill);
        result.push(Health {
            name: format!("text-to-cad / {skill}"),
            available: p.is_some(),
            detail: p.map(|p| p.display().to_string()).unwrap_or_else(|| {
                "Not detected in ~/.agents/skills, ~/.codex/skills or ~/.claude/skills".into()
            }),
        });
    }
    Ok(result)
}
fn skill_path(name: &str) -> Option<PathBuf> {
    let base = directories::BaseDirs::new()?;
    for parent in [".agents/skills", ".codex/skills", ".claude/skills"] {
        let p = base.home_dir().join(parent).join(name).join("SKILL.md");
        if p.is_file() {
            return Some(p);
        }
    }
    None
}
pub async fn check_cad(root: &Path) -> Result<PathBuf> {
    let executable = processes::python(root).ok_or_else(|| AppError::Invalid("CAD-ядро не настроено. Откройте Settings → CAD environment и выберите Python с CadQuery.".into()))?;
    check_python(root, &executable).await?;
    Ok(executable)
}
async fn check_python(root: &Path, executable: &Path) -> Result<()> {
    processes::run(&CommandSpec {
        executable: executable.into(), cwd: root.into(),
        args: vec!["-I".into(), "-c".into(), "import cadquery as cq; assert cq.Workplane('XY').box(1,1,1).val().isValid(); print(cq.__version__)".into()],
    }, "", Arc::new(AtomicBool::new(false)), Duration::from_secs(30)).await
    .map_err(|_| AppError::Invalid(format!("CAD-ядро недоступно в Python {}. Выберите Python с CadQuery в Settings → CAD environment. Модель не изменена.", executable.display())))?;
    Ok(())
}
#[tauri::command]
pub async fn choose_cad_python(
    locale: Option<String>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Option<String>> {
    use tauri_plugin_dialog::DialogExt;
    let selected = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_title(if locale.as_deref() == Some("en") {
                "Python with CadQuery"
            } else {
                "Python с установленным CadQuery"
            })
            .blocking_pick_file()
    })
    .await
    .map_err(|e| AppError::Invalid(e.to_string()))?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected
        .into_path()
        .map_err(|_| AppError::Invalid("Выберите локальный Python".into()))?;
    check_python(&state.root, &path).await?;
    let config = state
        .root
        .parent()
        .ok_or_else(|| AppError::Invalid("Missing data directory".into()))?
        .join("cad-python.txt");
    let value = path.to_string_lossy().to_string();
    std::fs::write(config, &value)?;
    Ok(Some(value))
}
pub fn parse_output(provider: &str, output: &str) -> Result<PlanResult> {
    let mut final_text = None;
    for line in output.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if provider == "codex"
                && v["type"] == "item.completed"
                && v["item"]["type"] == "agent_message"
            {
                final_text = v["item"]["text"].as_str().map(str::to_owned);
            }
            if provider == "claude" && v["type"] == "result" {
                if v["is_error"] == true {
                    return Err(AppError::Invalid(
                        "Claude could not complete the request. Check your CLI login.".into(),
                    ));
                }
                final_text = v["result"].as_str().map(str::to_owned);
            }
        }
    }
    let raw = final_text
        .ok_or_else(|| AppError::Invalid("The agent returned no structured model result".into()))?;
    let raw = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|_| AppError::Invalid("Agent response was not valid model data".into()))?;
    let message = value["message"]
        .as_str()
        .ok_or_else(|| AppError::Invalid("Agent response is missing message".into()))?
        .to_string();
    let program = if let Some(cad) = value.get("cad").filter(|v| !v.is_null()) {
        if value.get("program").is_some_and(|v| !v.is_null()) {
            return Err(AppError::Invalid(
                "Return cad or fallback program, not both".into(),
            ));
        }
        let source = serde_json::to_string_pretty(cad)?;
        crate::cad_document::CadDocument::parse(&source)?.compile()?;
        if source.len() > 60000 {
            return Err(AppError::Invalid("CAD document exceeds 60000 bytes".into()));
        }
        Some(source)
    } else {
        match value.get("program") {
            Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::String(source))
                if !source.trim().is_empty() && source.len() <= 60000 =>
            {
                Some(source.trim().to_string())
            }
            _ => {
                return Err(AppError::Invalid(
                    "Agent response must contain a CAD program or program:null".into(),
                ))
            }
        }
    };
    Ok(PlanResult { message, program })
}
#[derive(Serialize)]
pub struct PlanResult {
    pub message: String,
    pub program: Option<String>,
}

fn public_event(line: &str) -> Option<(String, String)> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    match v["type"].as_str()? {
        "thread.started" => Some(("connected".into(), "CLI session connected".into())),
        "turn.started" => Some(("working".into(), "Preparing a response".into())),
        "item.started" | "item.updated" | "item.completed"
            if v["item"]["type"] == "agent_message" =>
        {
            let raw = v["item"]["text"].as_str()?;
            let text = serde_json::from_str::<serde_json::Value>(raw)
                .ok()
                .and_then(|value| value["message"].as_str().map(str::to_owned));
            text.map(|text| ("message".into(), text))
        }
        "turn.completed" => Some(("completed".into(), "Response received".into())),
        "turn.failed" | "error" => Some((
            "error".into(),
            v["message"]
                .as_str()
                .or(v["error"]["message"].as_str())
                .unwrap_or("CLI request failed")
                .chars()
                .take(2000)
                .collect(),
        )),
        _ => None,
    }
}
#[tauri::command]
pub async fn plan_model(
    project_id: String,
    prompt: String,
    attachment_names: Option<Vec<String>>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<PlanResult> {
    if prompt.trim().is_empty() || prompt.len() > 16000 {
        return Err(AppError::Invalid(
            "Prompt must contain between 1 and 16000 characters".into(),
        ));
    }
    let project = crate::projects::get(&state, &project_id).await?;
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
    check_cad(&state.root).await?;
    crate::permissions::consume(&state, &project_id, "run_agent").await?;
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
 sqlx::query("INSERT INTO agent_sessions(id,project_id,status,created_at) VALUES(?,?,'running',?)").bind(&session).bind(&project_id).bind(chrono::Utc::now().to_rfc3339()).execute(&state.pool).await?;
 let _=app.emit("agent://started",serde_json::json!({"projectId":project_id,"sessionId":session}));
 let cwd=crate::security::guarded(&state.root,Path::new(&format!("{project_id}/cache/planner/{session}")))?;std::fs::create_dir_all(&cwd)?;
 let exe=match project.agent.as_str(){"codex"=>processes::find_codex(),"claude"=>processes::find_executable("claude"),_=>None}.ok_or_else(||AppError::Invalid("The selected agent is not available. Install Codex or Claude Code, sign in, then restart Forma.".into()))?;
 let mut args=if project.agent=="codex"{
 // Require ignore-user-config support. Older versions fail closed instead of retrying unsafely.
 let mut args=vec!["exec".into(),"--ignore-user-config".into(),"--ignore-rules".into(),"--ephemeral".into(),"--skip-git-repo-check".into(),"--sandbox".into(),"read-only".into(),"--json".into(),"-c".into(),"approval_policy=\"never\"".into(),"-c".into(),"web_search=\"disabled\"".into(),"-c".into(),"mcp_servers={}".into()];
 for feature in ["shell_tool","unified_exec","apps","hooks","plugins","remote_plugin","skill_search","skill_mcp_dependency_install","multi_agent","multi_agent_v2","computer_use","browser_use","browser_use_external","image_generation","view_image","workspace_dependencies","code_mode_host","memories","goals","shell_snapshot"]{args.push("-c".into());args.push(format!("features.{feature}=false"));}args.push("-".into());args
 }else{vec!["--bare".into(),"--print".into(),"--tools".into(),"".into(),"--strict-mcp-config".into(),"--mcp-config".into(),"{\"mcpServers\":{}}".into(),"--output-format".into(),"json".into(),"--no-session-persistence".into()]};
 for (index,(ext,bytes)) in images.iter().enumerate() {
     let name=format!("reference-{index}.{ext}");
     crate::artifacts::immutable_write(&cwd,Path::new(&name),bytes)?;
     args.insert(args.len()-1,"--image".into());
     args.insert(args.len()-1,cwd.join(name).display().to_string());
 }
 let current=project.revisions.iter().find(|r|Some(&r.id)==project.current_revision.as_ref());
 let history:Vec<_>=project.messages.iter().rev().filter(|m| m.role == "user" || m.role == "assistant").take(6).map(|m|serde_json::json!({"role":m.role,"text":m.text.chars().take(2000).collect::<String>()})).collect();
 let context=serde_json::json!({"name":project.name,"dimensionsUnit":"mm","cadKernelReady":true,"referenceImages":image_names,"currentProgram":current.and_then(|r|r.program.as_ref()),"legacyModel":current.filter(|r|r.source.is_none()).map(|r|&r.parameters),"baseStepAvailable":current.and_then(|r|r.source.as_ref()).is_some_and(|s|s.ends_with(".step")||s.ends_with(".stp")),"recentMessagesNewestFirst":history,"request":prompt});
 let instructions=concat!(include_str!("../scripts/cad_protocol.txt"), "\n", include_str!("../scripts/model_protocol.txt"));
 let event_app=app.clone();let event_project=project_id.clone();let event_session=session.clone();
 let listener:processes::OutputListener=Arc::new(move |line|{if let Some((kind,text))=public_event(line){let _=event_app.emit("agent://progress",serde_json::json!({"projectId":event_project,"sessionId":event_session,"kind":kind,"text":text}));}});
 let output=processes::run_stream(&CommandSpec{executable:exe,args,cwd},&format!("{instructions}{context}"),cancel,Duration::from_secs(180),Some(listener)).await?;
 parse_output(&project.agent,&output)
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
    if let Some(cancel) = state.tasks.lock().await.get(&project_id) {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_document_round_trips_through_agent_protocol() {
        let cad: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/fixtures/plate-hole.cad.json"
        ))
        .unwrap();
        let response = serde_json::json!({"message":"Plate", "cad":cad});
        let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":response.to_string()}});
        let parsed = parse_output("codex", &event.to_string()).unwrap();
        let stored: serde_json::Value = serde_json::from_str(&parsed.program.unwrap()).unwrap();
        assert_eq!(stored, cad);
        let response = serde_json::json!({"message":"Plate", "cad":cad, "program":"result = base"});
        let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":response.to_string()}});
        assert!(parse_output("codex", &event.to_string()).is_err());
    }
    #[test]
    fn blank_result_is_not_a_success() {
        let p = serde_json::json!({"kind":"blank","width":120,"depth":65,"height":60,"thickness":5,"holeDiameter":8,"holes":4});
        let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":p.to_string()}});
        assert!(parse_output("codex", &event.to_string()).is_err());
    }
    #[test]
    fn public_messages_and_status_are_forwarded() {
        assert_eq!(
            public_event(r#"{"type":"thread.started"}"#).unwrap().0,
            "connected"
        );
        assert!(public_event(
            r#"{"type":"item.completed","item":{"type":"reasoning","text":"private"}}"#
        )
        .is_none());
        let response = serde_json::json!({"message":"Ready","program":null});
        let line=serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":response.to_string()}}).to_string();
        assert_eq!(public_event(&line).unwrap().1, "Ready");
        assert!(parse_output("codex", &line).unwrap().program.is_none());
    }
    #[test]
    fn malformed_geometry_is_rejected() {
        assert!(parse_output(
            "codex",
            "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"{}\"}}"
        )
        .is_err());
        assert!(parse_output("codex", "not json").is_err());
    }
    #[test]
    fn valid_structured_event_is_read() {
        let p = serde_json::json!({"message":"Шар", "program":"result = cq.Workplane(\"XY\").sphere(25)"});
        let event = serde_json::json!({"type":"item.completed","item":{"type":"agent_message","text":p.to_string()}});
        assert!(parse_output("codex", &event.to_string())
            .unwrap()
            .program
            .unwrap()
            .contains("sphere"));
    }
}
