# Sections of the displayed original opening — frontend

[Русский](storage-section-renderer-source.md) · [Host API](../en/storage-folder-host-api.md) · [Manual STEP backend](storage-manual-section-source.en.md)

Status: **prepared, unapplied; separate agreement for nine files required**.
Issue #31 / PR #35; implementation wenlixxq, agreement and acceptance lovlyDev.
Base `bc9878a2c76e9df29372ede720f4476497259b94` plus the explicitly approved and
applied six-file manual-section patch. The exact [diff](storage-section-renderer-source.patch)
does not change Rust, Cargo, native, backend IPC, SQL, permissions or versions.

## Behavior and route

The existing section panel passes its original opening to both manual STEP sections
and selected-face sections. Backend API requests use the original FolderSourceRef;
foreign face-capture overrides are refused. A manual request keeps its original
commit and restarts after metadata refresh using the next commit. The running
request is never retargeted. Face requests retain geometry-compatible results on
metadata refresh; the next request receives the new original-source reference.

`sceneRead` distinguishes same-opening checks from current-geometry checks. A retained
object can keep its local GPU plane within its own opening during geometry replacement.
New exact curves wait for actually displayed current geometry. Objects from another
folder, even with equal project/revision IDs, cannot borrow a new source. Unregistered
folder objects cannot issue exact queries. Process-local references never enter GLB/JSON.

Manual result/error adoption requires the original current snapshot. Source switches
hide old replies. The existing single-unfinished-native-call sequencing between
manual and face modes remains. Retired explicit face Calculate callbacks cannot
start commands in a new opening. Folder panel preferences stay in RAM, reset on
reopen and neither read nor overwrite managed preferences. Metadata refresh within
the same opening preserves those preferences.

## All nine paths

Relative to `apps/desktop/src/`:

1. `features/viewer/sections/face-section/faceSectionCapture.ts`
2. `features/viewer/sections/face-section/faceSectionApi.ts`
3. `features/viewer/sections/face-section/useFaceSection.ts`
4. new `features/project-sources/faceSectionSource.test.tsx`
5. `features/project-sources/sceneRead.ts`
6. new `features/viewer/sections/sectionSource.ts`
7. `features/viewer/sections/useSectionView.ts`
8. `features/viewer/sections/useSectionView.test.tsx`
9. new `features/project-sources/sectionSource.test.tsx`

The existing test now initializes a managed opening through normal setProject,
instead of assigning project directly without its binding. All mode-switching,
native call-count and result-restoration assertions remain.

## Evidence and limits

A separate snapshot with exactly these frontend changes over the approved manual
API passed **397 tests /121 files**, production build, lint and i18n (1483 messages).
Every other tracked frontend source matches the working base; exact diff applicability
passed. Fifteen new tests comprise six face and nine section cases, including the
actual frontend API invoke contract over mocked transport and refusal of another
STEP hash. These checks are not OCCT execution or installed-app acceptance.

No user text or styles are added. Folder controls, writer/permission UI and
history/import/export are not connected by this patch. Stage 4 is incomplete.
Specification §5 requires separate agreement before shared frontend/scene/section edits.
