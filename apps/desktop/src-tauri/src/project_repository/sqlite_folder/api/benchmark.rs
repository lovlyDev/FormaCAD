//! Opt-in actual FCP1 read timings. Synthetic bytes are not CAD or hardware proof.
use super::*;
use crate::{project_history, project_repository::benchmark::fixture};
use std::time::Instant;

async fn sample(source: &FolderProject, complete: bool) -> u128 {
    let started = Instant::now();
    let view = if complete {
        source.capture().await.unwrap()
    } else {
        source.capture_metadata().await.unwrap()
    };
    source.recheck(&view).await.unwrap();
    std::hint::black_box(view.commit_id());
    drop(view);
    started.elapsed().as_nanos()
}

fn distribution(samples: &mut [u128]) -> serde_json::Value {
    samples.sort_unstable();
    let rank = |percent: usize| (samples.len() * percent).div_ceil(100) - 1;
    serde_json::json!({"samples":samples.len(),
        "median_ms":samples[rank(50)] as f64 / 1_000_000.,
        "p95_ms":samples[rank(95)] as f64 / 1_000_000.})
}

#[tokio::test]
#[ignore = "explicit real FCP1 benchmark; raw timings, no performance thresholds"]
async fn measure_scoped_reads() {
    println!(
        "FCP1_READ_BENCHMARK {}",
        serde_json::json!({
            "kind":"environment", "os":std::env::consts::OS,"arch":std::env::consts::ARCH,
            "profile":"cargo test debug; not release-app performance",
            "filesystem":"runner temporary directory; caches not flushed; no USB/NAS claim",
            "provenance":"synthetic saved revision/chat/STEP-labelled bytes; no CAD geometry",
            "operation":"actual FolderProject capture + recheck + view drop",
            "validation":"metadata validates declared seals; complete also validates archive bytes; contracts differ"
        })
    );
    for (label, revisions, attachments, asset_bytes) in [
        ("small", 4, 4, 4096),
        ("medium", 32, 64, 32768),
        ("large", 128, 256, 131072),
    ] {
        let setup = tempfile::tempdir().unwrap();
        let setup_path = setup.path().to_owned();
        let target = tempfile::tempdir().unwrap();
        let (pool, project, assets) =
            fixture::build(setup.path(), revisions, attachments, asset_bytes).await;
        let archive = project_history::export_history(&mut pool.acquire().await.unwrap(), &project)
            .await
            .unwrap();
        project_history::validate_archive(&project, &archive).unwrap();
        let archive_bytes = archive.len();
        let project_bytes = serde_json::to_vec(&project).unwrap().len();
        let unique_asset_bytes: usize = assets.values().map(Vec::len).sum();
        let unique_assets = assets.len();
        pool.close().await;
        let saved = FolderProject::create(
            target.path(),
            project,
            assets.into_iter().collect(),
            Some(archive),
        )
        .await
        .unwrap();
        assert!(matches!(saved, Publication::Committed(_)));
        drop(saved);
        drop(setup);
        assert!(
            !setup_path.exists(),
            "temporary setup SQL must not remain available to FCP1"
        );
        let before =
            crate::artifacts::digest(&std::fs::read(target.path().join("project.sqlite")).unwrap());
        let started = Instant::now();
        let source = FolderProject::open(target.path()).await.unwrap();
        let open_ms = started.elapsed().as_secs_f64() * 1000.;
        // Warm both paths explicitly, then alternate their order to reduce ordering bias.
        sample(&source, false).await;
        sample(&source, true).await;
        let mut metadata = Vec::new();
        let mut complete = Vec::new();
        for index in 0..12 {
            if index % 2 == 0 {
                metadata.push(sample(&source, false).await);
                complete.push(sample(&source, true).await);
            } else {
                complete.push(sample(&source, true).await);
                metadata.push(sample(&source, false).await);
            }
        }
        let view = source.capture_metadata().await.unwrap();
        let history = source.read_history(&view).await.unwrap();
        assert_eq!(history.len(), archive_bytes);
        project_history::validate_archive(view.project(), &history).unwrap();
        source.recheck(&view).await.unwrap();
        source.close();
        assert!(source.recheck(&view).await.is_err());
        assert_eq!(
            before,
            crate::artifacts::digest(&std::fs::read(target.path().join("project.sqlite")).unwrap())
        );
        println!(
            "FCP1_READ_BENCHMARK {}",
            serde_json::json!({
                "kind":"timings", "size":label, "revisions":revisions,
                "attachment_references":attachments,"unique_assets":unique_assets,
                "unique_asset_bytes":unique_asset_bytes,"history_archive_bytes":archive_bytes,
                "project_json_bytes":project_bytes,"first_api_open_ms":open_ms,
                "setup_sql_removed":true,"project_sql_unchanged":true,
                "metadata_capture_recheck":distribution(&mut metadata),
                "complete_capture_recheck":distribution(&mut complete)
            })
        );
    }
}
