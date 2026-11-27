# Original-folder manual section — exact patch

[Русский](storage-manual-section-source.md) · [Host API](../en/storage-folder-host-api.md)

Status: **applied after explicit owner agreement for all six files; native CI PASS on fcd7ab1**. Issue #31 / PR #35; implementation wenlixxq,
agreement and acceptance lovlyDev. Runtime base `64bde10`.
Diff: [storage-manual-section-source.patch](storage-manual-section-source.patch).

## Contract and six files

Manual sections of saved STEP retain RepositoryRead for the original opening.
Authored CAD IR is unnecessary. Strict complete-snapshot, current-revision and
sealed-STEP checks surround worker execution. Unlike authored measurements,
metadata-only changes still invalidate this request, preserving existing behavior.
CAD grouping comes from the original reader; the worker algorithm is unchanged.

Verified bytes of the single STEP move into the existing stage_assets_with.
No managed AppData fallback or extra full staging-buffer clone is introduced.
Existing section_model accepts optional sourceId/expectedCommitId; incomplete pairs
fail, the host verifies window and commit, and folder errors cannot fall back.
Existing managed calls and test hooks remain. SourceRef is never persisted.

Rust paths relative to `apps/desktop/src-tauri/src/`:

- `model_section/capture.rs`
- `model_section/build.rs`
- `model_section/build_tests.rs` — existing fixture module only gains pub(super).
- `model_section/mod.rs`
- new `model_section/folder_build_tests.rs`

Frontend: `apps/desktop/src/features/viewer/sections/sectionApi.ts`.
The API accepts an optional FolderSourceRef while preserving source SHA checks.
Browser folder requests reuse the translated PROJECT_FOLDER_UNSUPPORTED error.

## Checks and unfinished gates

Applicability and staged rustfmt passed. Full frontend TypeScript checking with
all five staged CAD APIs virtually overlaid passed before application. Production build also passed in the working branch after the approved application.
The new native regression is implemented and **passed on fcd7ab1**: a real 40×20×10
STEP is copied to FCP1 without authored program/preview; its managed twin is corrupted.
Assertions cover source hash, 120 mm section perimeter, rejection after closing
during actual worker execution, unchanged SQL/app state and transient cleanup.
All three fcd7ab1 CI runs succeeded: Quality42226650589, comparison42226643155, native42226643167. The terminal native log confirms explicit raw_folder_step_section_uses_original_source_and_refuses_late_close PASS; strict Clippy and 15 window-source tests also passed.

This six-file backend patch does not add folder controls; stage 4 is incomplete. The production section caller is connected by the separately approved [renderer patch](storage-section-renderer-source.en.md).
No native/Cargo/C++/protocol/permissions/SQL/version changes. Shared model_section
and frontend files received explicit owner agreement under specification §5 before application.
