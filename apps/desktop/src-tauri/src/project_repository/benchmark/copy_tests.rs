use super::{copy_manifest, copy_manifest_inner, copy_sql, copy_sql_inner, read_sql};
use crate::{
    core::Result,
    models::Project,
    project_repository::benchmark::{candidate, fixture},
    project_storage_v2 as folder,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[tokio::test]
async fn excessive_history_text_is_rejected_before_archive_materialization() {
    let parent = tempfile::tempdir().unwrap();
    let (sql_root, _, project, _) = sources(parent.path()).await;
    let writer = candidate::open_writer(&sql_root.join("project.sqlite"))
        .await
        .unwrap();
    // Deliberately corrupt only this fixture's external database, without a
    // large Rust buffer, to exercise admission before fetching event strings.
    sqlx::query(
        "UPDATE project_history_events SET created_at=zeroblob(3145728) WHERE project_id=?",
    )
    .bind(&project.id)
    .execute(&writer)
    .await
    .unwrap();
    writer.close().await;
    let before = files(&sql_root);
    let destination = parent.path().join("refused-copy");
    let error = copy_sql(&sql_root, &destination).await.err().unwrap();
    assert_eq!(error.to_string(), "HISTORY_ARCHIVE_TOO_LARGE");
    assert!(!destination.exists());
    assert_eq!(files(&sql_root), before);
}

#[tokio::test]
async fn excessive_asset_references_are_refused_without_reading_or_creating_assets() {
    let parent = tempfile::tempdir().unwrap();
    let (sql_root, _, mut project, _) = sources(parent.path()).await;
    project.files.truncate(1);
    project.files[0].size = crate::artifacts::MAX_FILE_BYTES as u64;
    for index in 0..2 {
        let mut file = project.files[0].clone();
        file.name = format!("excess-{index}.step");
        project.files.push(file);
    }
    project.validate().unwrap();
    let writer = candidate::open_writer(&sql_root.join("project.sqlite"))
        .await
        .unwrap();
    sqlx::query("UPDATE projects SET payload=? WHERE id=?")
        .bind(serde_json::to_string(&project).unwrap())
        .bind(&project.id)
        .execute(&writer)
        .await
        .unwrap();
    writer.close().await;
    let before = files(&sql_root);
    let destination = parent.path().join("refused-copy");
    assert!(copy_sql(&sql_root, &destination).await.is_err());
    assert!(!destination.exists());
    assert_eq!(files(&sql_root), before);
}

#[tokio::test]
async fn manifest_reference_budget_is_rejected_before_creating_or_publishing_destination() {
    let parent = tempfile::tempdir().unwrap();
    let seed = parent.path().join("sql-seed");
    let source = parent.path().join("manifest-source");
    fs::create_dir(&seed).unwrap();
    fs::create_dir(&source).unwrap();
    let (writer, mut project, assets) = fixture::build(&seed, 3, 2, 64 * 1024).await;
    project.files.truncate(1);
    // 1601 references cross 100 MiB, but all point to one valid 64 KiB CAS
    // object. Count, per-file and manifest unique-byte limits still pass.
    // No oversized attachment buffer is allocated for this admission test.
    for index in 0..1600 {
        let mut file = project.files[0].clone();
        file.name = format!("repeated-{index}.step");
        project.files.push(file);
    }
    project.validate().unwrap();
    let history =
        crate::project_history::export_history(&mut writer.acquire().await.unwrap(), &project)
            .await
            .unwrap();
    let mut staged = folder::stage_with(&source, &project, |file| {
        Ok(assets[file.sha256.as_ref().unwrap()].clone())
    })
    .unwrap();
    folder::stage_history(&source, &mut staged, &history).unwrap();
    folder::publish(&source, staged).unwrap();
    writer.close().await;
    // Full production read verifies the source itself is intact; refusal must
    // be the copy admission budget, not missing or corrupt attachment bytes.
    assert_eq!(folder::read_folder(&source).unwrap().files.len(), 1601);
    let before = files(&source);
    let destination = parent.path().join("refused-manifest-copy");
    let error = copy_manifest(&source, &destination).await.err().unwrap();
    assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_CHANGED");
    assert!(!destination.exists());
    assert_eq!(files(&source), before);
    parent.close().unwrap();
}

fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, current: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

async fn sources(parent: &Path) -> (PathBuf, PathBuf, Project, usize) {
    let sql_root = parent.join("sql-source");
    let manifest_root = parent.join("manifest-source");
    fs::create_dir(&sql_root).unwrap();
    fs::create_dir(&manifest_root).unwrap();
    let (writer, project, assets) = fixture::build(&sql_root, 3, 4, 256).await;
    let history =
        crate::project_history::export_history(&mut writer.acquire().await.unwrap(), &project)
            .await
            .unwrap();
    let mut staged = folder::stage_with(&manifest_root, &project, |file| {
        Ok(assets[file.sha256.as_ref().unwrap()].clone())
    })
    .unwrap();
    folder::stage_history(&manifest_root, &mut staged, &history).unwrap();
    folder::publish(&manifest_root, staged).unwrap();
    writer.close().await;
    (sql_root, manifest_root, project, assets.len())
}

fn preserved_model(original: &Project, copied: &Project) {
    assert_ne!(original.id, copied.id);
    assert_eq!(original.current_revision, copied.current_revision);
    assert_eq!(
        serde_json::to_vec(&original.revisions).unwrap(),
        serde_json::to_vec(&copied.revisions).unwrap()
    );
    assert_eq!(original.files, copied.files);
    assert_eq!(original.messages, copied.messages);
}

#[tokio::test]
async fn prototypes_copy_real_saved_model_and_archive_to_new_identity_without_source_writes() {
    let parent = tempfile::tempdir().unwrap();
    let (sql_root, manifest_root, original, asset_count) = sources(parent.path()).await;
    let before_sql = files(&sql_root);
    let before_manifest = files(&manifest_root);
    let sql_destination = parent.path().join("sql-copy");
    let manifest_destination = parent.path().join("manifest-copy");
    let copied_sql = copy_sql(&sql_root, &sql_destination).await.unwrap();
    let copied_manifest = copy_manifest(&manifest_root, &manifest_destination)
        .await
        .unwrap();
    preserved_model(&original, &copied_sql);
    preserved_model(&original, &copied_manifest);
    let original_archive = read_sql(&sql_root).await.unwrap().archive;
    let sql = read_sql(&sql_destination).await.unwrap();
    assert_eq!(sql.project.id, copied_sql.id);
    assert_eq!(sql.archive, original_archive);
    let manifest = folder::reader::FolderSnapshot::capture(&manifest_destination).unwrap();
    assert_eq!(manifest.project().id, copied_manifest.id);
    assert_eq!(
        manifest
            .read_history(&manifest_destination)
            .unwrap()
            .unwrap(),
        original_archive
    );
    for file in &manifest.project().files {
        manifest.read_asset(&manifest_destination, file).unwrap();
    }
    assert_eq!(
        fs::read_dir(sql_destination.join("assets"))
            .unwrap()
            .count(),
        asset_count
    );
    assert_eq!(
        fs::read_dir(manifest_destination.join("assets"))
            .unwrap()
            .count(),
        asset_count
    );
    assert_eq!(files(&sql_root), before_sql);
    assert_eq!(files(&manifest_root), before_manifest);
    parent.close().unwrap();
}

#[tokio::test]
async fn existing_destination_and_destination_inside_source_are_refused_without_changes() {
    let parent = tempfile::tempdir().unwrap();
    let (sql_root, manifest_root, _, _) = sources(parent.path()).await;
    let existing = parent.path().join("existing");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("keep.txt"), b"untouched").unwrap();
    let before = files(parent.path());
    assert!(copy_manifest(&manifest_root, &existing).await.is_err());
    assert!(copy_sql(&sql_root, &existing).await.is_err());
    assert!(
        copy_manifest(&manifest_root, &manifest_root.join("forbidden-copy"))
            .await
            .is_err()
    );
    assert!(copy_sql(&sql_root, &sql_root.join("forbidden-copy"))
        .await
        .is_err());
    assert_eq!(files(parent.path()), before);
    parent.close().unwrap();
}

async fn sql_has_project(root: &Path) -> bool {
    if !root.join("project.sqlite").exists() {
        return false;
    }
    let reader = candidate::open_reader(&root.join("project.sqlite"))
        .await
        .unwrap();
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
        .fetch_one(&reader)
        .await
        .unwrap();
    reader.close().await;
    count != 0
}

#[tokio::test]
async fn corrupt_source_asset_cannot_publish_a_destination_model_or_repair_the_source() {
    let parent = tempfile::tempdir().unwrap();
    let (sql_root, manifest_root, original, _) = sources(parent.path()).await;
    let hash = original.files[0].sha256.as_ref().unwrap();
    for root in [&sql_root, &manifest_root] {
        fs::write(root.join("assets").join(hash), b"corrupt").unwrap();
    }
    let before_sql = files(&sql_root);
    let before_manifest = files(&manifest_root);
    let sql_destination = parent.path().join("sql-copy");
    let manifest_destination = parent.path().join("manifest-copy");
    assert!(copy_sql(&sql_root, &sql_destination).await.is_err());
    assert!(copy_manifest(&manifest_root, &manifest_destination)
        .await
        .is_err());
    assert!(!sql_has_project(&sql_destination).await);
    assert!(!manifest_destination.join("manifest.json").exists());
    assert_eq!(files(&sql_root), before_sql);
    assert_eq!(files(&manifest_root), before_manifest);
    parent.close().unwrap();
}

#[tokio::test]
async fn disappearance_after_staging_refuses_publication_and_sql_target_commit() {
    let parent = tempfile::tempdir().unwrap();
    let (sql_root, manifest_root, _, _) = sources(parent.path()).await;
    let before_sql = files(&sql_root);
    let before_manifest = files(&manifest_root);
    let moved_sql = parent.path().join("moved-sql-source");
    let moved_manifest = parent.path().join("moved-manifest-source");
    let manifest_destination = parent.path().join("manifest-copy");
    let sql_destination = parent.path().join("sql-copy");
    let manifest_result = copy_manifest_inner(&manifest_root, &manifest_destination, || {
        fs::rename(&manifest_root, &moved_manifest)?;
        Ok(())
    })
    .await;
    assert!(manifest_result.is_err());
    let sql_result = copy_sql_inner(&sql_root, &sql_destination, || async {
        fs::rename(&sql_root, &moved_sql)?;
        Ok(())
    })
    .await;
    assert!(sql_result.is_err());
    assert!(!manifest_destination.join("manifest.json").exists());
    assert!(!sql_has_project(&sql_destination).await);
    assert_eq!(files(&moved_sql), before_sql);
    assert_eq!(files(&moved_manifest), before_manifest);
    parent.close().unwrap();
}

#[tokio::test]
async fn typed_source_commit_during_copy_is_preserved_and_destination_transaction_rolls_back() {
    let parent = tempfile::tempdir().unwrap();
    let (sql_root, _, original, _) = sources(parent.path()).await;
    let destination = parent.path().join("sql-copy");
    let mut changed = original.clone();
    changed.name = "A newer source commit".into();
    let result = copy_sql_inner(&sql_root, &destination, || async {
        let writer = candidate::open_writer(&sql_root.join("project.sqlite")).await?;
        fixture::persist(&writer, Some(&original), &changed).await;
        writer.close().await;
        Result::Ok(())
    })
    .await;
    assert!(result.is_err());
    let latest = read_sql(&sql_root).await.unwrap();
    assert_eq!(latest.project.name, changed.name);
    assert!(!sql_has_project(&destination).await);
    parent.close().unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn inside_root_asset_directory_symlink_is_refused_without_changing_source_bytes() {
    let parent = tempfile::tempdir().unwrap();
    let (sql_root, _, project, _) = sources(parent.path()).await;
    let assets = sql_root.join("assets");
    let original_assets = sql_root.join("assets-original");
    // Both paths are inside this test's newly created source. Preserve the
    // real asset directory and introduce only an owned, non-cyclic alias.
    fs::rename(&assets, &original_assets).unwrap();
    std::os::unix::fs::symlink("assets-original", &assets).unwrap();
    assert!(assets
        .canonicalize()
        .unwrap()
        .starts_with(sql_root.canonicalize().unwrap()));
    let before = files(&sql_root);
    let before_link = fs::read_link(&assets).unwrap();
    let error = super::read_asset(&sql_root, &project.files[0])
        .err()
        .unwrap();
    assert_eq!(error.to_string(), "PROJECT_SNAPSHOT_CHANGED");
    let destination = parent.path().join("refused-symlink-copy");
    assert!(copy_sql(&sql_root, &destination).await.is_err());
    assert!(!destination.join("project.sqlite").exists());
    assert_eq!(fs::read_link(&assets).unwrap(), before_link);
    assert_eq!(files(&sql_root), before);
    parent.close().unwrap();
}
