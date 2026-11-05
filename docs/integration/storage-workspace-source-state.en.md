# Workspace source and late-result guards — exact patch

[Русский](storage-workspace-source-state.md) · [Host API](../en/storage-folder-host-api.md)

Status: **prepared, unapplied**. Base `9604647389fa234fc87d57cd5a95b714a3b56d34`.
Issue #31 / PR #35; implementation wenlixxq, agreement and acceptance lovlyDev.
Diff: [storage-workspace-source-state.patch](storage-workspace-source-state.patch).

## Behavior and data

Workspace retains Project alongside a process-local opening binding. Managed
metadata preserves its request family; explicit activation creates a fresh family.
Folder bindings copy and freeze host FolderSourceRef. The local UUID guards renderer
callbacks, granting no native read/write authority. The host still validates window,
source and commit.

The store provides explicit managed/folder activation and compare-and-set folder
snapshot publication. Source switches reset topology/body selection; same-opening
metadata preserves compatible selection. Late managed save/event/CAD results cannot
replace equal-ID folder views. Folder read references cannot invoke managed
persistence. Retired folder selectors close through existing close_project_source.

Scene identity includes the opening alongside project/revision/asset seals, hides
another source's geometry immediately and disposes late objects. Folder model loading
uses host bytes even with an inline candidate. A pure check of the same source/commit
and file seal follows model decoding; stale objects are disposed. Ready geometry
survives metadata-only refresh without another asset load. Pending IO restarts with
the fresh commit reference and rejects older objects.

Root saves, primary managed authoring callbacks and CapturedApply gain renderer
opening guards. Late managed-access release errors cannot appear in another source
view. Access status is opening-scoped and hides a cached write mode until access
acquisition for the reopened equal-ID project completes. Folder read references
never acquire managed editing. Both catalogs
gain PROJECT_SOURCE_WRITE_NOT_AUTHORIZED. Folder project drafts remain in memory;
disabled persistence neither reads nor writes managed preferences or source/epoch
tokens. Old setters cannot change a newer opening's draft. Managed APIs remain.

## All 31 paths

Relative to `apps/desktop/src/`:

1. new `features/project-sources/sourceBinding.ts`
2. new `features/project-sources/sourceBinding.test.ts`
3. new `features/project-sources/sourceApi.ts`
4. new `features/project-sources/useWorkspaceSourceAccess.ts`
5. new `features/project-sources/sourceAction.ts`
6. new `features/project-sources/fileReadGuard.ts`
7. new `features/project-sources/useSourcePreference.ts`
8. new `stores/workspaceProject.ts`
9. new `stores/projectDefaults.ts`
10. `stores/workspace.ts`
11. `lib/persistence.ts`
12. new `lib/modelLoader.ts`
13. `lib/files.ts`
14. `features/model-apply/committedScene.ts`
15. `features/model-apply/committedSelectionScene.ts`
16. `features/model-apply/useCommittedScene.ts`
17. `features/project-catalog/useProjectNavigation.ts`
18. `features/model-apply/useRevisionEvents.ts`
19. `features/model-apply/capturedApply.ts`
20. `features/model-apply/useModelApply.ts`
21. `app/App.tsx`
22. `i18n/ru.json`
23. `i18n/en.json`
24. new `stores/workspaceSource.test.ts`
25. new `features/project-sources/sceneSource.test.tsx`
26. new `features/project-sources/sourcePreference.test.tsx`
27. new `features/project-sources/managedRequests.test.tsx`
28. new `features/project-sources/fileSource.test.ts`
29. new `features/project-sources/sourceAccess.test.tsx`
30. `features/project-access/useProjectAccess.ts`
31. new `features/project-sources/managedAccessScope.test.tsx`

Model loading and project-state transitions move out of mixed files/workspace
modules; re-exports preserve existing imports. No Rust/native/Cargo/worker protocol,
native permissions, SQL, version or backend IPC contract changes.

## Checks and unfinished scope

An isolated full base snapshot with these 31 files passed 332 tests across 112
files, production build, check:i18n (1483 EN/RU messages) and check:docs (190 base
documents). This includes 38 new state/renderer tests: same-ID switch/reopen,
metadata refresh, late save/event/CAD adoption, scoped geometry, refusal after
decoder await, object disposal, isolated preferences and read-only access. Exact
patch applicability passed. Shared working sources remain unchanged.

Frontend tests use renderer fixtures/mocks and do not replace actual CAD evidence.
The native preview base `4b96158` passed all three CI workflows. After application,
actual-head checks must repeat, with browser scenarios as wiring becomes available.

This establishes store/scene and primary callback guards. It **adds no folder
button**, does not wire all remaining geometry/history/import/export consumers and
does not implement folder writer/permissions UI. Read-only binding is not stage 4's
final outcome; visible folder flow awaits completion of its read/write route.
The separate manual-section patch remains unapproved and is not included here.
Specification §5 requires agreement before shared store/App/frontend/i18n edits.
