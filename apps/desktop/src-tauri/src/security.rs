use crate::core::{AppError, Result};
use std::path::{Component, Path, PathBuf};
pub fn valid_id(id: &str) -> Result<()> {
    if uuid::Uuid::parse_str(id).is_err() || id.len() != 36 {
        return Err(AppError::Invalid(
            "Invalid project or revision identifier".into(),
        ));
    }
    Ok(())
}
pub fn file_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 255
        || name.contains(['/', '\\', ':', '\0'])
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
    {
        return Err(AppError::Invalid("Unsafe filename".into()));
    }
    Ok(())
}
/// Reject links at every existing path component, including a missing leaf's parent.
/// All paths passed here are app-owned paths, never commands supplied by the agent.
pub fn guarded(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute()
        || relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(AppError::Invalid("Path traversal is not allowed".into()));
    }
    let root = root.canonicalize()?;
    let mut current = root.clone();
    for part in relative.components() {
        current.push(part);
        if current.exists() {
            let meta = std::fs::symlink_metadata(&current)?;
            if meta.file_type().is_symlink() {
                return Err(AppError::Invalid(
                    "Symbolic links are not allowed in managed project paths".into(),
                ));
            }
            if !current.canonicalize()?.starts_with(&root) {
                return Err(AppError::Invalid("Path is outside the workspace".into()));
            }
        }
    }
    Ok(current)
}
pub fn executable(path: &Path) -> Result<()> {
    if !path.is_absolute() || !path.is_file() {
        return Err(AppError::Invalid(
            "Agent must resolve to an installed executable".into(),
        ));
    }
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if [
        "cmd",
        "powershell",
        "pwsh",
        "sh",
        "bash",
        "zsh",
        "sudo",
        "doas",
        "format",
        "shutdown",
        "reg",
        "sc",
    ]
    .contains(&name.as_str())
        || path
            .extension()
            .is_some_and(|s| s == "cmd" || s == "bat" || s == "ps1")
    {
        return Err(AppError::Invalid(
            "Shell wrappers and system commands are not permitted".into(),
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traversal_rejected() {
        let dir = tempfile::tempdir().unwrap();
        for path in ["../escape", "a/../../escape", "/tmp/escape"] {
            assert!(guarded(dir.path(), Path::new(path)).is_err());
        }
    }
    #[test]
    fn missing_leaf_is_guarded() {
        let dir = tempfile::tempdir().unwrap();
        assert!(guarded(dir.path(), Path::new("workspace/new.step"))
            .unwrap()
            .starts_with(dir.path().canonicalize().unwrap()));
    }
    #[test]
    fn filename_injection_rejected() {
        for name in ["../x", "a\\b", "a:stream", "x\0y", "..", "."] {
            assert!(file_name(name).is_err());
        }
    }
    #[cfg(unix)]
    #[test]
    fn symlink_escape_rejected() {
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(std::env::temp_dir(), dir.path().join("link")).unwrap();
        assert!(guarded(dir.path(), Path::new("link/new.step")).is_err());
    }
}
