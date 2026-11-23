use super::FolderOperation;
use crate::{core::Result, project_repository::sqlite_folder::api::LocalDraftStore};

impl FolderOperation {
    pub(crate) async fn keep_local_editor_draft(
        &self,
        store: &LocalDraftStore,
        text: String,
    ) -> Result<()> {
        let approval = self.source.approve_local_draft(&self.view).await?;
        store.save(approval, text)?;
        Ok(())
    }

    pub(crate) async fn review_local_editor_draft(
        &self,
        store: &LocalDraftStore,
    ) -> Result<Option<String>> {
        let draft = self.source.latest_local_draft(&self.view, store).await?;
        Ok(draft.map(|draft| draft.editor_text().to_owned()))
    }
}
