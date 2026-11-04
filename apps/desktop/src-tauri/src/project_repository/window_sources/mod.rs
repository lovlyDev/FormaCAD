//! Window-local FCP1 sources. No app project rows or persisted access keys.
use crate::{
    core::{AppError, Result},
    models::Project,
};
pub mod ipc;

#[cfg(any(
    windows,
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
))]
mod supported;
#[derive(Default)]
pub struct WindowSources {
    #[cfg(any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    ))]
    scopes: std::sync::Mutex<
        std::collections::HashMap<String, std::sync::Arc<super::sqlite_folder::api::FolderSources>>,
    >,
}
fn changed() -> AppError {
    AppError::Invalid("PROJECT_SNAPSHOT_CHANGED".into())
}

impl WindowSources {
    pub fn close_window(&self, label: &str) {
        #[cfg(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        ))]
        {
            let scope = self
                .scopes
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(label);
            if let Some(scope) = scope {
                let _ = scope.close_all();
            }
        }
        #[cfg(not(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        )))]
        let _ = label;
    }
    pub async fn read(&self, label: &str, key: &str, id: &str, expected: &str) -> Result<Project> {
        #[cfg(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        ))]
        {
            let source = self.existing(label)?.lookup(key)?;
            let view = source.capture_metadata().await?;
            if view.project().id != id || view.commit_id() != expected {
                return Err(changed());
            }
            let project = view.project().clone();
            source.recheck(&view).await?;
            Ok(project)
        }
        #[cfg(not(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        )))]
        {
            let _ = (label, key, id, expected);
            Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()))
        }
    }
    pub fn close_source(&self, label: &str, key: &str) -> Result<bool> {
        #[cfg(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        ))]
        {
            self.existing(label)?.close(key)
        }
        #[cfg(not(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        )))]
        {
            let _ = (label, key);
            Err(changed())
        }
    }
}

impl Drop for WindowSources {
    fn drop(&mut self) {
        #[cfg(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        ))]
        for scope in self
            .scopes
            .get_mut()
            .unwrap_or_else(|error| error.into_inner())
            .values()
        {
            let _ = scope.close_all();
        }
    }
}
