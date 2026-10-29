//! Opt-in storage representation comparison, not a production folder backend.
//! Run with --ignored --nocapture; every output line starts STORAGE_BENCHMARK.
mod fixture;
mod measurement;

use crate::{project_history, project_storage_v2 as folder};
use measurement::{bytes_on_disk, measured, row};
use std::time::Instant;

#[tokio::test]
#[ignore = "explicit benchmark; emits raw timings without performance thresholds"]
async fn compare_storage_representations() {
    println!(
        "STORAGE_BENCHMARK {}",
        serde_json::json!({
            "kind":"environment", "os":std::env::consts::OS,
            "arch":std::env::consts::ARCH,
            "logical_cpus":std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
            "filesystem":"runner temporary directory; filesystem cache not flushed",
            "cpu_model":"collect runner metadata separately", "disk_type":"not detected",
            "provenance":"synthetic model metadata/chat/binary STEP-labelled payload; no CAD geometry claim",
            "scope":"existing APIs have different validation work; timings do not establish semantic equivalence or select a format"
        })
    );
    for (label, revisions, attachments, asset_bytes) in [
        ("small", 4, 4, 4096),
        ("medium", 32, 64, 32768),
        ("large", 128, 256, 131072),
    ] {
        let root = tempfile::tempdir().unwrap();
        let manifest_root = root.path().join("manifest");
        let sql_root = root.path().join("sql-candidate");
        std::fs::create_dir(&manifest_root).unwrap();
        std::fs::create_dir(&sql_root).unwrap();
        let (pool, project, assets) =
            fixture::build(&sql_root, revisions, attachments, asset_bytes).await;
        let archive = project_history::export_history(&mut pool.acquire().await.unwrap(), &project)
            .await
            .unwrap();
        project_history::validate_archive(&project, &archive).unwrap();
        let mut staged = folder::stage_with(&manifest_root, &project, |file| {
            Ok(assets[file.sha256.as_ref().unwrap()].clone())
        })
        .unwrap();
        folder::stage_history(&manifest_root, &mut staged, &archive).unwrap();
        folder::publish(&manifest_root, staged).unwrap();
        pool.close().await;
        println!(
            "STORAGE_BENCHMARK {}",
            serde_json::json!({"kind":"fixture","size":label,
            "revisions":revisions,"attachment_references":attachments,"unique_assets":assets.len(),
            "asset_bytes":asset_bytes,"history_archive_bytes":archive.len(),
            "project_json_bytes":serde_json::to_vec(&project).unwrap().len()})
        );

        measured(label, "manifest-v2", "first_api_open", || {
            folder::read_folder(&manifest_root).unwrap()
        });
        measured(label, "manifest-v2-metadata", "first_api_open", || {
            folder::reader::FolderSnapshot::capture(&manifest_root).unwrap()
        });
        let started = Instant::now();
        let reopened = crate::storage::open(&sql_root.join("project.sqlite"))
            .await
            .unwrap();
        let current = fixture::read_project(&reopened, &project.id).await;
        row(
            label,
            "sqlite-candidate",
            "first_api_open",
            started.elapsed(),
            "includes existing DB integrity/migrations/session recovery route; not a pure reader",
        );
        assert_eq!(
            serde_json::to_vec(&current).unwrap(),
            serde_json::to_vec(&project).unwrap()
        );
        measured(label, "manifest-v2", "history_status", || {
            let saved = folder::read_folder(&manifest_root).unwrap();
            let bytes = folder::read_history(&manifest_root).unwrap().unwrap();
            project_history::validate_archive(&saved, &bytes).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let cursor: project_history::HistoryState =
                serde_json::from_value(value["state"].clone()).unwrap();
            (cursor.can_undo(), cursor.can_redo())
        });
        let started = Instant::now();
        let metadata_snapshot = folder::reader::FolderSnapshot::capture(&manifest_root).unwrap();
        let bytes = metadata_snapshot
            .read_history(&manifest_root)
            .unwrap()
            .unwrap();
        // read_history validates the complete event archive before returning it.
        // Format v2 has no separate cursor entry: extracting these flags still
        // scans/deserializes the event archive, although it avoids unrelated assets.
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let cursor: project_history::HistoryState =
            serde_json::from_value(value["state"].clone()).unwrap();
        std::hint::black_box((cursor.can_undo(), cursor.can_redo()));
        row(label, "manifest-v2-metadata", "history_status", started.elapsed(),
            "captures metadata and validates/scans complete v2 history event archive; no unrelated asset reads; not an independent fast cursor");
        let started = Instant::now();
        let file = &metadata_snapshot.project().files[0];
        let asset = metadata_snapshot.read_asset(&manifest_root, file).unwrap();
        assert_eq!(asset.len() as u64, file.size);
        std::hint::black_box(asset);
        row(label, "manifest-v2-metadata", "single_attachment_public_api", started.elapsed(),
            "uses existing captured metadata; validates one asset with bounded size/hash plus before/after metadata recheck");
        measured(label, "manifest-v2-metadata", "snapshot_recheck", || {
            metadata_snapshot.recheck(&manifest_root).unwrap()
        });
        let started = Instant::now();
        let cursor = project_history::load_state(&reopened, &current)
            .await
            .unwrap();
        std::hint::black_box((cursor.can_undo(), cursor.can_redo()));
        row(
            label,
            "sqlite-candidate",
            "history_status",
            started.elapsed(),
            "validates typed cursor; does not hash assets or full event archive",
        );

        let before_manifest = bytes_on_disk(&manifest_root);
        let before_sql = bytes_on_disk(&sql_root);
        let mut renamed = project.clone();
        renamed.name = "Metadata update".into();
        measured(label, "manifest-v2", "metadata_save", || {
            let mut staged = folder::stage_with(&manifest_root, &renamed, |file| {
                Ok(assets[file.sha256.as_ref().unwrap()].clone())
            })
            .unwrap();
            folder::stage_history(&manifest_root, &mut staged, &archive).unwrap();
            folder::publish(&manifest_root, staged).unwrap();
        });
        let started = Instant::now();
        fixture::persist(&reopened, Some(&current), &renamed).await;
        row(label,"sqlite-candidate","metadata_save",started.elapsed(),"same typed record_commit validation; existing SQL schema only; no production writer/session adapter");
        measured(label, "manifest-v2", "explicit_previous_read", || {
            let previous = folder::read_backup(&manifest_root).unwrap();
            assert_eq!(previous.name, project.name);
        });
        measured(label, "manifest-v2", "hydrate_export_input", || {
            let (hydrated, history) = folder::hydrate_with_history(&manifest_root).unwrap();
            project_history::validate_archive(&hydrated, &history.unwrap()).unwrap();
            assert!(hydrated.files.iter().all(|file| file.data.is_some()));
        });
        println!(
            "STORAGE_BENCHMARK {}",
            serde_json::json!({"kind":"growth","size":label,
            "manifest_before_bytes":before_manifest,"manifest_after_bytes":bytes_on_disk(&manifest_root),
            "sqlite_before_bytes":before_sql,"sqlite_after_bytes":bytes_on_disk(&sql_root),
            "sqlite_includes_active_wal":true,
            "manifest_assets_after":std::fs::read_dir(manifest_root.join("assets")).unwrap().count(),
            "expected_unique_assets":assets.len()})
        );
        for backend in ["manifest-v2", "manifest-v2-metadata", "sqlite-candidate"] {
            for operation in [
                "single_attachment_public_api",
                "copy_export_transaction",
                "crash_recovery",
                "process_cold_open",
                "peak_memory",
            ] {
                if backend == "manifest-v2-metadata" && operation == "single_attachment_public_api"
                {
                    continue;
                }
                println!(
                    "STORAGE_BENCHMARK {}",
                    serde_json::json!({"kind":"unsupported","size":label,"backend":backend,"operation":operation,
                    "reason":"no comparable implemented production route in this benchmark; not measured"})
                );
            }
        }
        reopened.close().await;
        assert_eq!(
            std::fs::read_dir(manifest_root.join("assets"))
                .unwrap()
                .count(),
            assets.len()
        );
        root.close().unwrap();
    }
    fixture::reject_limits();
}
