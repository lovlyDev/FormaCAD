//! Source-bound attachment reads; no UI descriptor or managed blob fallback.
use super::WindowSources;
use crate::core::{AppError, Result};

impl WindowSources {
    pub async fn read_asset(
        &self,
        label: &str,
        key: &str,
        id: &str,
        expected: &str,
        name: &str,
    ) -> Result<String> {
        #[cfg(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        ))]
        {
            crate::security::file_name(name)?;
            let read = self.capture_expected(label, key, id, expected).await?;
            let file = read
                .project()
                .files
                .iter()
                .find(|file| file.name == name)
                .ok_or_else(|| AppError::Invalid("Project attachment was not found".into()))?;
            let bytes = read.read_asset(file).await?;
            let response = crate::artifacts::data_url(&file.name, &bytes);
            // Encoding can be substantial; recheck the same original view after it.
            read.recheck().await?;
            Ok(response)
        }
        #[cfg(not(any(
            windows,
            all(
                target_os = "linux",
                any(target_arch = "x86_64", target_arch = "aarch64")
            )
        )))]
        {
            let _ = (label, key, id, expected, name);
            Err(AppError::Invalid("PROJECT_FOLDER_UNSUPPORTED".into()))
        }
    }
}

#[cfg(all(
    test,
    any(
        windows,
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )
))]
mod tests;
