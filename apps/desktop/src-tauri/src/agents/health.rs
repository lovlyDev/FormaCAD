use crate::{
    core::{AppError, AppState, Result},
    processes::{self, CommandSpec},
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tauri::State;
#[derive(Serialize)]
pub struct Health {
    pub name: String,
    pub available: bool,
    pub detail: String,
}
#[tauri::command]
pub async fn detect_environment(state: State<'_, AppState>) -> Result<Vec<Health>> {
    let mut result = Vec::new();
    #[cfg(feature = "native-occt")]
    {
        let worker = std::env::current_exe()?
            .parent()
            .ok_or_else(|| AppError::Invalid("CAD worker path is unavailable".into()))?
            .join(format!("forma-cad-worker{}", std::env::consts::EXE_SUFFIX));
        result.push(Health {
            name: "OpenCascade kernel".into(),
            available: worker.is_file(),
            detail: worker.display().to_string(),
        });
    }
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
    let custom = super::custom::load(&state.pool).await?;
    result.push(Health {
        name: "Custom CLI".into(),
        available: custom
            .as_ref()
            .is_some_and(|config| crate::security::executable(&config.executable).is_ok()),
        detail: custom
            .map(|config| config.executable.display().to_string())
            .unwrap_or_else(|| "Custom CLI is not configured".into()),
    });
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
    result.push(Health{name:if cfg!(feature = "native-occt") { "Legacy CadQuery (optional)" } else { "CadQuery kernel" }.into(),available:installed.is_some(),detail:installed.unwrap_or_else(||"Install cadquery in a dedicated Python environment; set FORMA_PYTHON to its executable.".into())});
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
#[cfg(feature = "native-occt")]
pub fn check_native_cad() -> Result<()> {
    let worker = std::env::current_exe()?
        .parent()
        .ok_or_else(|| AppError::Invalid("CAD worker path is unavailable".into()))?
        .join(format!("forma-cad-worker{}", std::env::consts::EXE_SUFFIX));
    if !worker.is_file() {
        return Err(AppError::Invalid("Native CAD kernel is unavailable".into()));
    }
    Ok(())
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
