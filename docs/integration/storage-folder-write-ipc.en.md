# Writing to the original folder through existing commands

[Русский](storage-folder-write-ipc.md) · [Host API](../en/storage-folder-host-api.md)

Status: **proposed for agreement in Issue #31; source files are unchanged**.
Implementation by wenlixxq, acceptance by lovlyDev. Base
`ad710626e0ee21b87ce6bf33170516f0b7d89faf`. Exact
[patch](storage-folder-write-ipc.patch), SHA256
`6926a08d0628f923483a28f986d8d71679aa1207a7325723c594ac35ea4328f3`.

## Flow

The existing `apply_program`, `apply_ir_commands`, `undo_model` and
`redo_model` commands accept an optional `folderWrite` object with
`sourceId`, `expectedCommitId`, and `approvalId`. The call also sends
`projectId`. A call mixing the folder request with a program, CAD commands,
prompt, expected revision or ReviewPlan is rejected before spending the
approval. The host checks the window, source, base commit and operation kind
under the original request lock and executes the retained intent.

Managed calls keep their existing Project JSON and persistence route. Folder
replies bind `sourceId`, `projectId`, and `baseCommitId` to one of
`noOp`, `committed`, `rejectedBefore`, and `indeterminate`. An uncertain
result retains the attempted commit ID. The frontend helper checks the
reply against the original ticket and makes a single call. The caller must
check the active opening before adopting the result. Visible controls,
lookup of an uncertain commit ID, and reading the folder Undo/Redo cursor
remain separate work.

Execution uses the existing CAD worker and transactional FCP1 host. Approval
state remains in memory. This route accepts neither workspace nor executable
paths and creates no managed project row for the folder. It uses the existing
error codes and translations.

## Files

| Scope | Paths |
| --- | --- |
| Shared IPC | `apps/desktop/src-tauri/src/modeling.rs`, `apps/desktop/src-tauri/src/project_history/mod.rs` |
| Storage | `apps/desktop/src-tauri/src/project_repository/window_sources/{mod.rs,write_ipc.rs,write_ipc/tests.rs}` |
| Transport | `apps/desktop/src/features/project-sources/{writeApi.ts,writeApi.test.ts}` |
| Verification | `.github/workflows/storage-benchmark.yml` |

The patch applies to the stated base. The isolated copy passed rustfmt,
TypeScript, ESLint on the new modules, and 25 frontend tests (15 transport,
10 permission). Two new Rust tests cover request parsing and the unchanged
managed reply. They and the Tauri handler changes require compilation and
storage CI after application.
