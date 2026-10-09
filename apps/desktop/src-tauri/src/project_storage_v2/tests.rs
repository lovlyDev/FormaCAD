use super::*;
use crate::models::ProjectFile;

fn fixture() -> Project {
    serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Portable fixture",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-10-04T00:00:00Z",
        "updatedAt":"2026-10-04T00:00:00Z","revisions":[],"currentRevision":null,
        "messages":[],"files":[],"exports":[]
    }))
    .unwrap()
}

fn stage_fixture(root: &Path, project: &Project) -> StagedSnapshot {
    stage_with(root, project, |_| Ok(b"fixture attachment".to_vec())).unwrap()
}

#[test]
fn folder_moves_without_global_database_or_blob_cache() {
    let origin = tempfile::tempdir().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let mut project = fixture();
    let bytes = b"fixture attachment";
    project.files.push(ProjectFile {
        name: "saved.step".into(),
        size: bytes.len() as u64,
        kind: "attachment".into(),
        data: None,
        sha256: Some(crate::artifacts::digest(bytes)),
    });
    let staged = stage_fixture(origin.path(), &project);
    publish(origin.path(), staged).unwrap();
    // Copy only files named by the manifest. No global .blobs or original DB exists here.
    let manifest = manifest_bytes(origin.path(), "manifest.json").unwrap();
    for entry in &manifest.files {
        let relative = entry_relative(&manifest, &entry.path);
        let bytes = std::fs::read(origin.path().join(&relative)).unwrap();
        crate::artifacts::immutable_write(destination.path(), Path::new(&relative), &bytes)
            .unwrap();
    }
    std::fs::copy(
        origin.path().join("manifest.json"),
        destination.path().join("manifest.json"),
    )
    .unwrap();
    let hydrated = hydrate_folder(destination.path()).unwrap();
    assert_eq!(hydrated.id, project.id);
    assert_eq!(
        crate::artifacts::decode_data(hydrated.files[0].data.as_ref().unwrap()).unwrap(),
        bytes
    );
}

#[test]
fn uncommitted_generation_does_not_change_active_snapshot() {
    let folder = tempfile::tempdir().unwrap();
    let project = fixture();
    publish(folder.path(), stage_fixture(folder.path(), &project)).unwrap();
    let mut candidate = project.clone();
    candidate.name = "Never committed".into();
    let _staged = stage_fixture(folder.path(), &candidate);
    assert_eq!(read_folder(folder.path()).unwrap().name, project.name);
}

#[test]
fn corrupt_active_generation_fails_and_previous_remains_readable() {
    let folder = tempfile::tempdir().unwrap();
    let first = fixture();
    publish(folder.path(), stage_fixture(folder.path(), &first)).unwrap();
    let mut second = first.clone();
    second.name = "Next committed".into();
    let next = stage_fixture(folder.path(), &second);
    let generation = next.manifest.generation.clone();
    publish(folder.path(), next).unwrap();
    std::fs::write(
        folder
            .path()
            .join(format!("metadata/generations/{generation}/project.json")),
        b"corrupt",
    )
    .unwrap();
    assert!(read_folder(folder.path()).is_err());
    assert_eq!(read_backup(folder.path()).unwrap().name, first.name);
}

#[test]
fn manifest_rejects_future_schema_traversal_duplicate_and_oversized_files() {
    let folder = tempfile::tempdir().unwrap();
    let staged = stage_fixture(folder.path(), &fixture());
    let mut manifest = staged.manifest.clone();
    manifest.storage_version = 3;
    assert!(manifest.validate().is_err());
    manifest = staged.manifest.clone();
    manifest.files[0].path = "../../outside".into();
    assert!(manifest.validate().is_err());
    manifest = staged.manifest.clone();
    manifest.files.push(manifest.files[0].clone());
    assert!(manifest.validate().is_err());
    manifest = staged.manifest.clone();
    manifest.files[0].size = MAX_METADATA + 1;
    assert!(manifest.validate().is_err());
}

#[test]
fn process_lock_blocks_second_writer_and_releases_on_drop() {
    let folder = tempfile::tempdir().unwrap();
    let first = ProjectLock::acquire(folder.path()).unwrap();
    assert!(ProjectLock::acquire(folder.path()).is_err());
    drop(first);
    assert!(ProjectLock::acquire(folder.path()).is_ok());
}

#[test]
fn failed_attachment_migration_preserves_old_commit() {
    let folder = tempfile::tempdir().unwrap();
    let project = fixture();
    publish(folder.path(), stage_fixture(folder.path(), &project)).unwrap();
    let mut next = project.clone();
    next.files.push(ProjectFile {
        name: "broken.step".into(),
        size: 1,
        kind: "attachment".into(),
        data: None,
        sha256: Some(crate::artifacts::digest(b"a")),
    });
    assert!(stage_with(folder.path(), &next, |_| Ok(b"wrong bytes".to_vec())).is_err());
    assert!(read_folder(folder.path()).unwrap().files.is_empty());
}

#[test]
fn history_snapshot_is_bounded_and_verified() {
    let folder = tempfile::tempdir().unwrap();
    let mut staged = stage_fixture(folder.path(), &fixture());
    stage_history(folder.path(), &mut staged, b"{\"schemaVersion\":1}").unwrap();
    publish(folder.path(), staged).unwrap();
    assert_eq!(
        read_history(folder.path()).unwrap().unwrap(),
        b"{\"schemaVersion\":1}"
    );
}

#[test]
fn metadata_saves_reuse_assets_without_copying_attachment_bytes() {
    let folder = tempfile::tempdir().unwrap();
    let bytes = b"fixture attachment";
    let mut project = fixture();
    project.files.push(ProjectFile {
        name: "saved.step".into(),
        size: bytes.len() as u64,
        kind: "attachment".into(),
        data: None,
        sha256: Some(crate::artifacts::digest(bytes)),
    });
    publish(folder.path(), stage_fixture(folder.path(), &project)).unwrap();
    for index in 0..5 {
        project.name = format!("Metadata edit {index}");
        let staged = stage_with(folder.path(), &project, |_| {
            panic!("unchanged assets must not read legacy source again")
        })
        .unwrap();
        publish(folder.path(), staged).unwrap();
    }
    let assets: Vec<_> = std::fs::read_dir(folder.path().join("assets"))
        .unwrap()
        .collect();
    assert_eq!(assets.len(), 1);
    assert_eq!(
        std::fs::metadata(assets[0].as_ref().unwrap().path())
            .unwrap()
            .len(),
        bytes.len() as u64
    );
    assert_eq!(
        std::fs::read_dir(folder.path().join("metadata/generations"))
            .unwrap()
            .count(),
        6
    );
}

async fn isolated_state(root: &Path) -> AppState {
    std::fs::create_dir_all(root).unwrap();
    let pool = crate::storage::open(&root.join("fixture.sqlite"))
        .await
        .unwrap();
    let (_, guard) = tracing_appender::non_blocking(std::io::sink());
    AppState {
        cad_tasks: Default::default(),
        project_access: Default::default(),
        root: root.to_path_buf(),
        pool,
        grants: tokio::sync::Mutex::new(Default::default()),
        tasks: tokio::sync::Mutex::new(Default::default()),
        writes: tokio::sync::Mutex::new(()),
        _log_guard: guard,
    }
}

async fn journal_commit(state: &AppState, root: &Path, project: &Project, staged: &StagedSnapshot) {
    let mut tx = state.pool.begin().await.unwrap();
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,payload=excluded.payload")
        .bind(&project.id).bind(&project.name).bind(serde_json::to_string(project).unwrap()).bind(&project.updated_at).execute(&mut *tx).await.unwrap();
    record_pending(&mut tx, root, project, staged)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn crash_after_sql_commit_recovers_only_journaled_generation() {
    let folder = tempfile::tempdir().unwrap();
    let state = isolated_state(folder.path()).await;
    let first = fixture();
    let root = folder.path().join(&first.id);
    std::fs::create_dir(&root).unwrap();
    publish(&root, stage_fixture(&root, &first)).unwrap();
    let mut second = first.clone();
    second.name = "SQL committed".into();
    let staged = stage_fixture(&root, &second);
    journal_commit(&state, &root, &second, &staged).await;
    assert_eq!(read_folder(&root).unwrap().name, first.name);
    assert_eq!(
        crate::projects::get(&state, &first.id).await.unwrap().name,
        second.name
    );
    assert_eq!(read_folder(&root).unwrap().name, second.name);
    assert_eq!(read_backup(&root).unwrap().name, first.name);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_storage_commits")
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn recovery_does_not_overwrite_an_external_committed_change() {
    let folder = tempfile::tempdir().unwrap();
    let state = isolated_state(folder.path()).await;
    let first = fixture();
    let root = folder.path().join(&first.id);
    std::fs::create_dir(&root).unwrap();
    publish(&root, stage_fixture(&root, &first)).unwrap();
    let mut candidate = first.clone();
    candidate.name = "Pending".into();
    let staged = stage_fixture(&root, &candidate);
    journal_commit(&state, &root, &candidate, &staged).await;
    let mut external = first.clone();
    external.name = "External".into();
    publish(&root, stage_fixture(&root, &external)).unwrap();
    assert!(reconcile(&state, candidate).await.is_err());
    assert_eq!(read_folder(&root).unwrap().name, "External");
}

#[tokio::test]
async fn portable_import_conflicts_are_explicit_and_copy_keeps_source_unchanged() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();
    let project = fixture();
    publish(source.path(), stage_fixture(source.path(), &project)).unwrap();
    let original = std::fs::read(source.path().join("manifest.json")).unwrap();
    let state = isolated_state(target.path()).await;
    let imported = crate::projects::import_folder(&state, source.path(), false, None)
        .await
        .unwrap();
    assert_eq!(imported.id, project.id);
    assert!(
        crate::projects::import_folder(&state, source.path(), false, None)
            .await
            .is_err()
    );
    let copied = crate::projects::import_folder(&state, source.path(), true, None)
        .await
        .unwrap();
    assert_ne!(copied.id, project.id);
    assert_eq!(
        std::fs::read(source.path().join("manifest.json")).unwrap(),
        original
    );
    assert!(target
        .path()
        .join(&copied.id)
        .join("manifest.json")
        .exists());
}

#[tokio::test]
async fn crash_after_publication_before_journal_cleanup_keeps_previous_backup() {
    let folder = tempfile::tempdir().unwrap();
    let state = isolated_state(folder.path()).await;
    let first = fixture();
    let root = folder.path().join(&first.id);
    std::fs::create_dir(&root).unwrap();
    publish(&root, stage_fixture(&root, &first)).unwrap();
    let mut candidate = first.clone();
    candidate.name = "Published".into();
    let staged = stage_fixture(&root, &candidate);
    journal_commit(&state, &root, &candidate, &staged).await;
    publish(&root, staged).unwrap();
    assert_eq!(
        reconcile(&state, candidate).await.unwrap().name,
        "Published"
    );
    assert_eq!(read_backup(&root).unwrap().name, first.name);
}

#[tokio::test]
async fn baseline_migration_before_failed_sql_preserves_legacy_inline_payload() {
    let folder = tempfile::tempdir().unwrap();
    let state = isolated_state(folder.path()).await;
    let bytes = b"fixture attachment";
    let mut legacy = fixture();
    legacy.files.push(ProjectFile {
        name: "legacy.step".into(),
        size: bytes.len() as u64,
        kind: "attachment".into(),
        data: Some(crate::artifacts::data_url("legacy.step", bytes)),
        sha256: None,
    });
    sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?)")
        .bind(&legacy.id)
        .bind(&legacy.name)
        .bind(serde_json::to_string(&legacy).unwrap())
        .bind(&legacy.updated_at)
        .execute(&state.pool)
        .await
        .unwrap();
    let mut normalized = legacy.clone();
    crate::artifacts::normalize(&mut normalized).unwrap();
    let root = folder.path().join(&legacy.id);
    std::fs::create_dir(&root).unwrap();
    publish(&root, stage_fixture(&root, &normalized)).unwrap();
    let restored = crate::projects::get(&state, &legacy.id).await.unwrap();
    assert_eq!(restored.name, legacy.name);
    assert_eq!(restored.files[0].sha256, normalized.files[0].sha256);
    let raw: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(&legacy.id)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert!(serde_json::from_str::<Project>(&raw).unwrap().files[0]
        .data
        .is_some());
}

#[tokio::test]
async fn new_project_sql_commit_before_first_manifest_is_recovered() {
    let folder = tempfile::tempdir().unwrap();
    let state = isolated_state(folder.path()).await;
    let project = fixture();
    let root = folder.path().join(&project.id);
    std::fs::create_dir(&root).unwrap();
    let staged = stage_fixture(&root, &project);
    journal_commit(&state, &root, &project, &staged).await;
    assert!(!root.join("manifest.json").exists());
    assert_eq!(
        reconcile(&state, project.clone()).await.unwrap().id,
        project.id
    );
    assert_eq!(read_folder(&root).unwrap().id, project.id);
}

#[test]
fn local_layout_fixture_measures_large_history_and_asset_reuse() {
    let folder = tempfile::tempdir().unwrap();
    let mut project = fixture();
    let bytes = vec![42u8; 8 * 1024 * 1024];
    project.files.push(ProjectFile {
        name: "large.step".into(),
        size: bytes.len() as u64,
        kind: "attachment".into(),
        data: None,
        sha256: Some(crate::artifacts::digest(&bytes)),
    });
    for _ in 0..1000 {
        let parent = project.revisions.last().map(|revision| revision.id.clone());
        let id = uuid::Uuid::new_v4().to_string();
        project.revisions.push(serde_json::from_value(serde_json::json!({
            "id":id,"parent":parent,"createdAt":"2026-10-04T00:00:00Z","prompt":"Fixture revision",
            "parameters":{"kind":"box","width":40,"depth":20,"height":10,"thickness":1,"holeDiameter":0,"holes":0}
        })).unwrap());
    }
    project.current_revision = project.revisions.last().map(|revision| revision.id.clone());
    let started = std::time::Instant::now();
    publish(
        folder.path(),
        stage_with(folder.path(), &project, |_| Ok(bytes.clone())).unwrap(),
    )
    .unwrap();
    let initial_ms = started.elapsed().as_millis();
    let started = std::time::Instant::now();
    for index in 0..3 {
        project.name = format!("Metadata save {index}");
        publish(
            folder.path(),
            stage_with(folder.path(), &project, |_| {
                panic!("unchanged asset is already in CAS")
            })
            .unwrap(),
        )
        .unwrap();
    }
    let metadata_ms = started.elapsed().as_millis();
    let started = std::time::Instant::now();
    let opened = read_folder(folder.path()).unwrap();
    let open_ms = started.elapsed().as_millis();
    assert_eq!(opened.revisions.len(), 1000);
    assert_eq!(
        std::fs::read_dir(folder.path().join("assets"))
            .unwrap()
            .count(),
        1
    );
    println!("storage_v2 fixture: 1000 revisions, 8 MiB asset; initial={initial_ms} ms, 3 metadata saves={metadata_ms} ms, verified warm open={open_ms} ms; asset bytes retained=8388608 vs 33554432 for per-generation copies");
}

#[test]
fn reviewed_folder_snapshot_rejects_replacement_before_import() {
    let folder = tempfile::tempdir().unwrap();
    let project = fixture();
    publish(folder.path(), stage_fixture(folder.path(), &project)).unwrap();
    let (inspected, hash) = inspect_folder(folder.path()).unwrap();
    assert_eq!(inspected.id, project.id);
    assert!(hydrate_checked(folder.path(), Some(&hash)).is_ok());
    let mut next = project.clone();
    next.name = "Changed after review".into();
    publish(folder.path(), stage_fixture(folder.path(), &next)).unwrap();
    assert!(hydrate_checked(folder.path(), Some(&hash))
        .unwrap_err()
        .to_string()
        .contains("PROJECT_FOLDER_CHANGED"));
    assert!(hydrate_checked(folder.path(), Some("bad checksum")).is_err());
    assert_eq!(read_folder(folder.path()).unwrap().name, next.name);
}

#[test]
fn history_capture_rechecks_bytes_against_captured_manifest() {
    let folder = tempfile::tempdir().unwrap();
    let mut staged = stage_fixture(folder.path(), &fixture());
    stage_history(folder.path(), &mut staged, b"original history").unwrap();
    let captured = staged.manifest.clone();
    publish(folder.path(), staged).unwrap();
    let path = folder.path().join(format!(
        "metadata/generations/{}/history.json",
        captured.generation
    ));
    std::fs::write(path, b"tampered history").unwrap();
    assert!(history_for_manifest(folder.path(), &captured).is_err());
}
