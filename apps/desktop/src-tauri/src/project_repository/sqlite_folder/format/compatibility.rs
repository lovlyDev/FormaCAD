//! Latest metadata from a verified continuation of the original model state.
//! No schema change: immutable commit cursor/archive seals prove that intervening
//! publications did not edit or move the history cursor.
use super::*;

pub(super) async fn read(
    connection: &mut SqliteConnection,
    original: &Snapshot,
) -> Result<Snapshot> {
    let mut transaction = connection.begin_with("BEGIN DEFERRED").await?;
    let result = async {
        let latest = super::read::within(&mut transaction).await?;
        let before = original.expectation();
        let after = latest.expectation();
        if before.repository_id() != after.repository_id()
            || before.project_id() != after.project_id()
            || after.sequence() < before.sequence()
            || original.project.current_revision != latest.project.current_revision
            || original.project.files != latest.project.files
            || original.project.revisions != latest.project.revisions
        { return Err(changed()); }
        // The captured commit itself must still be present byte-for-byte. A
        // rewritten same-ID head is not a metadata continuation.
        let original_present: i64 = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM folder_commits WHERE sequence=? AND commit_id=? AND project_sha256=? AND cursor_sha256=? AND history_sha256=?)")
            .bind(before.sequence() as i64).bind(before.commit_id())
            .bind(&original.saved_state.project).bind(&original.saved_state.cursor).bind(&original.saved_state.history)
            .fetch_one(&mut *transaction).await?;
        if original_present != 1 { return Err(changed()); }
        // read::within already validates the complete bounded, contiguous chain
        // and each seal. Any model edit/undo/redo changes cursor/archive seals,
        // even if geometry later returns to the same authored state.
        let incompatible: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM folder_commits WHERE sequence>? AND (revision_id IS NOT ? OR cursor_sha256<>? OR history_sha256<>?)")
            .bind(before.sequence() as i64).bind(before.revision())
            .bind(&original.saved_state.cursor).bind(&original.saved_state.history)
            .fetch_one(&mut *transaction).await?;
        if incompatible != 0 { return Err(changed()); }
        Ok(latest)
    }.await;
    transaction.rollback().await?;
    result
}

#[cfg(test)]
mod tests;
