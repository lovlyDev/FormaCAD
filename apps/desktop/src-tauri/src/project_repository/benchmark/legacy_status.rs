//! End-to-end public history-status observations over real managed storage.
//! Setup/materialization and source-purity scans are outside the measured call.
//! This is the existing app-schema LegacyManaged backend, not the SQL candidate.
use crate::{
    core::{AppState, Result},
    models::Project,
    project_history,
    project_repository::legacy::read_test_fixture as fixture,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Debug, PartialEq)]
struct SourceProof {
    logical: (String, String, i64, i64),
    project_ids: Vec<String>,
    history_sha256: String,
    files: BTreeMap<PathBuf, (u64, String)>,
    access_mode: String,
}

fn file_proof(root: &Path) -> BTreeMap<PathBuf, (u64, String)> {
    fn visit(root: &Path, at: &Path, proof: &mut BTreeMap<PathBuf, (u64, String)>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name();
            // Physical app DB/WAL bytes are not a project-purity definition.
            // Its exact logical source payload/cursor/archive are captured too.
            // Access owner hints and OS lock bytes are not saved CAD data.
            if at == root
                && (name == ".project-access" || name.to_string_lossy().starts_with("read.sqlite"))
            {
                continue;
            }
            let path = entry.path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            let metadata = std::fs::symlink_metadata(&path).unwrap();
            assert!(!metadata.file_type().is_symlink());
            if metadata.is_dir() {
                proof.insert(relative, (0, "directory".into()));
                visit(root, &path, proof);
            } else {
                assert!(metadata.is_file());
                let mut file = std::fs::File::open(&path).unwrap();
                let mut hash = Sha256::new();
                let mut buffer = [0u8; 64 * 1024];
                let mut read = 0u64;
                loop {
                    let count = file.read(&mut buffer).unwrap();
                    if count == 0 {
                        break;
                    }
                    read += count as u64;
                    hash.update(&buffer[..count]);
                }
                assert_eq!(read, metadata.len());
                proof.insert(relative, (read, format!("{:x}", hash.finalize())));
            }
        }
    }
    let mut proof = BTreeMap::new();
    visit(root, root, &mut proof);
    proof
}

async fn source_proof(state: &AppState, project: &Project) -> SourceProof {
    let archive = {
        let mut connection = state.pool.acquire().await.unwrap();
        project_history::export_history(&mut connection, project)
            .await
            .unwrap()
    };
    project_history::validate_archive(project, &archive).unwrap();
    SourceProof {
        logical: fixture::logical(state, &project.id).await,
        project_ids: sqlx::query_scalar("SELECT id FROM projects ORDER BY id")
            .fetch_all(&state.pool)
            .await
            .unwrap(),
        history_sha256: crate::artifacts::digest(&archive),
        files: file_proof(&state.root),
        access_mode: state
            .project_access
            .status(&state.root, &project.id)
            .unwrap()
            .mode,
    }
}

async fn build(
    state: &AppState,
    revision_count: usize,
    attachment_count: usize,
    asset_bytes: usize,
    message_bytes: usize,
) -> Project {
    let files: Vec<_> = (0..attachment_count).map(|index| {
        let mut bytes = vec![b'x'; asset_bytes];
        bytes[..8].copy_from_slice(&((index / 2) as u64).to_le_bytes());
        serde_json::json!({"name":format!("fixture-{index}.step"),"kind":"source","size":asset_bytes,
            "data":crate::artifacts::data_url("fixture.step",&bytes),"sha256":crate::artifacts::digest(&bytes)})
    }).collect();
    let messages: Vec<_> = (0..revision_count).map(|_| serde_json::json!({
        "id":uuid::Uuid::new_v4().to_string(),"role":"user","text":"m".repeat(message_bytes),
        "createdAt":"2026-10-31T00:00:00Z"
    })).collect();
    let mut project: Project = serde_json::from_value(serde_json::json!({
        "schemaVersion":1,"id":uuid::Uuid::new_v4().to_string(),"name":"Managed status benchmark",
        "units":"mm","agent":"codex","pinned":false,"createdAt":"2026-10-31T00:00:00Z",
        "updatedAt":"2026-10-31T00:00:00Z","currentRevision":null,"revisions":[],
        "messages":messages,"files":files,"exports":[]
    }))
    .unwrap();
    project.validate().unwrap();
    let pending = crate::artifacts::normalize(&mut project).unwrap();
    project.validate().unwrap();
    // Real app-schema SQL transactions and the existing typed history writer;
    // no folder is published yet, so there is no stale mirror to reconcile.
    super::fixture::persist(&state.pool, None, &project).await;
    for index in 0..revision_count {
        let previous = project.clone();
        let revision = serde_json::from_value(serde_json::json!({
            "id":uuid::Uuid::new_v4().to_string(),"parent":project.current_revision,
            "createdAt":"2026-10-31T00:00:00Z","prompt":format!("Synthetic storage revision {index}"),
            "source":"fixture-0.step","parameters":{"kind":"box","width":40+index,"depth":20,
                "height":10,"thickness":2,"holeDiameter":0,"holes":0}
        })).unwrap();
        project.revisions.push(revision);
        project.current_revision = Some(project.revisions.last().unwrap().id.clone());
        super::fixture::persist(&state.pool, Some(&previous), &project).await;
    }
    // The real persistence path materializes .blobs, attachment mirrors, CAS,
    // manifest, metadata, revisions and typed history before any timing begins.
    let saved = crate::projects::persist(state, project, pending, None)
        .await
        .unwrap();
    assert!(saved.files.iter().all(|file| file.data.is_none()));
    assert!(state.root.join(&saved.id).join("manifest.json").is_file());
    saved
}

pub(super) async fn measure() -> Result<()> {
    for (label, revisions, attachments, asset_bytes, message_bytes) in [
        ("small", 4, 4, 4096, 256),
        ("medium", 32, 64, 32768, 4096),
        ("large", 128, 256, 131072, 8192),
    ] {
        let root = tempfile::tempdir()?;
        let state = fixture::state(root.path()).await;
        let project = build(&state, revisions, attachments, asset_bytes, message_bytes).await;
        let before = source_proof(&state, &project).await;
        assert_eq!(before.access_mode, "closed");
        let started = Instant::now();
        // No Project preload or cursor load is substituted for this public API.
        let status = project_history::status_for(&state, &project.id).await?;
        let elapsed_ns = started.elapsed().as_nanos();
        assert!(status.can_undo);
        assert!(!status.can_redo);
        assert_eq!(status.expected_revision, project.current_revision);
        let after = source_proof(&state, &project).await;
        assert_eq!(after, before);
        let unique: BTreeMap<_, _> = project
            .files
            .iter()
            .map(|file| (file.sha256.as_ref().unwrap(), file.size))
            .collect();
        let cas_unique_bytes: u64 = unique.values().sum();
        let chat_utf8_bytes: usize = project
            .messages
            .iter()
            .map(|message| message.text.len())
            .sum();
        println!(
            "STORAGE_BENCHMARK {}",
            serde_json::json!({
                "kind":"measurement","backend":"legacy-managed","operation":"public_history_status_end_to_end",
                "size":label,"elapsed_ns":elapsed_ns,"samples":1,"revision_count":project.revisions.len(),
                "message_count":project.messages.len(),"chat_utf8_bytes":chat_utf8_bytes,
                "attachment_references":project.files.len(),"unique_assets":unique.len(),"cas_unique_bytes":cas_unique_bytes,
                "logical_attachment_bytes":project.files.iter().map(|file|file.size).sum::<u64>(),
                "project_json_bytes":serde_json::to_vec(&project)?.len(),"source_purity_verified":true,
                "source_history_sha256":before.history_sha256,"source_model_entries":before.files.len(),
                "note":"actual public project_history::status_for against materialized managed AppState/app schema; setup and source-purity hashes outside timer; purity scan warms OS caches; no independent-format, cold-cache, CAD-geometry or per-operation peak-memory claim"
            })
        );
        state.pool.close().await;
        drop(state);
        root.close()?;
    }
    Ok(())
}
