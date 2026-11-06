//! Fresh-process read observations over parent-owned synthetic fixtures.
//! Process startup is measured separately from child API work. The OS cache is
//! deliberately not flushed; this is not a cold-cache or production backend test.
use crate::{
    core::{AppError, Result},
    models::{Project, ProjectFile},
    project_history,
    project_repository::folder::FolderRepository,
    project_storage_v2,
};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const HELPER: &str = "project_repository::benchmark::process::storage_read_probe";
const PREFIX: &str = "CHILD_STORAGE_PROBE ";
const LIMIT: usize = 64 * 1024;
const ENV_NONCE: &str = "FORMA_STORAGE_PROBE_NONCE";
const ENV_BACKEND: &str = "FORMA_STORAGE_PROBE_BACKEND";
const ENV_ROOT: &str = "FORMA_STORAGE_PROBE_ROOT";
const ENV_PROJECT: &str = "FORMA_STORAGE_PROBE_PROJECT";
const ENV_PARENT: &str = "FORMA_STORAGE_PROBE_PARENT";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    nonce: String,
    child_pid: u32,
    backend: String,
    project_id: String,
    project_sha256: String,
    revision_count: usize,
    open_elapsed_ns: u64,
    read_checks_elapsed_ns: u64,
    asset_check: bool,
    history_check: bool,
}

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::Invalid(format!("Storage process benchmark: {}", detail.into()))
}

fn supported(backend: &str) -> bool {
    matches!(
        backend,
        "manifest-v2" | "manifest-v2-metadata" | "sqlite-delete-prototype"
    )
}

struct OwnedChild {
    child: Child,
    reaped: bool,
}

impl OwnedChild {
    fn terminate(&mut self) -> std::io::Result<()> {
        if self.reaped {
            return Ok(());
        }
        // This is exclusively the handle returned by our Command::spawn. No
        // PID lookup, name matching or unrelated application process is used.
        if let Ok(Some(_)) = self.child.try_wait() {
            self.reaped = true;
            return Ok(());
        }
        if let Err(error) = self.child.kill() {
            // A child can exit between observation and kill. A terminal handle
            // proves cleanup; any other error remains visible to the caller.
            if let Ok(Some(_)) = self.child.try_wait() {
                self.reaped = true;
                return Ok(());
            }
            return Err(error);
        }
        self.child.wait()?;
        self.reaped = true;
        Ok(())
    }

    fn wait(&mut self) -> Result<ExitStatus> {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.reaped = true;
                    return Ok(status);
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => {
                    self.terminate()
                        .map_err(|error| invalid(format!("timeout cleanup failed: {error}")))?;
                    return Err(invalid(
                        "owned child exceeded 30 seconds and was killed/reaped",
                    ));
                }
                Err(error) => {
                    self.terminate().map_err(|cleanup| {
                        invalid(format!("wait failed: {error}; cleanup failed: {cleanup}"))
                    })?;
                    return Err(error.into());
                }
            }
        }
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.reaped {
            // Panic/error fallback. Normal wait/timeout paths report cleanup
            // errors explicitly; this guard still never intentionally detaches.
            if let Err(error) = self.terminate() {
                eprintln!("Storage benchmark owned child cleanup failed: {error}");
            }
        }
    }
}

fn read_output(stream: impl Read) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    stream.take(LIMIT as u64 + 1).read_to_end(&mut output)?;
    if output.len() > LIMIT {
        return Err(invalid("child output exceeded 64 KiB limit"));
    }
    Ok(output)
}

pub(super) fn measure(label: &str, backend: &str, root: &Path, expected: &Project) -> Result<()> {
    if !supported(backend) || !root.is_absolute() || !root.is_dir() {
        return Err(invalid("unsupported backend or invalid synthetic root"));
    }
    expected.validate()?;
    let nonce = uuid::Uuid::new_v4().to_string();
    let started = Instant::now();
    let child = Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            HELPER,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(ENV_NONCE, &nonce)
        .env(ENV_BACKEND, backend)
        .env(ENV_ROOT, root)
        .env(ENV_PROJECT, &expected.id)
        .env(ENV_PARENT, std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let child_pid = child.id();
    let mut owned = OwnedChild {
        child,
        reaped: false,
    };
    let status = owned.wait()?;
    let process_wall_ns = started.elapsed().as_nanos();
    let stdout = read_output(
        owned
            .child
            .stdout
            .take()
            .ok_or_else(|| invalid("missing child stdout"))?,
    )?;
    let stderr = read_output(
        owned
            .child
            .stderr
            .take()
            .ok_or_else(|| invalid("missing child stderr"))?,
    )?;
    if !status.success() {
        return Err(invalid(format!(
            "owned child failed ({status}); stderr: {}",
            String::from_utf8_lossy(&stderr)
        )));
    }
    let text = String::from_utf8(stdout).map_err(|_| invalid("child stdout is not UTF-8"))?;
    // libtest may print `test <name> ... ` before the helper's first output.
    let records: Vec<_> = text
        .lines()
        .filter_map(|line| line.split_once(PREFIX).map(|(_, record)| record))
        .collect();
    if records.len() != 1 {
        return Err(invalid("expected exactly one child probe record"));
    }
    let probe: Probe = serde_json::from_str(records[0])?;
    if probe.nonce != nonce
        || probe.child_pid != child_pid
        || child_pid == std::process::id()
        || probe.backend != backend
        || probe.project_id != expected.id
        || probe.project_sha256 != crate::artifacts::digest(&serde_json::to_vec(expected)?)
        || probe.revision_count != expected.revisions.len()
        || !probe.asset_check
        || !probe.history_check
    {
        return Err(invalid(
            "child identity, fixture or validation response mismatch",
        ));
    }
    println!(
        "STORAGE_BENCHMARK {}",
        serde_json::json!({
            "kind":"fresh_process", "size":label,"backend":backend,
            "child_pid":child_pid,"project_id":probe.project_id,"project_sha256":probe.project_sha256,"revision_count":probe.revision_count,
            "process_wall_ns":process_wall_ns,"child_open_elapsed_ns":probe.open_elapsed_ns,
            "child_read_checks_elapsed_ns":probe.read_checks_elapsed_ns,"asset_check":probe.asset_check,
            "history_check":probe.history_check,"samples":1,
            "note":"fresh executable process; wall time includes spawn/loader/runtime/test harness; child API time is separate; OS filesystem cache remains warm/uncontrolled"
        })
    );
    Ok(())
}

fn required(name: &str) -> Result<String> {
    std::env::var(name)
        .map_err(|_| invalid(format!("required parent-controlled env missing: {name}")))
}

fn elapsed(started: Instant) -> Result<u64> {
    started
        .elapsed()
        .as_nanos()
        .try_into()
        .map_err(|_| invalid("elapsed duration overflow"))
}

fn asset_check(root: &Path, project: &Project, file: &ProjectFile) -> Result<bool> {
    if !project.files.contains(file) || file.size > crate::artifacts::MAX_FILE_BYTES as u64 {
        return Err(invalid("invalid candidate asset metadata"));
    }
    let hash = file
        .sha256
        .as_ref()
        .filter(|hash| crate::artifacts::valid_digest(hash))
        .ok_or_else(|| invalid("candidate asset checksum missing"))?;
    let path = crate::security::guarded(root, Path::new(&format!("assets/{hash}")))?;
    let file_handle = std::fs::File::open(path)?;
    let metadata = file_handle.metadata()?;
    if !metadata.is_file() || metadata.len() != file.size {
        return Err(invalid("candidate asset length/type mismatch"));
    }
    let mut bytes = Vec::new();
    file_handle.take(file.size + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != file.size || crate::artifacts::digest(&bytes) != *hash {
        return Err(invalid("candidate asset bytes mismatch"));
    }
    Ok(true)
}

async fn run_probe() -> Result<Probe> {
    let nonce = required(ENV_NONCE)?;
    crate::security::valid_id(&nonce)?;
    let backend = required(ENV_BACKEND)?;
    let project_id = required(ENV_PROJECT)?;
    crate::security::valid_id(&project_id)?;
    let root =
        PathBuf::from(std::env::var_os(ENV_ROOT).ok_or_else(|| invalid("missing source root"))?);
    let parent: u32 = required(ENV_PARENT)?
        .parse()
        .map_err(|_| invalid("invalid parent identity"))?;
    if parent == std::process::id() || !supported(&backend) || !root.is_absolute() || !root.is_dir()
    {
        return Err(invalid("invalid parent/backend/source root"));
    }
    let started = Instant::now();
    let (project, open_elapsed_ns, read_checks_elapsed_ns, checked_asset, checked_history) =
        match backend.as_str() {
            "manifest-v2" => {
                let project = project_storage_v2::read_folder(&root)?;
                let open = elapsed(started)?;
                let checks = Instant::now();
                let history = project_storage_v2::read_history(&root)?
                    .ok_or_else(|| invalid("history absent"))?;
                project_history::validate_archive(&project, &history)?;
                let asset = project
                    .files
                    .first()
                    .ok_or_else(|| invalid("asset absent"))?;
                let checked_asset = asset_check(&root, &project, asset)?;
                (project, open, elapsed(checks)?, checked_asset, true)
            }
            "manifest-v2-metadata" => {
                let repository = FolderRepository::open(&root)?;
                let snapshot = repository.capture()?;
                let open = elapsed(started)?;
                let checks = Instant::now();
                let file = snapshot
                    .project()
                    .files
                    .first()
                    .ok_or_else(|| invalid("asset absent"))?;
                let bytes = repository.read_asset(&snapshot, file)?;
                let checked_asset = bytes.len() as u64 == file.size;
                repository
                    .read_history(&snapshot)?
                    .ok_or_else(|| invalid("history absent"))?;
                repository.recheck(&snapshot)?;
                (
                    snapshot.project().clone(),
                    open,
                    elapsed(checks)?,
                    checked_asset,
                    true,
                )
            }
            "sqlite-delete-prototype" => {
                let pool = super::candidate::open_reader(&root.join("project.sqlite")).await?;
                let project = super::fixture::read_project(&pool, &project_id).await;
                let open = elapsed(started)?;
                let checks = Instant::now();
                let cursor = project_history::load_state(&pool, &project).await?;
                std::hint::black_box((cursor.can_undo(), cursor.can_redo()));
                let history = {
                    let mut connection = pool.acquire().await?;
                    project_history::export_history(&mut connection, &project).await?
                };
                project_history::validate_archive(&project, &history)?;
                let file = project
                    .files
                    .first()
                    .ok_or_else(|| invalid("asset absent"))?;
                let checked_asset = asset_check(&root, &project, file)?;
                let checked = elapsed(checks)?;
                pool.close().await;
                (project, open, checked, checked_asset, true)
            }
            _ => return Err(invalid("unsupported backend")),
        };
    if project.id != project_id {
        return Err(invalid("source project differs from expected fixture"));
    }
    let project_sha256 = crate::artifacts::digest(&serde_json::to_vec(&project)?);
    Ok(Probe {
        nonce,
        child_pid: std::process::id(),
        backend,
        project_id,
        project_sha256,
        revision_count: project.revisions.len(),
        open_elapsed_ns,
        read_checks_elapsed_ns,
        asset_check: checked_asset,
        history_check: checked_history,
    })
}

#[test]
#[ignore = "internal fresh-process helper; requires parent-controlled synthetic fixture environment"]
fn storage_read_probe() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let probe = runtime.block_on(run_probe()).unwrap();
    println!("{PREFIX}{}", serde_json::to_string(&probe).unwrap());
}
