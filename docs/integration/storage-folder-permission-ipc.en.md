# Original-folder permission through existing IPC — exact patch

[Русский](storage-folder-permission-ipc.md) · [Host API](../en/storage-folder-host-api.md) · [Sections](storage-section-renderer-source.en.md)

Status: **prepared, unapplied in the task branch; owner agreement not requested yet**.
Issue #31 / PR #35; implementation wenlixxq, agreement and acceptance lovlyDev.
Base `3e945bca9cfb956b23ecdaafd4eb52e95bd0af37`. The exact
[patch](storage-folder-permission-ipc.patch) changes the 16 paths below. SHA256:
`02a11c3298fff552d2129188957dbacfd017a6f467e92194dcbd2ed5c709a8f0`.

## Contract and storage

Existing `request_permission` and `resolve_permission` are extended; no parallel IPC
is introduced. Missing folder fields retain the managed payload/String reply.
Managed request/resolve move into one module; consume and its rules are unchanged.

A folder request sends project ID, sourceId/expectedCommitId and a typed Model with
optional ReviewPlan, CAD Commands or History intent. Incomplete references fail
without managed fallback. The invoking WebviewWindow supplies the actual window.
The registry captures one original FolderOperation; pure preparation receives its
Project and reuses existing validators/command preparation. The complete ApplyRequest,
including ReviewPlan and prepared revision, stays in RAM through decision/execution.
The same executor builds geometry; no second algorithm is introduced.

The folder reply is `{id, requiresConfirmation}`. Only existing server confirmation
policy determines this flag. The host handles none/override modes; renderer flags
or settings cannot manufacture permission. The frontend overload captures original
source/context and deeply copies intent before awaiting. Explicit null folder
contexts and managed String replies cannot become folder permission receipts.

For manual decisions the host checks original identity before audit SQL. Permission
becomes available after a successful decision record and source recheck. Denial
revokes the pending request even if audit SQL fails. Audit rows/settings cannot
resurrect authority after restart/close/reopen or determine the source; source/path/
access tokens are not stored in audit. Request ID remains a diagnostic identifier;
actual permission and command exist only in the live RAM registry. Registry locks
never span SQL awaits. One-use consumption, expiry and the actual OS writer lease
remain backend requirements.

This patch does not yet connect writer controls or existing apply_program/
apply_ir_commands/undo/redo to folder receipts. The next agreed scope will execute
the retained command through those existing IPC and adopt a publication with its
new commit. Folder controls remain hidden until the full flow works. A permission
API alone is not completed folder editor/AI/command behavior.

## All 16 paths

1. `apps/desktop/src-tauri/src/permissions.rs`
2. new `apps/desktop/src-tauri/src/permissions/managed_requests.rs`
3. new `apps/desktop/src-tauri/src/permissions/folder.rs`
4. new `apps/desktop/src-tauri/src/permissions/folder/host.rs`
5. new `apps/desktop/src-tauri/src/permissions/folder/host/tests.rs`
6. new `apps/desktop/src-tauri/src/permissions/folder/host/tests/worker.rs`
7. `apps/desktop/src-tauri/src/project_repository/window_sources/approval.rs`
8. `apps/desktop/src-tauri/src/project_repository/window_sources/approval/tests.rs`
9. `apps/desktop/src-tauri/src/project_repository/window_sources/supported.rs`
10. `apps/desktop/src-tauri/src/project_repository/window_sources/operation.rs`
11. `apps/desktop/src-tauri/src/project_repository/sqlite_folder/api/model.rs`
12. `apps/desktop/src/lib/api.ts`
13. new `apps/desktop/src/features/project-sources/permissionContract.ts`
14. new `apps/desktop/src/features/project-sources/permissionContract.test.ts`
15. `.github/workflows/storage-benchmark.yml`
16. `.github/workflows/storage-native-check.yml`

The supported.rs `open_for_test` helper exists only under cfg(test) and opens a
genuine FCP1 fixture through actual WindowSources without the picker. Production
folder selection is unchanged. No Cargo/native/C++/worker protocol/Tauri capabilities/
migration/version/dependency changes. No new user text or styles.

## Evidence and isolated CI review

A separate copy with exactly these sources passed **407 frontend tests /122 files**,
production build, lint, i18n (1483 messages) and Rustfmt. Every other tracked frontend/
Rust/workflow source matches the base; applicability passed. Ten new transport tests
cover legacy wire shape, original source, deep Commands/ReviewPlan capture, the host
flag and refusal of malformed receipts.

The new Rust tests **have not executed yet**. Twelve genuine SQL/FCP1 cases cover
policy/overrides, corrupt settings, zero managed project rows, wrong source/window/
project/commit, physical same-ID folders, audit failure/tampering, denial, replay/
reopen and SQL waits without registry locks. The window suite adds refusal of a
prepared request with foreign project/revision. An explicit ignored native test
checks Model with ReviewPlan → rejection of false volume claims without changing
the folder → actual 40→80 mm CAD commands → Undo/Redo, including automatic policy
and STEP inspection.

The owned [review workflow](../../.github/workflows/storage-permission-review.yml)
applies the SHA256-checked proposal **only in a temporary CI checkout**, records
exactly 16 staged paths/hashes, and runs frontend, both strict Clippy configurations,
SQL and the explicit genuine worker test. The branch contains review artifacts;
shared permission source waits for specification §5 agreement. Such CI validates the
proposal, not its runtime application in the task branch. After agreement, checks
on the actually applied current SHA remain required. This is not installed-app
acceptance; the local Windows SDK is incomplete.
