# Editor and AI-review preview: original opening

[Русский](storage-preview-renderer-source.md) · [State foundation](storage-workspace-source-state.en.md) · [Host API](../en/storage-folder-host-api.md)

Status: **prepared, unapplied; separate approval not yet requested**.
Issue #31 / PR #35, implementation wenlixxq, acceptance lovlyDev.
The exact [patch](storage-preview-renderer-source.patch) applies **after**
`storage-workspace-source-state.patch`, including its preference-key correction.
Backend preview approval does not authorize this dependent frontend scope.

## Behavior

`useModelPreview` receives the original opening binding. Folder build/cancel calls
pass its FolderSourceRef to the existing IPC. An explicit missing or mismatched
binding never becomes a managed call. Opening changes immediately hide old geometry,
cancel only the original source group and dispose late objects. Retired task errors
cannot appear in the new opening. Commit changes within an opening restart strict
preview IO without retaining the previous geometry. Text changes within the same
source retain existing debounce, serialized scheduling and previous geometry until
the new preview is ready.

ProgramDialog passes the binding and keeps folder drafts in memory through the
shared source preference. Revision keys participate in draft identity; retired
setters cannot change a new key or write managed preferences. Global editor settings
retain their existing route.

App captures the AI-review binding before confirmation awaits, preserving an already
supplied binding. CandidateReview checks the original opening against the active one
alongside project/revision checks. Candidate preview and the separate saved GLB copy
use that source. Opening another folder with identical project/revision IDs blocks
acceptance. Baseline loading hides retired scenes and disposes late objects; the
previously prepared modelLoader performs host reads and post-decode source checks.

## All 8 paths

Relative to `apps/desktop/src/`:

1. `features/model-preview/useModelPreview.ts`
2. new `features/project-sources/previewRequests.test.tsx`
3. `app/dialogs/ProgramDialog.tsx`
4. `features/agents/review/CandidateReview.tsx`
5. `features/agents/review/useSavedBaseline.ts`
6. `app/App.tsx`
7. new `features/project-sources/baselineSource.test.tsx`
8. new `features/project-sources/candidateSource.test.tsx`

## Checks and limits

Complete isolated snapshot with both patches: **347 tests / 115 files PASS**,
production build PASS, i18n 1483 PASS. This patch adds 13 tests; existing preview
scheduling and AI-review tests remain. The foundation separately passed 334 tests /
112 files and its build. Dependent patch applicability passed against that isolated
foundation. These are renderer fixtures/mocks, not worker geometry or installed-app
acceptance. Native preview has separately passed real STEP/FCP1 checks.

No Rust/native/Cargo/protocol/backend IPC/SQL/version changes, new user text or visible
folder button. Remaining measurement/history/import/export consumers, folder writers
and permission UI are not connected. These patches do not complete stage 4 or prove
an available end-user folder flow. Specification §5 requires agreement before shared
frontend/App edits.
