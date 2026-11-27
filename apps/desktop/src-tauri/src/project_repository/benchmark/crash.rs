//! Controlled process-death observations at representation publication boundaries.
//! Only disposable parent-owned fixtures are used. The SQL repair function below
//! explicitly opts into SQLite recovery; it is not a production recovery API.
use crate::{
    core::{AppError, Result},
    models::Project,
    project_history, project_storage_v2 as folder,
};
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const HELPER: &str = "project_repository::benchmark::crash::storage_crash_probe";
const ENV_PROTOCOL: &str = "FORMA_STORAGE_CRASH_PROTOCOL";
const APPLICATION_ID: u32 = 0x4643_4231;
const USER_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Protocol {
    nonce: String,
    parent_pid: u32,
    root: PathBuf,
    ready: PathBuf,
    project_id: String,
    backend: String,
    phase: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ready {
    nonce: String,
    pid: u32,
    project_id: String,
    backend: String,
    phase: String,
}

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::Invalid(format!("Storage crash benchmark: {}", detail.into()))
}

fn changed(project: &Project) -> Project {
    let mut next = project.clone();
    next.name = "Crash boundary metadata update".into();
    next.updated_at = "2026-10-31T00:00:00Z".into();
    next
}

fn hash(project: &Project) -> Result<String> {
    Ok(crate::artifacts::digest(&serde_json::to_vec(project)?))
}

fn files(root: &Path) -> Result<BTreeMap<PathBuf, (u64, String)>> {
    fn walk(root: &Path, at: &Path, result: &mut BTreeMap<PathBuf, (u64, String)>) -> Result<()> {
        for entry in fs::read_dir(at)? {
            let path = entry?.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.is_dir() {
                walk(root, &path, result)?;
            } else if metadata.is_file() {
                // These are our modest bounded fixtures, never user projects.
                if metadata.len() > 32 * 1024 * 1024 {
                    return Err(invalid("fixture unexpectedly exceeded its budget"));
                }
                result.insert(
                    path.strip_prefix(root)
                        .map_err(|_| invalid("fixture outside root"))?
                        .into(),
                    (metadata.len(), crate::artifacts::digest(&fs::read(path)?)),
                );
            } else {
                return Err(invalid("non-regular fixture entry"));
            }
        }
        Ok(())
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result)?;
    Ok(result)
}

struct OwnedChild {
    child: Child,
    reaped: bool,
}

impl OwnedChild {
    /// True only when this owner successfully requested kill and then reaped
    /// the child. Cleanup remains idempotent, but an already exited process is
    /// not evidence of a parent-induced crash at the selected boundary.
    fn kill_reap(&mut self) -> Result<bool> {
        if self.reaped {
            return Ok(false);
        }
        if self.child.try_wait()?.is_some() {
            self.reaped = true;
            return Ok(false);
        }
        if let Err(error) = self.child.kill() {
            if let Ok(Some(_)) = self.child.try_wait() {
                self.reaped = true;
                return Ok(false);
            }
            return Err(error.into());
        }
        self.child.wait()?;
        self.reaped = true;
        Ok(true)
    }

    fn wait_ready(&mut self, protocol: &Protocol) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            // Child writes and syncs readiness before parking. A partially
            // written file is retried while the exact owned child stays alive.
            if protocol.ready.is_file() {
                let metadata = fs::metadata(&protocol.ready)?;
                if metadata.len() > 4096 {
                    return Err(invalid("ready record exceeded limit"));
                }
                if let Ok(ready) = serde_json::from_slice::<Ready>(&fs::read(&protocol.ready)?) {
                    if ready.nonce != protocol.nonce
                        || ready.pid != self.child.id()
                        || ready.pid == protocol.parent_pid
                        || ready.project_id != protocol.project_id
                        || ready.backend != protocol.backend
                        || ready.phase != protocol.phase
                    {
                        return Err(invalid("owned child's ready identity mismatch"));
                    }
                    if self.child.try_wait()?.is_some() {
                        self.reaped = true;
                        return Err(invalid("owned child exited instead of parking at boundary"));
                    }
                    return Ok(());
                }
            }
            if let Some(status) = self.child.try_wait()? {
                self.reaped = true;
                return Err(invalid(format!(
                    "owned child exited before readiness: {status}"
                )));
            }
            if Instant::now() >= deadline {
                self.kill_reap()?;
                return Err(invalid("owned child timed out and was killed/reaped"));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.reaped {
            if let Err(error) = self.kill_reap() {
                eprintln!("Storage crash benchmark child cleanup failed: {error}");
            }
        }
    }
}

fn kill_at_boundary(protocol: &Protocol) -> Result<u32> {
    // Files avoid stdout/stderr pipe backpressure while the child deliberately
    // parks. Both logs are created exclusively inside this parent's tempdir.
    let stdout = File::options()
        .write(true)
        .create_new(true)
        .open(protocol.ready.with_extension("stdout"))?;
    let stderr_path = protocol.ready.with_extension("stderr");
    let stderr = File::options()
        .write(true)
        .create_new(true)
        .open(&stderr_path)?;
    let child = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            HELPER,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(ENV_PROTOCOL, serde_json::to_string(protocol)?)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()?;
    let pid = child.id();
    let mut owned = OwnedChild {
        child,
        reaped: false,
    };
    if let Err(error) = owned.wait_ready(protocol) {
        owned.kill_reap()?;
        let mut detail = Vec::new();
        File::open(&stderr_path)?
            .take(4096)
            .read_to_end(&mut detail)?;
        return Err(invalid(format!(
            "{error}; child stderr: {}",
            String::from_utf8_lossy(&detail)
        )));
    }
    if !owned.kill_reap()? {
        return Err(invalid(
            "owned child exited before the parent requested kill",
        ));
    }
    Ok(pid)
}

/// Bound the payload in the same SELECT that fetches it, so an unexpected
/// fixture failure is a Result and callers can still close their pools.
async fn read_project(pool: &SqlitePool, expected_id: &str) -> Result<Project> {
    const MAX_METADATA: i64 = 32 * 1024 * 1024;
    crate::security::valid_id(expected_id)?;
    let (size, payload): (i64, Option<String>) = sqlx::query_as(
        "SELECT length(CAST(payload AS BLOB)), CASE WHEN length(CAST(payload AS BLOB))<=? THEN payload END FROM projects WHERE id=?",
    )
    .bind(MAX_METADATA)
    .bind(expected_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| invalid("fixture project is missing"))?;
    if !(0..=MAX_METADATA).contains(&size) {
        return Err(invalid("fixture project metadata exceeds its limit"));
    }
    let payload = payload.ok_or_else(|| invalid("fixture project payload is missing"))?;
    let project: Project = serde_json::from_str(&payload)?;
    project.validate()?;
    if project.id != expected_id {
        return Err(invalid("fixture project identity mismatch"));
    }
    Ok(project)
}

/// Explicitly selected repair of a known, parent-owned benchmark SQLite DB.
/// The normal reader and candidate::open_writer both refuse pending journals.
async fn explicit_sqlite_recovery(path: &Path, expected_id: &str) -> Result<(Project, Vec<u8>)> {
    let mut header = [0u8; 100];
    File::open(path)?.read_exact(&mut header)?;
    if &header[..16] != b"SQLite format 3\0"
        || header[18] != 1
        || header[19] != 1
        || u32::from_be_bytes(header[60..64].try_into().unwrap()) != USER_VERSION
        || u32::from_be_bytes(header[68..72].try_into().unwrap()) != APPLICATION_ID
    {
        return Err(invalid(
            "refusing explicit recovery of unknown fixture format",
        ));
    }
    // A write-capable SQLite open is the explicit recovery action here. There
    // are no application migrations, settings, grants or recent-index writes.
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .read_only(false)
        .foreign_keys(true)
        .synchronous(SqliteSynchronous::Full);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let result: Result<(Project, Vec<u8>)> = async {
        let application_id: i64 = sqlx::query_scalar("PRAGMA application_id")
            .fetch_one(&pool)
            .await?;
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&pool)
            .await?;
        if application_id != i64::from(APPLICATION_ID) || version != i64::from(USER_VERSION) {
            return Err(invalid("recovered fixture format changed"));
        }
        let integrity: String = sqlx::query_scalar("PRAGMA quick_check")
            .fetch_one(&pool)
            .await?;
        if integrity != "ok" {
            return Err(invalid("SQLite recovery integrity failure"));
        }
        let project = read_project(&pool, expected_id).await?;
        let history = {
            let mut connection = pool.acquire().await?;
            project_history::export_history(&mut connection, &project).await?
        };
        project_history::validate_archive(&project, &history)?;
        Ok((project, history))
    }
    .await;
    pool.close().await;
    result
}

pub(super) async fn measure() -> Result<()> {
    for backend in ["manifest-v2", "sqlite-delete-prototype"] {
        for phase in ["beforecommit", "aftercommit"] {
            let fixture = tempfile::tempdir()?;
            let source = fixture.path().join("source");
            fs::create_dir(&source)?;
            let setup = if backend == "manifest-v2" {
                let root = fixture.path().join("setup");
                fs::create_dir(&root)?;
                root
            } else {
                source.clone()
            };
            let (pool, project, assets) = super::fixture::build(&setup, 32, 4, 4096).await;
            let archive = {
                let mut connection = pool.acquire().await?;
                project_history::export_history(&mut connection, &project).await?
            };
            if backend == "manifest-v2" {
                let mut staged = folder::stage_with(&source, &project, |file| {
                    Ok(assets[file.sha256.as_ref().unwrap()].clone())
                })?;
                folder::stage_history(&source, &mut staged, &archive)?;
                folder::publish(&source, staged)?;
            }
            pool.close().await;
            let before = files(&source)?;
            let nonce = uuid::Uuid::new_v4().to_string();
            let protocol = Protocol {
                nonce: nonce.clone(),
                parent_pid: std::process::id(),
                root: source.clone(),
                ready: fixture.path().join(format!("ready-{nonce}.json")),
                project_id: project.id.clone(),
                backend: backend.into(),
                phase: phase.into(),
            };
            let started = Instant::now();
            let child_pid = kill_at_boundary(&protocol)?;
            let process_wall_ns = started.elapsed().as_nanos();
            let after_kill = files(&source)?;
            let database_pages_changed = before.get(Path::new("project.sqlite"))
                != after_kill.get(Path::new("project.sqlite"));
            let expected = if phase == "beforecommit" {
                project.clone()
            } else {
                changed(&project)
            };
            let read_started = Instant::now();
            let mut recovery_elapsed_ns = None;
            let (actual, reader_refused, explicit_recovery, journal_hot_header) = if backend
                == "manifest-v2"
            {
                let actual = folder::read_folder(&source)?;
                let history = folder::read_history(&source)?
                    .ok_or_else(|| invalid("manifest history absent"))?;
                project_history::validate_archive(&actual, &history)?;
                if history != archive || files(&source)? != after_kill {
                    return Err(invalid("manifest reader changed source/history"));
                }
                (actual, false, false, false)
            } else if phase == "beforecommit" {
                let journal = source.join("project.sqlite-journal");
                let mut header = [0u8; 8];
                File::open(&journal)?.read_exact(&mut header)?;
                let hot = header == [0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7];
                if !hot {
                    return Err(invalid(
                        "cache-spill fixture did not leave a synced hot-journal header",
                    ));
                }
                if !database_pages_changed {
                    return Err(invalid(
                        "cache-spill fixture did not write uncommitted database pages",
                    ));
                }
                if let Ok(reader) =
                    super::candidate::open_reader(&source.join("project.sqlite")).await
                {
                    reader.close().await;
                    return Err(invalid("reader accepted pending rollback journal"));
                }
                if files(&source)? != after_kill {
                    return Err(invalid("reader silently repaired pending journal"));
                }
                let recovery_started = Instant::now();
                let (recovered, recovered_history) =
                    explicit_sqlite_recovery(&source.join("project.sqlite"), &project.id).await?;
                recovery_elapsed_ns = Some(recovery_started.elapsed().as_nanos());
                if recovered_history != archive {
                    return Err(invalid("recovered history differs from precommit archive"));
                }
                let reader = super::candidate::open_reader(&source.join("project.sqlite")).await?;
                reader.close().await;
                (recovered, true, true, hot)
            } else {
                let reader = super::candidate::open_reader(&source.join("project.sqlite")).await?;
                let observed: Result<(Project, Vec<u8>)> = async {
                    let actual = read_project(&reader, &project.id).await?;
                    let history = {
                        let mut connection = reader.acquire().await?;
                        project_history::export_history(&mut connection, &actual).await?
                    };
                    project_history::validate_archive(&actual, &history)?;
                    Ok((actual, history))
                }
                .await;
                reader.close().await;
                let (actual, history) = observed?;
                if history != archive {
                    return Err(invalid("postcommit metadata update changed history"));
                }
                if files(&source)? != after_kill {
                    return Err(invalid("postcommit reader changed source"));
                }
                (actual, false, false, false)
            };
            let read_recovery_elapsed_ns = read_started.elapsed().as_nanos();
            if hash(&actual)? != hash(&expected)? {
                return Err(invalid("wrong metadata survived process death"));
            }
            let final_files = files(&source)?;
            let original_assets: Vec<_> = before
                .iter()
                .filter(|(path, _)| path.starts_with("assets"))
                .collect();
            let final_assets: Vec<_> = final_files
                .iter()
                .filter(|(path, _)| path.starts_with("assets"))
                .collect();
            if original_assets != final_assets {
                return Err(invalid("crash/recovery changed immutable assets"));
            }
            println!(
                "STORAGE_BENCHMARK {}",
                serde_json::json!({"kind":"crash_boundary","backend":backend,
                "phase":phase,"child_pid":child_pid,"child_killed_and_reaped":true,"process_wall_ns":process_wall_ns,
                "expected_project_sha256":hash(&expected)?,"actual_project_sha256":hash(&actual)?,
                "reader_refused_pending_journal":reader_refused,"explicit_sqlite_recovery":explicit_recovery,
                "synced_hot_journal_header":journal_hot_header,"reader_source_bytes_unchanged":true,
                "explicit_recovery_elapsed_ns":recovery_elapsed_ns,"read_recovery_elapsed_ns":read_recovery_elapsed_ns,
                "database_bytes_changed_before_kill":database_pages_changed,
                "asset_bytes_preserved":true,"history_archive_sha256":crate::artifacts::digest(&archive),
                "source_file_count_before":before.len(),"source_file_count_after_kill":after_kill.len(),
                "note":"owned synthetic process killed at explicit stage/commit boundary; metadata/history primitives only; no real CAD model commit, production lease/recovery API, power-loss or NAS/USB claim"})
            );
            fixture.close()?;
        }
    }
    Ok(())
}

fn signal_and_park(protocol: &Protocol) -> Result<()> {
    let ready = Ready {
        nonce: protocol.nonce.clone(),
        pid: std::process::id(),
        project_id: protocol.project_id.clone(),
        backend: protocol.backend.clone(),
        phase: protocol.phase.clone(),
    };
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(&protocol.ready)?;
    file.write_all(&serde_json::to_vec(&ready)?)?;
    file.sync_all()?;
    drop(file);
    loop {
        std::thread::sleep(Duration::from_millis(10));
    }
}

async fn run_child(protocol: &Protocol) -> Result<()> {
    crate::security::valid_id(&protocol.nonce)?;
    crate::security::valid_id(&protocol.project_id)?;
    if protocol.parent_pid == std::process::id()
        || !protocol.root.is_absolute()
        || !protocol.root.is_dir()
        || protocol.ready.parent() != protocol.root.parent()
        || protocol.ready.file_name()
            != Some(std::ffi::OsStr::new(&format!(
                "ready-{}.json",
                protocol.nonce
            )))
        || !matches!(protocol.phase.as_str(), "beforecommit" | "aftercommit")
    {
        return Err(invalid("invalid controlled crash protocol"));
    }
    match protocol.backend.as_str() {
        "manifest-v2" => {
            let project = folder::read_folder(&protocol.root)?;
            if project.id != protocol.project_id {
                return Err(invalid("manifest fixture identity mismatch"));
            }
            let history =
                folder::read_history(&protocol.root)?.ok_or_else(|| invalid("history absent"))?;
            let next = changed(&project);
            let mut staged = folder::stage_with(&protocol.root, &next, |file| {
                super::copy::read_asset(&protocol.root, file)
            })?;
            folder::stage_history(&protocol.root, &mut staged, &history)?;
            if protocol.phase == "aftercommit" {
                folder::publish(&protocol.root, staged)?;
            }
            signal_and_park(protocol)
        }
        "sqlite-delete-prototype" => {
            let writer =
                super::candidate::open_writer(&protocol.root.join("project.sqlite")).await?;
            // A tiny cache forces dirty overflow pages out before commit. FULL
            // sync makes the rollback header durable before those DB writes.
            sqlx::query("PRAGMA cache_size = 1")
                .execute(&writer)
                .await?;
            sqlx::query("PRAGMA cache_spill = ON")
                .execute(&writer)
                .await?;
            let project = read_project(&writer, &protocol.project_id).await?;
            let next = changed(&project);
            let mut transaction = writer.begin().await?;
            sqlx::query("UPDATE projects SET name=?,payload=?,updated_at=? WHERE id=?")
                .bind(&next.name)
                .bind(serde_json::to_string(&next)?)
                .bind(&next.updated_at)
                .bind(&next.id)
                .execute(&mut *transaction)
                .await?;
            project_history::record_commit(&mut transaction, Some(&project), &next, None).await?;
            if protocol.phase == "aftercommit" {
                transaction.commit().await?;
            }
            signal_and_park(protocol)
        }
        _ => Err(invalid("unknown crash backend")),
    }
}

#[test]
#[ignore = "internal child helper; requires parent-owned synthetic crash protocol"]
fn storage_crash_probe() {
    let protocol = std::env::var(ENV_PROTOCOL).expect("parent-controlled protocol is required");
    assert!(protocol.len() <= 16384, "crash protocol exceeds limit");
    let protocol: Protocol = serde_json::from_str(&protocol).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(run_child(&protocol)).unwrap();
}
