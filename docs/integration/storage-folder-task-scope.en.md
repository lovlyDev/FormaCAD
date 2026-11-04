# CAD tasks of the actual folder opening — exact patch

[Русский](storage-folder-task-scope.md) · [Host API](../en/storage-folder-host-api.md)

Status: **applied after lovlyDev approved all 8 files** on 4 November 2026.
Scope assertions and native regression passed CI at `5899c64`. Patch base:
`4fd64373fa0e029896f4a296456a567bae3d1775`, Issue #31 / PR #35.
Owner: wenlixxq; scope agreement and acceptance: lovlyDev.

## Contract

FCP1 Opening already creates an unpersisted UUID `task_id`; FolderApplyHost
already uses it as its task group. The patch exposes that same value read-only
through FolderSnapshot → ReadView → RepositoryRead. No new identity, path/hash or
persisted token is created. Managed/V2 retain project-ID grouping; FCP1 uses its
actual original opening.

Inspection registers this group in the existing CadTaskGate. Its global semaphore
still permits one heavy worker; individual task UUIDs, status/cancel API and worker
protocol are unchanged. Equal project IDs at two folders have distinct groups;
commits retain a group's identity, close/reopen creates a fresh one. It matches
the existing FolderApplyHost group.

`open_project_folder` additionally returns `taskScopeId`; FolderSourceRef carries
it for subsequent workspace status filtering. The server does not accept it as
authority or a source-selection argument: reads/inspection still validate the
actual window, sourceId and expectedCommitId. The value is not stored in Project,
SQLite or app index, and provides no writer grant.

## Eight exact paths

Rust paths are relative to `apps/desktop/src-tauri/src/`:

- `project_repository/sqlite_folder/session.rs`: accessor for the snapshot's existing task_id.
- `project_repository/sqlite_folder/api.rs`: ReadView accessor.
- `project_repository/backend/read.rs`: common task_scope_id, retaining legacy groups.
- `project_repository/window_sources/ipc.rs`: additional folder-open response field.
- `apps/desktop/src/features/project-sources/contract.ts`: client reference taskScopeId.
- `model_inspection/read.rs`: bound-reader group instead of project ID.
- `model_inspection/read/folder_tests.rs`: actual registered group during the real worker.
- `project_repository/window_sources/assets/tests.rs`: distinct groups for equal IDs, stable commit group and fresh reopen group.

## Evidence and limitations

Exact diff: [storage-folder-task-scope.patch](storage-folder-task-scope.patch).
At `5899c64fe30bd5f83beb31caceaf7884acd56948`, strict compilation of both variants
and [Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187435)
passed: 98 folder, 2 window/asset and 3 original-source refresh tests.
[Native CI](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187467) passed
`independent_step_properties_use_original_folder_and_refuse_late_close`, including
the original opening task group assertion: 535 library PASS, 18 ignored; 11 smoke
PASS, 41 host consumer PASS and three separately executed folder roundtrip PASS.

UI task filtering, other geometry consumers and workspace/model/history/index
remain unwired. Source closure revokes its results; individual operation cancel
still uses the existing task UUID. No CadTaskGate status schema, native/Cargo/C++,
protocol/permissions/SQL/version changes. Spec §5 requires agreement for the
shared model_inspection, frontend contract and IPC response.
