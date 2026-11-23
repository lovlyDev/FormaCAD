//! Resolve one known commit identity without replaying an uncertain operation.
use super::*;

/// Caller retains original physical source/opening authority. This SQL result
/// is an observation of the validated commit chain, never permission to replay.
/// In particular, Absent is not an automatic retry instruction.
pub(super) async fn resolve_commit(
    connection: &mut SqliteConnection,
    source: &Expectation,
    commit_id: &str,
) -> Result<CommitPresence> {
    crate::security::valid_id(commit_id)?;
    let mut transaction = connection.begin_with("BEGIN DEFERRED").await?;
    let result = async {
        let current = read::within(&mut transaction).await?;
        if current.expectation().repository_id() != source.repository_id()
            || current.expectation().project_id() != source.project_id()
        {
            return Err(changed());
        }
        let present: i64 =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM folder_commits WHERE commit_id=?)")
                .bind(commit_id)
                .fetch_one(&mut *transaction)
                .await?;
        Ok(if present == 1 {
            CommitPresence::Present {
                is_head: current.expectation().commit_id() == commit_id,
            }
        } else {
            CommitPresence::Absent
        })
    }
    .await;
    transaction.rollback().await?;
    result
}
