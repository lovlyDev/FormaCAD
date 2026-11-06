//! Platform-stable host capture entry for common asynchronous read consumers.
use super::WindowSources;
use crate::{core::Result, project_repository::backend::RepositoryRead};

impl WindowSources {
    pub async fn capture_read(
        &self,
        label: &str,
        key: &str,
        id: &str,
        expected: &str,
    ) -> Result<RepositoryRead<'static>> {
        #[cfg(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        ))]
        {
            self.capture_operation(label, key, id, expected)
                .await?
                .into_read()
                .await
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
            Err(crate::core::AppError::Invalid(
                "PROJECT_FOLDER_UNSUPPORTED".into(),
            ))
        }
    }
}
