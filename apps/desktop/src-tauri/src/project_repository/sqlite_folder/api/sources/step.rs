use super::{
    super::{ModelRuntime, SelectedStepFile, StepProjectIntent, StepRead},
    FolderCreation, FolderSources,
};
use crate::core::Result;
use std::path::Path;

impl FolderSources {
    /// Trusted host entry point after explicit native source and destination
    /// selection. Open only the exact publication in this window's registry.
    pub async fn transfer_selected_step(
        &self,
        source: &SelectedStepFile,
        original: &StepRead,
        destination: &Path,
        intent: StepProjectIntent,
        runtime: ModelRuntime<'_>,
    ) -> Result<FolderCreation> {
        let opening = self.reserve()?;
        let approval = source.approve_destination(original, destination)?;
        let publication = source
            .transfer_to_folder(original, approval, intent, runtime)
            .await?;
        opening.open_publication(destination, publication).await
    }
}
