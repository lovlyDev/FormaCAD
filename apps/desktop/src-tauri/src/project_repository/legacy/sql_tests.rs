use super::*;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

#[tokio::test]
async fn one_read_transaction_never_mixes_committed_payload_and_later_journal() {
    let temp = tempfile::tempdir().unwrap();
    let pool = SqlitePoolOptions::new()
        .max_connections(3)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(temp.path().join("interleave.sqlite"))
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await
        .unwrap();
    sqlx::query("CREATE TABLE projects(id TEXT PRIMARY KEY,payload TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE project_storage_commits(project_id TEXT,base_manifest_sha256 TEXT,target_manifest TEXT,project_sha256 TEXT)").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO projects VALUES('id','old')")
        .execute(&pool)
        .await
        .unwrap();
    let mut reader = pool.begin().await.unwrap();
    let first: String = sqlx::query_scalar("SELECT payload FROM projects WHERE id='id'")
        .fetch_one(&mut *reader)
        .await
        .unwrap();
    assert_eq!(first, "old");
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("UPDATE projects SET payload='new' WHERE id='id'")
        .execute(&mut *writer)
        .await
        .unwrap();
    sqlx::query("INSERT INTO project_storage_commits VALUES('id',NULL,'target','hash')")
        .execute(&mut *writer)
        .await
        .unwrap();
    writer.commit().await.unwrap();
    let old = read_transaction(&mut reader, "id", false).await.unwrap();
    assert_eq!(old.project, "old");
    assert!(old.journal.is_none());
    reader.rollback().await.unwrap();
    let new = read(&pool, "id", false).await.unwrap();
    assert_eq!(new.project, "new");
    assert_eq!(new.journal, Some(()));
}

#[tokio::test]
async fn legacy_raw_payload_larger_than_portable_metadata_is_not_truncated() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE projects(id TEXT PRIMARY KEY,payload TEXT)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE project_storage_commits(project_id TEXT,base_manifest_sha256 TEXT,target_manifest TEXT,project_sha256 TEXT)").execute(&pool).await.unwrap();
    let bytes = "A".repeat(20 * 1024 * 1024);
    sqlx::query("INSERT INTO projects VALUES('id',?)")
        .bind(&bytes)
        .execute(&pool)
        .await
        .unwrap();
    let captured = read(&pool, "id", false).await.unwrap();
    assert_eq!(captured.project, bytes);
    assert!(captured.history.is_none());
}
