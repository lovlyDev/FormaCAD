# Folder attachments through existing IPC — exact patch

[Русский](storage-folder-attachments.md) · [Host API](../en/storage-folder-host-api.md)

Status: **applied after lovlyDev approved all six files** on 4 November 2026.
Applied-revision checks are running. Patch base:
`1c15f96ba607d2745c26889c889696b27a15ee42`, Issue #31 / PR #35.
Owner: wenlixxq; shared-scope agreement and acceptance: lovlyDev.

## Contract and six paths

- `apps/desktop/src-tauri/src/artifacts.rs`: existing `read_project_file` retains
  managed `{projectId, name}` calls. Folder calls additionally require
  `{sourceId, expectedCommitId}` together and invoke `WindowSources::read_asset`
  with the actual invoking Tauri window. Partial references fail without fallback.
- `apps/desktop/src/lib/api.ts`: optional `FolderSourceRef` for `readProjectFile`;
  existing calls retain their IPC arguments, folder calls send key and commit.
  A browser folder reference is refused before invoke.
- `apps/desktop/src/features/project-catalog/api.ts`: the same optional reference
  for the connected `readProjectSnapshot`; browser calls cannot replace folder
  selection with a managed project sharing its ID.
- `apps/desktop/src/features/project-sources/contract.ts`: shared
  `{readonly sourceId, readonly commitId}` type, an unpersisted window reference
  rather than a writer grant.
- `apps/desktop/src/i18n/ru.json` and `en.json`: translations for
  `PROJECT_SOURCE_LIMIT` and `PROJECT_FOLDER_UNSUPPORTED`, including the browser
  refusal through i18n.

The response remains a data URL. File membership, size and checksum come from the
saved snapshot and are checked by the existing bounded CAS reader. The original
view is rechecked after encoding. No caller-supplied path/descriptor becomes
authority; global SQLite cannot substitute a source. No duplicate attachment IPC
is added; the legacy Rust helper remains compatible.

## Evidence and limitations

Exact diff: [storage-folder-attachments.patch](storage-folder-attachments.patch).
`git apply --check` and rustfmt of the staged Rust file passed. This shared patch
is applied; strict compilation is pending. Owned host reader tests were separately pushed
in `1c15f96`: two actual folders sharing ID/name with different bytes, unchanged
source SQL during reading, foreign commit, missing name, path traversal, old
commit after publication and closed source. That base passed
[Storage comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41807057154):
98 folder tests, two window/asset tests, both strict Clippy modes and both
measurement steps. This does not validate compilation of the subsequently applied shared patch.

After application: strict Clippy, window/asset packet, build, check:i18n and existing
frontend checks on the applied SHA. Spec §5 requires agreement for artifacts,
frontend API and i18n. Buttons, workspace source state, geometry/model IPC and
recent index remain unwired; this does not complete the folder user flow.
No Cargo/native/protocol/permissions/SQL schema/version changes.
