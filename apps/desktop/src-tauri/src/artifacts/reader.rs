//! Bound persisted attachment reads before allocation; caller retains SHA validation.
use crate::core::{AppError, Result};
use std::{fs::File, io::Read, path::Path};

fn invalid() -> AppError {
    AppError::Invalid("Project file failed its integrity check".into())
}

pub(super) fn read(path: &Path, expected_size: u64) -> Result<Vec<u8>> {
    if expected_size > super::MAX_FILE_BYTES as u64 {
        return Err(invalid());
    }
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() != expected_size {
        return Err(invalid());
    }
    // A concurrently growing file cannot cause an unbounded read after metadata.
    let mut bytes = Vec::new();
    file.take(expected_size + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != expected_size {
        return Err(invalid());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_oversized_attachment_mirror_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("oversized.step");
        let file = File::create(&path).unwrap();
        file.set_len(super::super::MAX_FILE_BYTES as u64 + 1)
            .unwrap();
        drop(file);
        // Only one byte is declared: no attempt should load the oversized mirror.
        assert!(read(&path, 1)
            .unwrap_err()
            .to_string()
            .contains("integrity check"));
    }

    #[test]
    fn oversized_declaration_is_rejected_before_opening_missing_path() {
        let temp = tempfile::tempdir().unwrap();
        let error = read(
            &temp.path().join("missing.step"),
            super::super::MAX_FILE_BYTES as u64 + 1,
        )
        .unwrap_err();
        assert!(matches!(error, AppError::Invalid(_)));
    }

    #[test]
    fn valid_bytes_and_existing_empty_attachment_remain_readable() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("saved.step");
        std::fs::write(&path, b"saved bytes").unwrap();
        assert_eq!(read(&path, 11).unwrap(), b"saved bytes");
        let empty = temp.path().join("empty.step");
        File::create(&empty).unwrap();
        assert!(read(&empty, 0).unwrap().is_empty());
    }

    #[test]
    fn truncated_and_wrong_declared_lengths_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("truncated.step");
        std::fs::write(&path, b"four").unwrap();
        assert!(read(&path, 8).is_err());
        assert!(read(&path, 2).is_err());
    }

    #[tokio::test]
    async fn real_artifact_entry_keeps_hash_integrity_validation() {
        let temp = tempfile::tempdir().unwrap();
        let pool = crate::storage::open(&temp.path().join("artifact.sqlite"))
            .await
            .unwrap();
        let (_, guard) = tracing_appender::non_blocking(std::io::sink());
        let state = crate::core::AppState {
            root: temp.path().into(),
            pool,
            _log_guard: guard,
            cad_tasks: Default::default(),
            project_access: Default::default(),
            grants: Default::default(),
            tasks: Default::default(),
            writes: Default::default(),
        };
        let project_id = uuid::Uuid::new_v4().to_string();
        let hash = crate::artifacts::digest(b"right");
        let directory = temp.path().join(&project_id).join("attachments");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("{hash}.step"));
        std::fs::write(&path, b"wrong").unwrap();
        let entry = crate::models::ProjectFile {
            name: "source.step".into(),
            size: 5,
            kind: "source".into(),
            data: None,
            sha256: Some(hash),
        };
        assert!(crate::artifacts::read(&state, &project_id, &entry)
            .unwrap_err()
            .to_string()
            .contains("integrity check"));
    }
}
