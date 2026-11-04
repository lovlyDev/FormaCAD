# Original measurement source over existing IPC

[Русский](storage-measurement-source-ipc.md) · [Core patch](storage-measurement-source-core.en.md)

Status: **prepared, unapplied**. Base `64bde10e5fe5174e783d79e5d6f20ac0b3a32323`.
Issue #31 / PR #35; implementation: wenlixxq; agreement and acceptance: lovlyDev.
Exact diff: [storage-measurement-source-ipc.patch](storage-measurement-source-ipc.patch).

## Contract and eight files

Four existing commands accept optional sourceId and expectedCommitId. Both absent
preserves managed behavior; an incomplete pair fails before worker execution.
Both present uses WindowSources.capture_read to verify the real invoking window,
project ID and commit, then runs the same source-bound CAD pipeline. Folder errors
cannot fall back to managed storage. taskScopeId is not accepted as authority.
Without a native client, folder requests fail with PROJECT_FOLDER_UNSUPPORTED.

Rust paths relative to `apps/desktop/src-tauri/src/`:

- `model_measurement/mod.rs`
- `model_pair_measurement/mod.rs`
- `model_face_section/mod.rs`
- `model_snap.rs`

TypeScript paths relative to `apps/desktop/src/`:

- `features/reference-measurements/measurementApi.ts`
- `features/pair-measurements/pairApi.ts`
- `features/viewer/sections/face-section/faceSectionApi.ts`
- `features/snapping/snapApi.ts`

Client APIs accept an optional FolderSourceRef and add only two fields to existing
requests. Existing callers, worker echo checks, source seals and document SHA
checks remain. Browser folder requests fail before invoke; the existing translated
PROJECT_FOLDER_UNSUPPORTED error is reused. There are no new user-facing strings.

## Checks and limits

Applicability and staged rustfmt passed. Full frontend TypeScript checking with
four virtually overlaid staged API files passed without modifying working sources.
These are not runtime evidence or a
complete user flow. After application, both strict compilation variants, native
tests and frontend tests/build are required. Narrow too_many_arguments allowances
apply only to four Tauri commands retaining compatible flat request fields.

Workspace must still pass the original FolderSourceRef and invalidate requests
and selection on source switch/close, including equal IDs. Controls, workspace
state, history, manual section and preview are not wired by this patch.
No Cargo/native/C++/worker protocol/permissions/SQL/version changes.
Specification §5 requires separate agreement before shared IPC/frontend edits.
