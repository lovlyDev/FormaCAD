//! Synthetic representation-copy prototypes, never a production folder writer.
//! Destinations must be new. Failures leave only this invocation's partial target;
//! no recursive cleanup, recovery, app database or source writer lease is used.
//! SQL compares a complete original payload/cursor/archive snapshot immediately
//! before the target commit. This is optimistic, with no atomic transaction
//! spanning source and destination and no production writer/lifecycle claim.
use super::candidate;
use crate::{
    core::{AppError, Result},
    models::{Project, ProjectFile},
    project_history,
    project_repository::{folder::FolderRepository, location::LocationIdentity},
    project_storage_v2 as storage,
};
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    fs,
    future::Future,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_METADATA: i64 = 32 * 1024 * 1024;
const MAX_CURSOR: i64 = 16 * 1024 * 1024;
const MAX_REFERENCED_ASSETS: u64 = 100 * 1024 * 1024;

pub(super) async fn copy_manifest(source_root: &Path, dest_new_root: &Path) -> Result<Project> {
    copy_manifest_inner(source_root, dest_new_root, || Ok(())).await
}

async fn copy_manifest_inner(
    source_root: &Path,
    dest_new_root: &Path,
    before_recheck: impl FnOnce() -> Result<()>,
) -> Result<Project> {
    let source = FolderRepository::open(source_root)?;
    let captured = source.capture()?;
    let history = source.read_history(&captured)?;
    let mut copied = captured.project().clone();
    copied.id = uuid::Uuid::new_v4().to_string();
    copied.validate()?;
    let copied_files = unique_files(&copied)?;
    let destination = create_destination(source_root, dest_new_root)?;
    let mut staged = storage::stage_with(dest_new_root, &copied, |file| {
        source.read_asset(&captured, file)
    })?;
    if let Some(history) = &history {
        storage::stage_history(dest_new_root, &mut staged, history)?;
    }
    before_recheck()?;
    source.recheck(&captured)?;
    if !destination.recheck(dest_new_root, Path::new(""))? {
        return Err(changed());
    }
    storage::publish(dest_new_root, staged)?;
    let reader = FolderRepository::open(dest_new_root)?;
    let actual = reader.capture()?;
    if serde_json::to_vec(actual.project())? != serde_json::to_vec(&copied)?
        || reader.read_history(&actual)? != history
    {
        return Err(changed());
    }
    for file in copied_files {
        reader.read_asset(&actual, file)?;
    }
    reader.recheck(&actual)?;
    Ok(copied)
}

pub(super) async fn copy_sql(source_root: &Path, dest_new_root: &Path) -> Result<Project> {
    copy_sql_inner(source_root, dest_new_root, || std::future::ready(Ok(()))).await
}

pub(super) struct SqlSnapshot {
    project: Project,
    payload: String,
    cursor: Option<String>,
    archive: Vec<u8>,
}

impl SqlSnapshot {
    pub(super) fn project(&self) -> &Project {
        &self.project
    }
    pub(super) fn archive(&self) -> &[u8] {
        &self.archive
    }
    pub(super) fn matches(&self, other: &Self) -> bool {
        self.payload == other.payload
            && self.cursor == other.cursor
            && self.archive == other.archive
    }
}

async fn copy_sql_inner<Hook, Pending>(
    source_root: &Path,
    dest_new_root: &Path,
    before_recheck: Hook,
) -> Result<Project>
where
    Hook: FnOnce() -> Pending,
    Pending: Future<Output = Result<()>>,
{
    let source_location = LocationIdentity::capture(source_root, Path::new(""))?;
    let captured = read_sql(source_root).await?;
    if !source_location.recheck(source_root, Path::new(""))? {
        return Err(changed());
    }
    let mut copied = captured.project.clone();
    copied.id = uuid::Uuid::new_v4().to_string();
    copied.validate()?;
    let destination = create_destination(source_root, dest_new_root)?;
    for file in unique_files(&captured.project)? {
        let bytes = read_asset(source_root, file)?;
        let relative = asset_relative(file)?;
        crate::artifacts::immutable_write(dest_new_root, &relative, &bytes)?;
    }
    let writer = candidate::create(&dest_new_root.join("project.sqlite")).await?;
    let result: Result<()> = async {
        let mut transaction = writer.begin().await?;
        sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?)")
            .bind(&copied.id)
            .bind(&copied.name)
            .bind(serde_json::to_string(&copied)?)
            .bind(&copied.updated_at)
            .execute(&mut *transaction)
            .await?;
        project_history::import_history(&mut transaction, &copied, &captured.archive).await?;
        before_recheck().await?;
        if !source_location.recheck(source_root, Path::new(""))? {
            return Err(changed());
        }
        let current = read_sql(source_root).await?;
        if captured.payload != current.payload
            || captured.cursor != current.cursor
            || captured.archive != current.archive
        {
            return Err(changed());
        }
        if !source_location.recheck(source_root, Path::new(""))?
            || !destination.recheck(dest_new_root, Path::new(""))?
        {
            return Err(changed());
        }
        transaction.commit().await?;
        Ok(())
    }
    .await;
    writer.close().await;
    result?;
    let verified = read_sql(dest_new_root).await?;
    if serde_json::to_vec(&verified.project)? != serde_json::to_vec(&copied)?
        || verified.archive != captured.archive
    {
        return Err(changed());
    }
    for file in unique_files(&verified.project)? {
        read_asset(dest_new_root, file)?;
    }
    Ok(copied)
}

pub(super) async fn read_sql(root: &Path) -> Result<SqlSnapshot> {
    let path = crate::security::guarded(root, Path::new("project.sqlite"))?;
    let reader = candidate::open_reader(&path).await?;
    let result = capture_sql(&reader).await;
    reader.close().await;
    result
}

async fn capture_sql(reader: &SqlitePool) -> Result<SqlSnapshot> {
    let mut transaction = reader.begin().await?;
    // All captured fields and exported events belong to this one read snapshot.
    // Bound payload/cursor before retrieving text; this is still a controlled
    // prototype, not a general hostile-database/parser or lifecycle contract.
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT substr(id,1,37),length(CAST(payload AS BLOB)) FROM projects LIMIT 2",
    )
    .fetch_all(&mut *transaction)
    .await?;
    if rows.len() != 1 || rows[0].1 > MAX_METADATA {
        return Err(changed());
    }
    let id = &rows[0].0;
    crate::security::valid_id(id)?;
    let payload: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id=?")
        .bind(id)
        .fetch_one(&mut *transaction)
        .await?;
    let project: Project = serde_json::from_str(&payload)?;
    project.validate()?;
    if project.id != *id {
        return Err(changed());
    }
    unique_files(&project)?;
    let size: Option<i64> = sqlx::query_scalar(
        "SELECT length(CAST(state_json AS BLOB)) FROM project_history WHERE project_id=?",
    )
    .bind(id)
    .fetch_optional(&mut *transaction)
    .await?;
    if size.is_some_and(|size| size > MAX_CURSOR) {
        return Err(changed());
    }
    // Bound event text before export_history fetches rows or allocates an
    // escaped JSON buffer. Six bytes per input byte plus fixed key overhead is
    // conservative for JSON string escaping. This prototype may reject a large
    // whitespace-heavy cursor rather than allocating beyond the archive budget.
    let (count, text_bytes): (i64, i64) = sqlx::query_as("SELECT COUNT(*),COALESCE(SUM(length(CAST(kind AS BLOB))+COALESCE(length(CAST(from_revision AS BLOB)),0)+COALESCE(length(CAST(to_revision AS BLOB)),0)+COALESCE(length(CAST(target_revision AS BLOB)),0)+length(CAST(created_at AS BLOB))),0) FROM (SELECT kind,from_revision,to_revision,target_revision,created_at FROM project_history_events WHERE project_id=? ORDER BY id LIMIT 30001)")
        .bind(id).fetch_one(&mut *transaction).await?;
    let archive_bound = text_bytes
        .checked_mul(6)
        .and_then(|value| {
            count
                .checked_mul(192)
                .and_then(|overhead| value.checked_add(overhead))
        })
        .and_then(|value| {
            size.unwrap_or(0)
                .checked_mul(6)
                .and_then(|cursor| value.checked_add(cursor))
        })
        .and_then(|value| value.checked_add(256));
    if count > 30000 || archive_bound.is_none_or(|bound| !(0..=MAX_CURSOR).contains(&bound)) {
        return Err(AppError::Invalid("HISTORY_ARCHIVE_TOO_LARGE".into()));
    }
    let cursor: Option<String> =
        sqlx::query_scalar("SELECT state_json FROM project_history WHERE project_id=?")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?;
    let archive = project_history::export_history(&mut transaction, &project).await?;
    project_history::validate_archive(&project, &archive)?;
    transaction.commit().await?;
    Ok(SqlSnapshot {
        project,
        payload,
        cursor,
        archive,
    })
}

fn unique_files(project: &Project) -> Result<Vec<&ProjectFile>> {
    let mut hashes = HashMap::new();
    let mut unique = Vec::new();
    let mut referenced = 0u64;
    for file in &project.files {
        referenced = referenced.checked_add(file.size).ok_or_else(changed)?;
        if referenced > MAX_REFERENCED_ASSETS {
            return Err(changed());
        }
        let hash = file
            .sha256
            .as_ref()
            .filter(|hash| crate::artifacts::valid_digest(hash))
            .ok_or_else(changed)?;
        if file.data.is_some() || file.size > crate::artifacts::MAX_FILE_BYTES as u64 {
            return Err(changed());
        }
        match hashes.insert(hash, file.size) {
            Some(size) if size != file.size => return Err(changed()),
            Some(_) => {}
            None => unique.push(file),
        }
    }
    Ok(unique)
}

fn asset_relative(file: &ProjectFile) -> Result<PathBuf> {
    let hash = file
        .sha256
        .as_ref()
        .filter(|hash| crate::artifacts::valid_digest(hash))
        .ok_or_else(changed)?;
    Ok(PathBuf::from(format!("assets/{hash}")))
}

pub(super) fn read_asset(root: &Path, file: &ProjectFile) -> Result<Vec<u8>> {
    if file.size > crate::artifacts::MAX_FILE_BYTES as u64 {
        return Err(changed());
    }
    let location = LocationIdentity::capture(root, Path::new("assets"))?;
    let path = crate::security::guarded(root, &asset_relative(file)?)?;
    let metadata = fs::symlink_metadata(&path)?;
    if !metadata.is_file() || metadata.len() != file.size {
        return Err(changed());
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(changed());
        }
    }
    let source = fs::File::open(path)?;
    let metadata = source.metadata()?;
    if !metadata.is_file() || metadata.len() != file.size {
        return Err(changed());
    }
    let mut bytes = Vec::new();
    source.take(file.size + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != file.size
        || file.sha256.as_deref() != Some(crate::artifacts::digest(&bytes).as_str())
        || !location.recheck(root, Path::new("assets"))?
    {
        return Err(changed());
    }
    Ok(bytes)
}

fn create_destination(source_root: &Path, destination: &Path) -> Result<LocationIdentity> {
    if !destination.is_absolute() {
        return Err(changed());
    }
    let parent = destination.parent().ok_or_else(changed)?;
    let name = destination.file_name().ok_or_else(changed)?;
    let parent_identity = LocationIdentity::capture(parent, Path::new(""))?;
    let guarded = crate::security::guarded(parent, Path::new(name))?;
    if guarded.starts_with(source_root.canonicalize()?) {
        return Err(changed());
    }
    fs::create_dir(&guarded)?;
    if !parent_identity.recheck(parent, Path::new(""))? {
        return Err(changed());
    }
    LocationIdentity::capture(destination, Path::new(""))
}

fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

#[cfg(test)]
#[path = "copy_tests.rs"]
mod tests;
