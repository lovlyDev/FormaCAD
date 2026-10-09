#![cfg(feature = "native-occt")]
//! Host selection trust boundary, real worker, isolated SQLite and project files.
use forma_core::{
    agents::selection::{trusted, SelectionContext},
    core::AppState,
    models::{Project, ProjectFile},
};
use serde_json::json;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

async fn fixture(root: &Path) -> (AppState, Project, SelectionContext) {
    let pool = forma_core::storage::open(&root.join("selection.sqlite"))
        .await
        .unwrap();
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    let state = AppState {
        root: root.into(),
        pool,
        _log_guard: guard,
        cad_tasks: Default::default(),
        project_access: Default::default(),
        grants: Default::default(),
        tasks: Default::default(),
        writes: Default::default(),
    };
    let source = json!({"schemaVersion":2,"revisionId":"11111111-1111-4111-8111-111111111111","parameters":[],"features":[
        {"id":"profile","name":"Profile","operation":{"type":"rectangle","width":{"kind":"literal","mm":40},"depth":{"kind":"literal","mm":20}}},
        {"id":"pad","name":"Pad","operation":{"type":"extrude","sketchId":"profile","distance":{"kind":"literal","mm":10}}}
    ],"bodies":[{"id":"body","name":"Body","sourceFeatureId":"pad"}]}).to_string();
    let worker_dir = root.join("worker");
    std::fs::create_dir(&worker_dir).unwrap();
    forma_core::native::worker::build_selected_with_executable(
        &source,
        "body",
        &worker_dir,
        Arc::new(AtomicBool::new(false)),
        Path::new(env!("CARGO_BIN_EXE_forma-cad-worker")),
    )
    .await
    .unwrap();
    let mut project: Project = serde_json::from_value(json!({"schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),
        "name":"Selection fixture","units":"mm","agent":"codex","pinned":false,"createdAt":"2026-10-05T00:00:00Z","updatedAt":"2026-10-05T00:00:00Z",
        "currentRevision":"11111111-1111-4111-8111-111111111111","revisions":[{"id":"11111111-1111-4111-8111-111111111111","parent":null,"createdAt":"2026-10-05T00:00:00Z","prompt":"Fixture",
        "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":2,"holeDiameter":0,"holes":0},
        "source":"model.step","preview":"preview.glb","program":source}],"files":[],"messages":[],"exports":[]})).unwrap();
    for name in ["model.step", "preview.glb"] {
        let bytes = std::fs::read(worker_dir.join(name)).unwrap();
        project.files.push(ProjectFile {
            name: name.into(),
            kind: "model".into(),
            size: bytes.len() as u64,
            sha256: Some(forma_core::artifacts::digest(&bytes)),
            data: Some(forma_core::artifacts::data_url(name, &bytes)),
        });
    }
    let pending = forma_core::artifacts::normalize(&mut project).unwrap();
    let project = forma_core::projects::persist(&state, project, pending, None)
        .await
        .unwrap();
    let selected = SelectionContext {
        body_id: "body".into(),
        revision_id: "11111111-1111-4111-8111-111111111111".into(),
        face_ordinal: Some(1),
        edge_ordinal: None,
        semantic_key: None,
        topology_ref: None,
    };
    (state, project, selected)
}

#[tokio::test]
async fn host_verifies_real_kernel_selection_and_rejects_forged_body_and_ordinals() {
    let root = tempfile::tempdir().unwrap();
    let (state, project, mut pick) = fixture(root.path()).await;
    let executable = Path::new(env!("CARGO_BIN_EXE_forma-cad-worker"));
    let state_ref = &state;
    let project_ref = &project;
    let validate = |pick: SelectionContext| async move {
        trusted::validate_with_executable(
            state_ref,
            project_ref,
            &pick,
            Arc::new(AtomicBool::new(false)),
            executable,
        )
        .await
    };
    validate(pick.clone()).await.unwrap();
    assert!(state.cad_tasks.status().unwrap().is_empty());
    pick.body_id = "fake_body".into();
    assert!(validate(pick.clone()).await.is_err());
    pick.body_id = "body".into();
    pick.face_ordinal = Some(7);
    assert!(validate(pick.clone()).await.is_err());
    pick.face_ordinal = Some(1);
    pick.revision_id = "old_revision".into();
    assert!(validate(pick.clone()).await.is_err());
    let bytes = std::fs::read(root.path().join("worker/preview.glb")).unwrap();
    let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    let glb: serde_json::Value = serde_json::from_slice(&bytes[20..20 + length]).unwrap();
    let edge = &glb["meshes"][0]["primitives"][0]["extras"]["formaEdges"][0];
    pick.revision_id = project.current_revision.clone().unwrap();
    pick.face_ordinal = None;
    pick.edge_ordinal = Some(1);
    pick.semantic_key = edge["semanticKey"].as_str().map(str::to_owned);
    pick.topology_ref = Some(serde_json::from_value(edge["topologyRef"].clone()).unwrap());
    validate(pick.clone()).await.unwrap();
    pick.topology_ref.as_mut().unwrap().owner_feature_id = "other_branch".into();
    assert!(validate(pick.clone()).await.is_err());
    pick.topology_ref = None;
    pick.semantic_key = Some("box-edge:x:ymax:zmax".into());
    // A valid-looking key from another ordinal must never redirect this pick.
    if edge["semanticKey"].as_str() == pick.semantic_key.as_deref() {
        pick.semantic_key = Some("box-edge:y:xmin:zmin".into());
    }
    assert!(validate(pick).await.is_err());
    assert_eq!(
        forma_core::projects::get(&state, &project.id)
            .await
            .unwrap()
            .revisions
            .len(),
        1
    );
}

#[tokio::test]
async fn tampered_saved_source_and_preview_never_reach_an_external_worker() {
    let root = tempfile::tempdir().unwrap();
    let (state, project, pick) = fixture(root.path()).await;
    // An impossible executable proves the host rejects the source before spawn.
    let executable = root.path().join("must-not-spawn.exe");
    for index in 0..2 {
        let mut forged = project.clone();
        forged.files[index].data = Some(forma_core::artifacts::data_url(
            &forged.files[index].name,
            b"tampered",
        ));
        let error = trusted::validate_with_executable(
            &state,
            &forged,
            &pick,
            Arc::new(AtomicBool::new(false)),
            &executable,
        )
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "SELECTION_CONTEXT_INVALID");

        let file = &project.files[index];
        let extension = Path::new(&file.name).extension().unwrap().to_str().unwrap();
        let path = root
            .path()
            .join(&project.id)
            .join("attachments")
            .join(format!("{}.{}", file.sha256.as_deref().unwrap(), extension));
        let original = std::fs::read(&path).unwrap();
        let mut corrupted = original.clone();
        corrupted[0] ^= 1;
        std::fs::write(&path, &corrupted).unwrap();
        let error = trusted::validate_with_executable(
            &state,
            &project,
            &pick,
            Arc::new(AtomicBool::new(false)),
            &executable,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("integrity"));
        std::fs::write(&path, &original).unwrap();
    }
    assert!(state.cad_tasks.status().unwrap().is_empty());
}

#[tokio::test]
async fn queued_selection_rechecks_committed_head_before_spawning_worker() {
    let root = tempfile::tempdir().unwrap();
    let (state, project, pick) = fixture(root.path()).await;
    let state = Arc::new(state);
    let hold = state
        .cad_tasks
        .acquire(&project.id, "fixture", Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    let task_state = state.clone();
    let captured = project.clone();
    let missing_executable = root.path().join("must-not-spawn.exe");
    let validation = tokio::spawn(async move {
        trusted::validate_with_executable(
            &task_state,
            &captured,
            &pick,
            Arc::new(AtomicBool::new(false)),
            &missing_executable,
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !state
            .cad_tasks
            .status()
            .unwrap()
            .iter()
            .any(|task| task.state == "queued")
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut changed = project.clone();
    let mut next = changed.revisions[0].clone();
    next.id = "22222222-2222-4222-8222-222222222222".into();
    next.parent = Some("11111111-1111-4111-8111-111111111111".into());
    changed.revisions.push(next);
    changed.current_revision = Some("22222222-2222-4222-8222-222222222222".into());
    forma_core::projects::persist(&state, changed, Vec::new(), None)
        .await
        .unwrap();
    drop(hold);
    let error = validation.await.unwrap().unwrap_err();
    assert!(error.to_string().contains("stale or invalid"));
    assert!(state.cad_tasks.status().unwrap().is_empty());
    assert_eq!(
        forma_core::projects::get(&state, &project.id)
            .await
            .unwrap()
            .current_revision
            .as_deref(),
        Some("22222222-2222-4222-8222-222222222222")
    );
}

#[tokio::test]
async fn cancelling_selection_in_shared_queue_never_spawns_worker() {
    let root = tempfile::tempdir().unwrap();
    let (state, project, pick) = fixture(root.path()).await;
    let state = Arc::new(state);
    let hold = state
        .cad_tasks
        .acquire(&project.id, "fixture", Arc::new(AtomicBool::new(false)))
        .await
        .unwrap();
    let task_state = state.clone();
    let captured = project.clone();
    let missing_executable = root.path().join("must-not-spawn.exe");
    let validation = tokio::spawn(async move {
        trusted::validate_with_executable(
            &task_state,
            &captured,
            &pick,
            Arc::new(AtomicBool::new(false)),
            &missing_executable,
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while !state
            .cad_tasks
            .status()
            .unwrap()
            .iter()
            .any(|task| task.state == "queued")
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    state.cad_tasks.cancel_project(&project.id).unwrap();
    let error = validation.await.unwrap().unwrap_err();
    assert!(error.to_string().to_lowercase().contains("cancel"));
    drop(hold);
    assert!(state.cad_tasks.status().unwrap().is_empty());
    assert_eq!(
        forma_core::projects::get(&state, &project.id)
            .await
            .unwrap()
            .revisions
            .len(),
        1
    );
}

#[tokio::test]
async fn imported_asset_checksum_is_rechecked_before_external_worker_spawn() {
    let root = tempfile::tempdir().unwrap();
    let (state, mut project, pick) = fixture(root.path()).await;
    let mut asset = std::fs::read(root.path().join("worker/model.step")).unwrap();
    asset.extend_from_slice(b"\n "); // valid independent STEP bytes, distinct content address
    let hash = forma_core::artifacts::digest(&asset);
    let mut document: serde_json::Value =
        serde_json::from_str(project.revisions[0].program.as_ref().unwrap()).unwrap();
    document["features"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"unused_import","name":"Imported asset",
        "operation":{"type":"importStep","assetId":format!("step_{hash}"),"sha256":hash}}));
    let mut next = project.revisions[0].clone();
    let next_id = "22222222-2222-4222-8222-222222222222";
    next.id = next_id.into();
    next.parent = project.current_revision.clone();
    document["revisionId"] = next_id.into();
    next.program = Some(document.to_string());
    project.revisions.push(next);
    project.current_revision = Some(next_id.into());
    let pick = SelectionContext {
        revision_id: next_id.into(),
        ..pick
    };
    project.files.push(ProjectFile {
        name: "asset.step".into(),
        size: asset.len() as u64,
        kind: "model".into(),
        sha256: Some(hash.clone()),
        data: Some(forma_core::artifacts::data_url("asset.step", &asset)),
    });
    let pending = forma_core::artifacts::normalize(&mut project).unwrap();
    let project = forma_core::projects::persist(&state, project, pending, None)
        .await
        .unwrap();
    let path = state
        .root
        .join(&project.id)
        .join("attachments")
        .join(format!("{hash}.step"));
    let original = std::fs::read(&path).unwrap();
    let mut corrupted = original.clone();
    corrupted[0] ^= 1;
    std::fs::write(&path, &corrupted).unwrap();
    let error = trusted::validate_with_executable(
        &state,
        &project,
        &pick,
        Arc::new(AtomicBool::new(false)),
        &root.path().join("must-not-spawn.exe"),
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("integrity") || error.to_string().contains("checksum"));
    assert!(state.cad_tasks.status().unwrap().is_empty());
    assert_eq!(
        forma_core::artifacts::digest(
            &std::fs::read(root.path().join("worker/model.step")).unwrap()
        ),
        project.files[0].sha256.as_deref().unwrap()
    );
}
