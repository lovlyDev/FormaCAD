# Cursor and pie snap: original opening and latest request

[Русский](storage-snap-renderer-source.md) · [Host API](../en/storage-folder-host-api.md) · [Measurements](storage-measurement-renderer-source.en.md)

Status: **applied after explicit owner agreement for all nine files**.
Issue #31 / PR #35, implementation wenlixxq, acceptance lovlyDev.
Base `db3ce03d643fcc4adc2ed592fbab2ac4c3f4046e` includes the applied 16-file
measurement scope. The exact [patch](storage-snap-renderer-source.patch) changes
nine paths and excludes the unwired face-section prototype.

## Behavior

A snap captures its original project/opening/backend binding, revision, saved
program/programBase and primary STEP SHA/size. Its readonly FolderSourceRef goes
to the existing exactSnap API. After await, the original opening and geometry
context are checked; an equal-revision project or folder cannot adopt the old
result. Compatible metadata-only refresh preserves a running request without
changing its original commit reference; a new operation captures the fresh commit
before its own await.

One controller selects the latest cursor placement across pointer and pie requests.
Older worker operations may finish, but cannot overwrite newer pending/error/position
through their result, failure or finally. Symbol identity avoids counter reuse on
overflow. It creates no opaque host token; snapshot/request identity stays in RAM
and grants no writer permission.

Success releases its listener. A displayed failure keeps only one bounded watcher
until source change or another request, preventing cross-opening errors. No entire
project is cloned: primitive original fields are compared. No source is reopened by
path or ID. Native scheduling/cancellation APIs remain unchanged; the controller
revokes adoption rather than promising to stop the worker.

Pointer handling checks its original displayed object, unmount and placement number.
A returned result is checked again immediately before cursor adoption. A pie captures
its edge/source when opened and never retargets a different edge/folder afterward.
World origin also supersedes an older native snap. Ordinary worker-error approximation
remains explicitly non-exact and only applies while the original context is current;
context change returns null rather than an old point.

The original read context is captured before load await and attached to the loaded
object afterward through a WeakMap, never userData, GLB or JSON. A retired object cannot borrow the updated global project.
Unregistered folder objects cannot borrow active folder authority. Viewport commands
pass the actual object into the snap pie, also checking the selected edge scene token.
An old menu cannot reset another source cursor through world origin. Host stale-source/
provenance refusals return null without a mesh-estimate fallback.

## All 9 paths

Relative to `apps/desktop/src/`:

1. new `features/snapping/snapRequest.ts`
2. `features/snapping/resolveSnap.ts`
3. new `features/pie-menus/edgeSnapActions.ts`
4. `features/pie-menus/viewportPies.ts`
5. `features/snapping/cursor/Cursor3D.tsx`
6. new `features/project-sources/snapSource.test.ts`
7. new `features/project-sources/sceneRead.ts`
8. `features/model-apply/useCommittedScene.ts`
9. `features/viewer/view-commands/viewportCommands.ts`

The independent native edge action moves out of the mixed viewport pie module.
Other menu actions/text remain. No new user text/styles or Rust/native/Cargo/backend
IPC/SQL/version changes.

## Checks and limits

A separate base snapshot with **only these nine files** passed **382 tests /119 files**,
production build, lint and i18n (1483 EN/RU messages). Nineteen new tests cover original
routing, equal-ID reopen/project change, metadata continuation, stale pending/error/
finally, scene close, next-request commit, retired menus and world-origin supersession.
Exact diff applicability passed. These renderer mocks are not new OCCT measurements
or installed-app acceptance. The composite with face core does not replace exact validation of these nine paths.

This patch does not connect face/manual sections or writer/permission UI. No visible
folder button is added; stage 4 is incomplete. Specification §5 agreement
was satisfied by explicit owner agreement before the shared source edits.
