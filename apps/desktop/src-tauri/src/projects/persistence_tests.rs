use super::*;

#[cfg(windows)]
fn directory_link(link: &Path, target: &Path) {
    use std::os::windows::process::CommandExt;
    let command = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap())
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    // The shell's filesystem provider does not accept canonical verbatim paths.
    let normal_path = |path: &Path| {
        let value = path.to_string_lossy();
        if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
            format!(r"\\{unc}")
        } else {
            value.strip_prefix(r"\\?\").unwrap_or(&value).to_string()
        }
    };
    assert_eq!(normal_path(Path::new(r"\\?\C:\fixture")), r"C:\fixture");
    assert_eq!(
        normal_path(Path::new(r"\\?\UNC\server\share\fixture")),
        r"\\server\share\fixture"
    );
    let link = normal_path(link);
    let target = normal_path(target);
    let literal = |value: &str| value.replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path '{}' -Target '{}' | Out-Null",
        literal(&link), literal(&target)
    );
    let output = std::process::Command::new(command)
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
#[cfg(unix)]
fn directory_link(link: &Path, target: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

async fn temporary_state(root: &Path) -> AppState {
    let (_, log_guard) = tracing_appender::non_blocking(std::io::sink());
    AppState {
        root: root.to_path_buf(),
        pool: crate::storage::open(&root.join("fixture.sqlite"))
            .await
            .unwrap(),
        cad_tasks: Default::default(),
        project_access: Default::default(),
        grants: Default::default(),
        tasks: Default::default(),
        writes: Default::default(),
        _log_guard: log_guard,
    }
}

fn blank() -> Project {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Postcommit fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-10-05T00:00:00Z",
        "updatedAt":"2026-10-05T00:00:00Z","revisions":[],"currentRevision":null,
        "messages":[],"files":[],"exports":[]
    }))
    .unwrap()
}
fn edited(previous: &Project) -> Project {
    let mut next = previous.clone();
    let id = uuid::Uuid::new_v4().to_string();
    next.revisions.push(
        serde_json::from_value(serde_json::json!({
            "id":id,"parent":previous.current_revision,"createdAt":"2026-10-05T00:01:00Z",
            "prompt":"Storage-only fault fixture","program":"fixture source",
        "parameters":{"kind":"blank","width":40,"depth":20,"height":10,"thickness":2,
            "holeDiameter":2,"holes":4},"source":null,"preview":null,"programBase":null
        }))
        .unwrap(),
    );
    next.current_revision = Some(id);
    next.updated_at = "2026-10-05T00:01:00Z".into();
    next
}

#[test]
fn metadata_save_does_not_repeat_revision_event_but_head_movement_still_does() {
    let first = blank();
    assert!(!super::super::events::revision_changed(None, &first));
    let next = edited(&first);
    assert!(super::super::events::revision_changed(Some(&first), &next));
    let mut metadata = next.clone();
    metadata.name = "Assistant metadata saved".into();
    assert!(!super::super::events::revision_changed(
        Some(&next),
        &metadata
    ));
    assert!(super::super::events::revision_changed(Some(&next), &first));
    assert!(super::super::events::revision_changed(Some(&first), &next));
}

#[tokio::test]
async fn missing_generation_after_sql_commit_returns_success_and_preserves_recovery_journal() {
    let temporary = tempfile::tempdir().unwrap();
    let (_, log_guard) = tracing_appender::non_blocking(std::io::sink());
    let state = AppState {
        root: temporary.path().to_path_buf(),
        pool: crate::storage::open(&temporary.path().join("fixture.sqlite"))
            .await
            .unwrap(),
        cad_tasks: Default::default(),
        project_access: Default::default(),
        grants: Default::default(),
        tasks: Default::default(),
        writes: Default::default(),
        _log_guard: log_guard,
    };
    let original = persist(&state, blank(), vec![], None).await.unwrap();
    let next = edited(&original);
    let project_root = state.root.join(&next.id);
    let generation_root = project_root.join("metadata/generations");
    let detached = project_root.join("metadata/generations-detached");
    // Filesystem disruption occurs AFTER the actual durable SQL commit, not before staging.
    let fault =
        |root: &Path| std::fs::rename(root.join("metadata/generations"), &detached).unwrap();
    let committed = persist_transaction_inner(
        &state,
        next.clone(),
        vec![],
        None,
        None,
        None,
        PostCommitHook {
            callback: Some(&fault),
            ..Default::default()
        },
    )
    .await
    .expect("Durable commit must not become a mirror/publication error");
    assert_eq!(committed.current_revision, next.current_revision);
    let raw: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&next.id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Project>(&raw)
            .unwrap()
            .current_revision,
        next.current_revision
    );
    let journal: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_storage_commits WHERE project_id=?")
            .bind(&next.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(journal, 1);
    let edits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM project_history_events WHERE project_id=? AND kind='edit'",
    )
    .bind(&next.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(edits, 1);
    assert!(!generation_root.exists());
    assert!(project_root.join("manifest.json").exists());
    assert!(detached.is_dir());
    // Restore only this temporary fixture; production recovery owns durable generations.
    std::fs::rename(&detached, &generation_root).unwrap();
    let recovered = crate::projects::get(&state, &next.id).await.unwrap();
    assert_eq!(recovered.current_revision, next.current_revision);
    assert_eq!(recovered.revisions, next.revisions);
    let remaining: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_storage_commits WHERE project_id=?")
            .bind(&next.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(remaining, 0);
    let mut metadata = recovered.clone();
    metadata.messages.push(crate::models::Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".into(),
        text: "Accepted model metadata".into(),
        created_at: "2026-10-05T00:02:00Z".into(),
    });
    assert!(!super::super::events::revision_changed(
        Some(&recovered),
        &metadata
    ));
    let metadata_saved = persist(&state, metadata.clone(), vec![], None)
        .await
        .unwrap();
    assert_eq!(metadata_saved.current_revision, next.current_revision);
    assert_eq!(metadata_saved.messages, metadata.messages);
    assert_eq!(metadata_saved.revisions, next.revisions);
    let edits_after_metadata: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM project_history_events WHERE project_id=? AND kind='edit'",
    )
    .bind(&next.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(edits_after_metadata, 1);
    state.pool.close().await;
}

#[tokio::test]
async fn guarded_mirror_failure_after_successful_publication_is_committed_success() {
    let temporary = tempfile::tempdir().unwrap();
    let state = temporary_state(temporary.path()).await;
    let original = persist(&state, blank(), vec![], None).await.unwrap();
    let next = edited(&original);
    let project_root = state.root.join(&next.id);
    let external = tempfile::tempdir().unwrap();
    let sentinel = external.path().join("model.py");
    std::fs::write(&sentinel, b"External source must stay unchanged").unwrap();
    let workspace = project_root.join("workspace");
    let detached = project_root.join("workspace-detached");
    let fault = |root: &Path| {
        std::fs::rename(root.join("workspace"), &detached).unwrap();
        directory_link(&root.join("workspace"), external.path());
    };
    let saved = persist_transaction_inner(
        &state,
        next.clone(),
        vec![],
        None,
        None,
        None,
        PostCommitHook {
            before_mirrors: Some(&fault),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(saved.current_revision, next.current_revision);
    assert!(
        crate::security::guarded(&state.root, &Path::new(&next.id).join("workspace/model.py"))
            .is_err()
    );
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"External source must stay unchanged"
    );
    assert!(!external.path().join("model.parameters.json").exists());
    assert_eq!(
        crate::project_storage_v2::hydrate_folder(&project_root)
            .unwrap()
            .current_revision,
        next.current_revision
    );
    let journal: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_storage_commits WHERE project_id=?")
            .bind(&next.id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(
        journal, 0,
        "Publication succeeded before the mirror-only fault"
    );
    let raw: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&next.id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Project>(&raw).unwrap().revisions,
        next.revisions
    );
    assert!(std::fs::symlink_metadata(&workspace)
        .unwrap()
        .file_type()
        .is_symlink());
    #[cfg(windows)]
    std::fs::remove_dir(&workspace).unwrap();
    #[cfg(unix)]
    std::fs::remove_file(&workspace).unwrap();
    std::fs::rename(detached, &workspace).unwrap();
    assert_eq!(
        crate::projects::get(&state, &next.id)
            .await
            .unwrap()
            .current_revision,
        next.current_revision
    );
    state.pool.close().await;
}
