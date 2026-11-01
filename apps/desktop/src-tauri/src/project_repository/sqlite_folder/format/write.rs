//! SQL transactions only; filesystem/CAS durability and writer authority are
//! supplied by the caller. No app DB or migration runner participates.
use super::*;
use sqlx::{Connection, Sqlite, Transaction};

async fn finish(
    transaction: Transaction<'_, Sqlite>,
    prepared: Result<Snapshot>,
    commit_id: String,
) -> Result<CommitOutcome> {
    match prepared {
        Ok(snapshot) => match transaction.commit().await {
            Ok(()) => Ok(CommitOutcome::Committed(Box::new(snapshot))),
            Err(error) => Ok(CommitOutcome::Indeterminate {
                commit_id,
                error: error.into(),
            }),
        },
        Err(error) => {
            // A rollback failure is a connection-level error. Caller must discard
            // this connection; COMMIT was never attempted and commands are not replayed.
            transaction.rollback().await?;
            Ok(CommitOutcome::RejectedBefore(error))
        }
    }
}

async fn writer_policy(connection: &mut SqliteConnection) -> Result<()> {
    let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
        .fetch_one(connection)
        .await?;
    if foreign_keys != 1 {
        return Err(corrupt());
    }
    Ok(())
}

pub(super) async fn initialize(
    connection: &mut SqliteConnection,
    repository_id: &str,
    project: &Project,
    imported_history: Option<&[u8]>,
) -> Result<CommitOutcome> {
    let validated = (|| {
        crate::security::valid_id(repository_id)?;
        normalized_project(project)?;
        if repository_id == project.id {
            return Err(corrupt());
        }
        if let Some(bytes) = imported_history {
            crate::project_history::validate_archive(project, bytes)?;
        }
        Ok(())
    })();
    if let Err(error) = validated {
        return Ok(CommitOutcome::RejectedBefore(error));
    }
    let commit_id = uuid::Uuid::new_v4().to_string();
    let mut transaction = match connection.begin_with("BEGIN IMMEDIATE").await {
        Ok(transaction) => transaction,
        Err(error) => return Ok(CommitOutcome::RejectedBefore(error.into())),
    };
    let result = async {
        writer_policy(&mut transaction).await?;
        // Verify emptiness under the write transaction, not only in a host
        // preflight. Unknown/future databases never receive CREATE statements.
        let application: i64 = sqlx::query_scalar("PRAGMA application_id").fetch_one(&mut *transaction).await?;
        let version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(&mut *transaction).await?;
        let objects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_schema").fetch_one(&mut *transaction).await?;
        if application != 0 || version != 0 || objects != 0 { return Err(unsupported()); }
        let ddl=schema_ddl();
        sqlx::Executor::execute(&mut *transaction, ddl.as_str()).await?;
        sqlx::query(&format!("PRAGMA application_id={APPLICATION_ID}")).execute(&mut *transaction).await?;
        sqlx::query(&format!("PRAGMA user_version={USER_VERSION}")).execute(&mut *transaction).await?;
        read::validate_schema(&mut transaction).await?;
        sqlx::query("INSERT INTO folder_format(singleton,marker,schema_sha256,schema_fingerprint) VALUES(1,?,?,?)")
            .bind(FORMAT_MARKER).bind(schema_hash()).bind(schema_fingerprint().await?)
            .execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?)")
            .bind(&project.id).bind(&project.name).bind(serde_json::to_string(project)?).bind(&project.updated_at)
            .execute(&mut *transaction).await?;
        if let Some(history) = imported_history {
            crate::project_history::import_history(&mut transaction,project,history).await?;
        } else {
            crate::project_history::record_commit(&mut transaction,None,project,None).await?;
        }
        record_head(&mut transaction,repository_id,project,&commit_id,None,1).await?;
        read::within(&mut transaction).await
    }.await;
    finish(transaction, result, commit_id).await
}

pub(super) async fn commit(
    connection: &mut SqliteConnection,
    expected: &Snapshot,
    project: &Project,
    transition: Option<&PreparedHistory>,
) -> Result<CommitOutcome> {
    if let Err(error) = normalized_project(project) {
        return Ok(CommitOutcome::RejectedBefore(error));
    }
    let commit_id = uuid::Uuid::new_v4().to_string();
    let mut transaction = match connection.begin_with("BEGIN IMMEDIATE").await {
        Ok(transaction) => transaction,
        Err(error) => return Ok(CommitOutcome::RejectedBefore(error.into())),
    };
    let result = async {
        writer_policy(&mut transaction).await?;
        let original = read::within(&mut transaction).await?;
        if !original.same_saved_state(expected) || project.id != expected.expectation.project_id {
            return Err(changed());
        }
        let expected = &expected.expectation;
        let sequence = expected
            .sequence
            .checked_add(1)
            .filter(|sequence| *sequence <= MAX_COMMITS)
            .ok_or_else(limited)?;
        let updated = sqlx::query("UPDATE projects SET name=?,payload=?,updated_at=? WHERE id=?")
            .bind(&project.name)
            .bind(serde_json::to_string(project)?)
            .bind(&project.updated_at)
            .bind(&project.id)
            .execute(&mut *transaction)
            .await?;
        if updated.rows_affected() != 1 {
            return Err(corrupt());
        }
        // The deliberately identical history-table contract reuses the same
        // pure prepare_commit validation through record_commit, not a second
        // storage-specific state machine or the app's migration runner.
        crate::project_history::record_commit(
            &mut transaction,
            Some(&original.project),
            project,
            transition,
        )
        .await?;
        record_head(
            &mut transaction,
            &expected.repository_id,
            project,
            &commit_id,
            Some(&expected.commit_id),
            sequence,
        )
        .await?;
        read::within(&mut transaction).await
    }
    .await;
    finish(transaction, result, commit_id).await
}

async fn record_head(
    connection: &mut SqliteConnection,
    repository_id: &str,
    project: &Project,
    commit_id: &str,
    parent: Option<&str>,
    sequence: u64,
) -> Result<()> {
    let payload = serde_json::to_string(project)?;
    let cursor: String =
        sqlx::query_scalar("SELECT state_json FROM project_history WHERE project_id=?")
            .bind(&project.id)
            .fetch_one(&mut *connection)
            .await?;
    if cursor.len() as i64 > MAX_HISTORY_BYTES {
        return Err(limited());
    }
    let history = crate::project_history::export_history(connection, project).await?;
    crate::project_history::validate_archive(project, &history)?;
    let mut record = CommitRecord {
        id: commit_id.to_owned(),
        parent: parent.map(str::to_owned),
        sequence,
        revision: project.current_revision.clone(),
        project_hash: crate::artifacts::digest(payload.as_bytes()),
        cursor_hash: crate::artifacts::digest(cursor.as_bytes()),
        history_hash: crate::artifacts::digest(&history),
        seal: String::new(),
    };
    record.seal = record_seal(repository_id, &project.id, &record)?;
    sqlx::query("INSERT INTO folder_commits(commit_id,parent_commit_id,sequence,revision_id,project_sha256,cursor_sha256,history_sha256,seal_sha256) VALUES(?,?,?,?,?,?,?,?)")
        .bind(&record.id).bind(&record.parent).bind(sequence as i64).bind(&record.revision)
        .bind(&record.project_hash).bind(&record.cursor_hash).bind(&record.history_hash).bind(&record.seal)
        .execute(&mut *connection).await?;
    sqlx::query("INSERT INTO repository_state(singleton,repository_id,project_id,head_commit_id,sequence) VALUES(1,?,?,?,?) ON CONFLICT(singleton) DO UPDATE SET head_commit_id=excluded.head_commit_id,sequence=excluded.sequence")
        .bind(repository_id).bind(&project.id).bind(commit_id).bind(sequence as i64).execute(connection).await?;
    Ok(())
}
