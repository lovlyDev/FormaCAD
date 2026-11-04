# Measurements through their original source — exact patch

[Русский](storage-measurement-source-core.md) · [Host API](../en/storage-folder-host-api.md)

Status: **applied after lovlyDev approved all 10 files** on 4 November 2026.
Base `5899c64fe30bd5f83beb31caceaf7884acd56948`, Issue #31 / PR #35.
Owner: wenlixxq. Exact diff: [storage-measurement-source-core.patch](storage-measurement-source-core.patch).

## Contract

Measurement capture retains RepositoryRead for the original session. Original
source, opening and revision checks surround CAD execution. Body, pair, face
section and snap receive source-bound Rust entrypoints while preserving existing
managed entrypoints and test hooks. IPC and frontend wiring are outside this patch.

Queued name/message updates remain compatible only after recapturing through the
same reader and comparing original revision, geometry bindings, preview and opening
task group. Only the validated attachment read context advances. Closing/reopening,
another folder with the same ID, or changed geometry cannot replace the source.

A separate staging module asynchronously reads verified source STEP bytes and moves
them into the existing native stager without another full buffer clone. Existing
file and aggregate 128 MB limits remain. Native code does not change.

## Exact ten files

Paths relative to `apps/desktop/src-tauri/src/`:

- `model_measurement/snapshot.rs`
- `model_measurement/capture.rs`
- `model_measurement/mod.rs`
- `model_measurement/build.rs`
- `model_measurement/build_tests.rs`
- `model_pair_measurement/build.rs`
- `model_face_section/build.rs`
- `model_snap/build.rs`
- new `model_measurement/staging.rs`
- new `model_measurement/folder_build_tests.rs`

New tests use a real imported STEP, an independent FCP1 copy and deliberately
corrupted managed attachments with the same project ID. They check volume 8000,
40×20×10 bounds, a queued metadata-only commit, preserved messages, read purity and
rejection after closing during actual CAD work. Existing assertions remain;
the fixture module only gains pub(super) visibility.

## Validation and limits

Patch applicability and applied Rust formatting passed. Compilation and new runtime
tests are pending CI. Strict Clippy and native tests, including existing managed
regressions, are required. The owned native workflow separately executes both new
folder tests and requires exactly 2 PASS. The base passed
[Quality](https://github.com/lovlyDev/FormaCAD/actions/runs/41817194409),
[comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187435) and
[native](https://github.com/lovlyDev/FormaCAD/actions/runs/41817187467).

No Cargo, C++, worker protocol, permissions, SQL, version or frontend changes.
Folder-opening controls and the complete stage 4 user flow remain unfinished.
Specification §5 requires separate approval for these shared measurement/snap files.
