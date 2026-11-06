use super::{
    list_managed, project_entries, read_index, serialized_size, CatalogEntry, CatalogIssueStatus,
    CatalogLimits,
};
use crate::{
    project_access::AccessRegistry, project_repository::legacy::read_test_fixture as fixture,
};
use std::{
    collections::BTreeSet,
    future::Future,
    path::{Path, PathBuf},
    task::Poll,
};

fn limits() -> CatalogLimits {
    CatalogLimits {
        max_entries: 10,
        max_total_payload_bytes: 1024 * 1024,
    }
}

fn issue_for<'a>(entries: &'a [CatalogEntry], id: &str) -> &'a super::CatalogIssue {
    entries
        .iter()
        .find_map(|entry| match entry {
            CatalogEntry::Unavailable { index, issue, .. }
                if index.project_id.as_deref() == Some(id) =>
            {
                Some(issue)
            }
            _ => None,
        })
        .expect("unavailable row remains explicitly represented")
}

fn names(root: &Path) -> BTreeSet<PathBuf> {
    fn walk(root: &Path, current: &Path, result: &mut BTreeSet<PathBuf>) {
        for entry in std::fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            result.insert(path.strip_prefix(root).unwrap().into());
            if path.is_dir() {
                walk(root, &path, result);
            }
        }
    }
    let mut result = BTreeSet::new();
    walk(root, root, &mut result);
    result
}

#[tokio::test]
async fn mixed_ready_pending_and_corrupt_projects_remain_visible_without_recovery_or_source_writes()
{
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let ready = fixture::project(&state, 16).await;
    let pending = fixture::project(&state, 16).await;
    let corrupt = fixture::project(&state, 16).await;
    fixture::pending(&state, &pending.id).await;
    sqlx::query("UPDATE projects SET payload=? WHERE id=?")
        .bind("not JSON")
        .bind(&corrupt.id)
        .execute(&state.pool)
        .await
        .unwrap();
    let before = [
        fixture::logical(&state, &ready.id).await,
        fixture::logical(&state, &pending.id).await,
        fixture::logical(&state, &corrupt.id).await,
    ];
    let files = fixture::files(temp.path());
    let journal: String = sqlx::query_scalar(
        "SELECT target_manifest FROM project_storage_commits WHERE project_id=?",
    )
    .bind(&pending.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    let report = list_managed(&state, limits()).await.unwrap();
    assert_eq!(report.entries.len(), 3);
    assert!(report.entries.iter().any(
        |entry| matches!(entry, CatalogEntry::Ready { project, .. } if project.id == ready.id)
    ));
    assert_eq!(
        issue_for(&report.entries, &pending.id).status,
        CatalogIssueStatus::RecoveryRequired
    );
    assert_eq!(
        issue_for(&report.entries, &corrupt.id).status,
        CatalogIssueStatus::Unavailable
    );
    let after = [
        fixture::logical(&state, &ready.id).await,
        fixture::logical(&state, &pending.id).await,
        fixture::logical(&state, &corrupt.id).await,
    ];
    assert_eq!(after, before);
    assert_eq!(fixture::files(temp.path()), files);
    let after_journal: String = sqlx::query_scalar(
        "SELECT target_manifest FROM project_storage_commits WHERE project_id=?",
    )
    .bind(&pending.id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(after_journal, journal);
    let value = serde_json::to_value(&report).unwrap();
    let row = value["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == "ready")
        .unwrap();
    let keys: BTreeSet<_> = row
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, BTreeSet::from(["kind", "rowKey", "project"]));
}

#[tokio::test]
async fn catalog_never_upgrades_a_read_only_project_or_creates_another_lock() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    let owner = AccessRegistry::default();
    owner.acquire(&state.root, &project.id).unwrap();
    assert_eq!(
        state
            .project_access
            .acquire(&state.root, &project.id)
            .unwrap()
            .mode,
        "read_only"
    );
    let before = fixture::logical(&state, &project.id).await;
    // Inspect names, not exclusively locked byte ranges on Windows.
    let before_names = names(temp.path());
    let report = list_managed(&state, limits()).await.unwrap();
    assert!(
        matches!(&report.entries[0], CatalogEntry::Ready { project: saved, .. } if saved.id == project.id)
    );
    assert_eq!(
        state
            .project_access
            .status(&state.root, &project.id)
            .unwrap()
            .mode,
        "read_only"
    );
    assert_eq!(
        owner.status(&state.root, &project.id).unwrap().mode,
        "write"
    );
    assert!(state
        .project_access
        .ensure_write(&state.root, &project.id)
        .is_err());
    assert_eq!(names(temp.path()), before_names);
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    state.project_access.release(&project.id).unwrap();
    owner.release(&project.id).unwrap();
}

#[tokio::test]
async fn entry_limit_is_a_top_error_and_stable_oversize_payload_is_not_decoded() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let first = fixture::project(&state, 16).await;
    let _second = fixture::project(&state, 16).await;
    let error = list_managed(
        &state,
        CatalogLimits {
            max_entries: 1,
            ..limits()
        },
    )
    .await
    .err()
    .unwrap();
    assert_eq!(error.to_string(), "PROJECT_CATALOG_LIMIT");
    // SQLite constructs the oversized invalid payload; the index query retrieves
    // only its length. If captured/decoded, this would be unavailable JSON, not limit.
    sqlx::query("UPDATE projects SET payload=CAST(zeroblob(1048576) AS TEXT) WHERE id=?")
        .bind(&first.id)
        .execute(&state.pool)
        .await
        .unwrap();
    let report = list_managed(
        &state,
        CatalogLimits {
            max_entries: 2,
            max_total_payload_bytes: 32768,
        },
    )
    .await
    .unwrap();
    assert_eq!(report.entries.len(), 2);
    assert_eq!(
        issue_for(&report.entries, &first.id).status,
        CatalogIssueStatus::Limit
    );
    assert!(issue_for(&report.entries, &first.id)
        .details
        .contains("preflight"));
    assert!(report
        .entries
        .iter()
        .any(|entry| matches!(entry, CatalogEntry::Ready { .. })));
}

#[tokio::test]
async fn actual_escaped_serialized_payloads_share_the_callers_aggregate_budget() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let mut project = fixture::project(&state, 16).await;
    project.messages.push(serde_json::from_value(serde_json::json!({
        "id":uuid::Uuid::new_v4().to_string(),"role":"user","text":"quote \" slash \\ newline\n",
        "createdAt":"2026-10-31T00:00:00Z"
    })).unwrap());
    fixture::replace(&state, &project, false).await;
    let mut second = project.clone();
    second.id = uuid::Uuid::new_v4().to_string();
    fixture::replace(&state, &second, true).await;
    let size = serde_json::to_vec(&project).unwrap().len() as u64;
    assert_eq!(serialized_size(&project, size).unwrap(), size);
    assert!(serialized_size(&project, size - 1).is_err());
    let report = list_managed(
        &state,
        CatalogLimits {
            max_entries: 2,
            max_total_payload_bytes: size,
        },
    )
    .await
    .unwrap();
    assert_eq!(report.serialized_project_bytes, size);
    assert_eq!(
        report
            .entries
            .iter()
            .filter(|entry| matches!(entry, CatalogEntry::Ready { .. }))
            .count(),
        1
    );
    assert_eq!(report.entries.iter().filter(|entry| matches!(entry, CatalogEntry::Unavailable { issue, .. } if issue.status == CatalogIssueStatus::Limit)).count(), 1);
}

#[tokio::test]
async fn stale_index_length_does_not_admit_a_larger_captured_project() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let mut project = fixture::project(&state, 16).await;
    let indexed = read_index(&state, 1).await.unwrap();
    let old_size = serde_json::to_vec(&project).unwrap().len() as u64;
    project.messages.push(
        serde_json::from_value(serde_json::json!({
            "id":uuid::Uuid::new_v4().to_string(),"role":"user","text":"x".repeat(8192),
            "createdAt":"2026-10-31T00:00:00Z"
        }))
        .unwrap(),
    );
    fixture::replace(&state, &project, false).await;
    let before = fixture::logical(&state, &project.id).await;
    let report = project_entries(
        &state,
        indexed,
        CatalogLimits {
            max_entries: 1,
            max_total_payload_bytes: old_size + 1,
        },
    )
    .await
    .unwrap();
    assert_eq!(report.serialized_project_bytes, 0);
    assert_eq!(
        issue_for(&report.entries, &project.id).status,
        CatalogIssueStatus::Limit
    );
    assert!(!issue_for(&report.entries, &project.id)
        .details
        .contains("preflight"));
    assert_eq!(fixture::logical(&state, &project.id).await, before);
}

#[tokio::test]
async fn invalid_index_identifier_and_mismatched_payload_id_are_not_fabricated_projects() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let original = fixture::project(&state, 16).await;
    let raw = serde_json::to_string(&original).unwrap();
    let mismatched = uuid::Uuid::new_v4().to_string();
    for id in ["not-a-project-id", &mismatched] {
        sqlx::query("INSERT INTO projects(id,name,payload,updated_at) VALUES(?,?,?,?)")
            .bind(id)
            .bind("Index hint only")
            .bind(&raw)
            .bind("not a date")
            .execute(&state.pool)
            .await
            .unwrap();
    }
    let report = list_managed(&state, limits()).await.unwrap();
    assert_eq!(report.entries.len(), 3);
    let invalid = report.entries.iter().find(|entry| matches!(entry, CatalogEntry::Unavailable { index, .. } if index.id_hint.as_deref() == Some("not-a-project-id"))).unwrap();
    match invalid {
        CatalogEntry::Unavailable { index, issue, .. } => {
            assert!(index.project_id.is_none());
            assert!(index.updated_at.is_none());
            assert_eq!(issue.status, CatalogIssueStatus::Unavailable);
        }
        _ => unreachable!(),
    }
    assert_eq!(
        issue_for(&report.entries, &mismatched).status,
        CatalogIssueStatus::Changed
    );
    assert_eq!(
        report
            .entries
            .iter()
            .filter(|entry| matches!(entry, CatalogEntry::Ready { .. }))
            .count(),
        1
    );
}

#[tokio::test]
async fn closure_during_catalog_row_sql_capture_remains_a_changed_entry() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let project = fixture::project(&state, 16).await;
    // Complete the actual bounded index query before exhausting the pool. The
    // future below executes the same row projection used by public list_managed.
    let indexed = read_index(&state, 1).await.unwrap();
    let before = fixture::logical(&state, &project.id).await;
    let files = fixture::files(temp.path());
    let mut held = Vec::new();
    for _ in 0..state.pool.options().get_max_connections() {
        held.push(state.pool.acquire().await.unwrap());
    }
    let listing = project_entries(&state, indexed, limits());
    tokio::pin!(listing);
    std::future::poll_fn(|context| {
        assert!(listing.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    state.project_access.release(&project.id).unwrap();
    drop(held);
    let report = listing.await.unwrap();
    assert_eq!(report.entries.len(), 1);
    assert_eq!(
        issue_for(&report.entries, &project.id).status,
        CatalogIssueStatus::Changed
    );
    assert_eq!(fixture::logical(&state, &project.id).await, before);
    assert_eq!(fixture::files(temp.path()), files);
}

#[tokio::test]
async fn long_invalid_index_hints_and_rowids_above_javascript_precision_stay_distinct() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    let rowids = [9_007_199_254_740_993i64, 9_007_199_254_740_994i64];
    for (index, rowid) in rowids.into_iter().enumerate() {
        sqlx::query("INSERT INTO projects(rowid,id,name,payload,updated_at) VALUES(?,?,?,?,?)")
            .bind(rowid)
            .bind(format!("{}{index}", "x".repeat(10000)))
            .bind("n".repeat(10000))
            .bind("not JSON")
            .bind("t".repeat(10000))
            .execute(&state.pool)
            .await
            .unwrap();
    }
    let report = list_managed(
        &state,
        CatalogLimits {
            max_entries: 2,
            max_total_payload_bytes: 1024,
        },
    )
    .await
    .unwrap();
    assert_eq!(report.entries.len(), 2);
    assert_eq!(report.serialized_project_bytes, 0);
    for entry in &report.entries {
        match entry {
            CatalogEntry::Unavailable { index, issue, .. } => {
                assert_eq!(index.id_hint.as_deref().unwrap().len(), 37);
                assert!(index.project_id.is_none());
                assert!(index.name.is_none());
                assert!(index.updated_at.is_none());
                assert_eq!(issue.status, CatalogIssueStatus::Unavailable);
            }
            _ => panic!("an invalid index row must never become a Project"),
        }
    }
    let value = serde_json::to_value(report).unwrap();
    let keys: BTreeSet<_> = value["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["rowKey"].as_str().unwrap())
        .collect();
    let expected: BTreeSet<_> = rowids
        .iter()
        .map(|rowid| format!("managed-index:{rowid}"))
        .collect();
    assert_eq!(keys.len(), 2);
    assert_eq!(
        keys,
        expected.iter().map(String::as_str).collect::<BTreeSet<_>>()
    );
}

#[tokio::test]
async fn unavailable_database_is_not_a_successful_empty_catalog() {
    let temp = tempfile::tempdir().unwrap();
    let state = fixture::state(temp.path()).await;
    fixture::project(&state, 16).await;
    let indexed = read_index(&state, 1).await.unwrap();
    state.pool.close().await;
    assert!(matches!(
        list_managed(&state, limits()).await,
        Err(crate::core::AppError::Database(_))
    ));
    assert!(matches!(
        project_entries(&state, indexed, limits()).await,
        Err(crate::core::AppError::Database(_))
    ));
}
