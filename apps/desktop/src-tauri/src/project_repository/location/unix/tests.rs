//! Check the kernel lock from another process, not SQLite's in-process cache.
use super::Directory;
use sqlx::{sqlite::SqliteConnectOptions, Connection, Executor, SqliteConnection};
use std::{path::Path, process::Command, time::Duration};

const PROBE: &str = "project_repository::location::platform::tests::directory_lock_probe";

fn probe(path: &Path, expected: &str) {
    let nonce = uuid::Uuid::new_v4().to_string();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", PROBE, "--nocapture"])
        .env("FORMA_DIRECTORY_LOCK_DB", path)
        .env("FORMA_DIRECTORY_LOCK_NONCE", &nonce)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let marker = format!("DIRECTORY_LOCK_PROBE:{nonce}:{expected}");
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .any(|line| line == marker),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[tokio::test]
async fn rejecting_a_database_as_a_directory_preserves_its_sqlite_os_lock() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("lock.sqlite");
    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .busy_timeout(Duration::ZERO);
    let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
    connection
        .execute("CREATE TABLE fixture(value INTEGER)")
        .await
        .unwrap();
    connection.execute("BEGIN IMMEDIATE").await.unwrap();
    probe(&path, "busy");
    assert!(Directory::open(&path).is_err());
    probe(&path, "busy");
    connection.execute("ROLLBACK").await.unwrap();
    probe(&path, "acquired");
    connection.close().await.unwrap();
}

#[tokio::test]
#[ignore = "owned child-process lock observer"]
async fn directory_lock_probe() {
    let path = std::env::var_os("FORMA_DIRECTORY_LOCK_DB").expect("owned probe DB");
    let nonce = std::env::var("FORMA_DIRECTORY_LOCK_NONCE").expect("owned probe nonce");
    uuid::Uuid::parse_str(&nonce).unwrap();
    let options = SqliteConnectOptions::new()
        .filename(Path::new(&path))
        .busy_timeout(Duration::ZERO);
    let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
    let result = connection.execute("BEGIN IMMEDIATE").await;
    let status = match result {
        Ok(_) => {
            connection.execute("ROLLBACK").await.unwrap();
            "acquired"
        }
        Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("5") => "busy",
        Err(error) => panic!("unexpected lock probe failure: {error}"),
    };
    connection.close().await.unwrap();
    println!("DIRECTORY_LOCK_PROBE:{nonce}:{status}");
}
