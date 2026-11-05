# Original-folder preview — exact patch

[Русский](storage-preview-source.md) · [Host API](../en/storage-folder-host-api.md)

Status: **applied after lovlyDev approved all seven files** on 5 November 2026.
IPC base `5cb38ef` passed native verification; Issue #31 / PR #35.
Implementation: wenlixxq; agreement and acceptance: lovlyDev.
Diff: [storage-preview-source.patch](storage-preview-source.patch).

## Contract and seven files

One preview host retains the original RepositoryRead. The managed entrypoint and
deterministic test worker hook adapt to that host. Checks precede work, follow the
worker and follow the final task-map cleanup await. Metadata-only changes still
invalidate previews, preserving strict complete-snapshot semantics.

FCP1 uses its original opening task scope in the task map and CadTaskGate. Folders
sharing project IDs cannot block or cancel each other's previews; the global CAD
semaphore remains. Existing preview_model and cancel_model_preview gain optional
sourceId/expectedCommitId; the host verifies real window, project and commit.
Source errors cannot fall back to managed storage. Cancellation requires a valid
source reference; references to an old commit are rejected after commit changes.

Imported STEP reads use the bound reader. Staging reuses the verified common
model_measurement/staging module, translating only its lifecycle errors back to
existing preview codes. Worker algorithms, packet encoding, limits and GLB remain.
Direct Rust build_with_project/build_with_executable seams remain; one build_staged
handles legacy and bound inputs. The unused legacy verifier is removed. Existing
test assertions remain intact.

Rust paths relative to `apps/desktop/src-tauri/src/`:

- `model_preview/host.rs`
- `model_preview/host/tests.rs` — shared reader in the existing cleanup-await test.
- `model_preview/build.rs`
- `model_preview/imports.rs`
- `model_preview/mod.rs`
- new `model_preview/host/folder_tests.rs`

Frontend: `apps/desktop/src/features/model-preview/previewApi.ts` — optional
FolderSourceRef on preview/cancel, preserving packet/hash/GLB checks and old calls.
The existing translated PROJECT_FOLDER_UNSUPPORTED error is reused.

## Checks and limits

Applicability, staged rustfmt and full TypeScript checking with the staged preview
API virtually overlaid passed. After application, 294 frontend tests across 104
files and the production build passed. At `4b9615843b6cf72eeaecf62f28fc76f48d904ad5`,
[both strict Clippy modes/comparison](https://github.com/lovlyDev/FormaCAD/actions/runs/42050145205)
and [native CI](https://github.com/lovlyDev/FormaCAD/actions/runs/42050145085) passed.
Downloaded job `127323748900` explicitly names all three preview tests as PASS;
their separate invocation returned exactly 3 PASS. Full packet: 540 library PASS
(18 ignored), 11 smoke PASS, 41 host PASS, two measurement PASS and three roundtrip PASS.
The owned workflow requires three executed preview tests, preventing empty-filter success.
Three tests create a genuine 40×20×10 FCP1 STEP and corrupt its managed twin sharing the
same ID. They assert volume 8000, bounds, program SHA, GLB, task group, rejection
after worker close and after close during the cleanup mutex await, SQL/app purity
task release and no remaining transient inputs in `.transient/previews`.
The added regression in the same approved new test file opens
two independent folders sharing project IDs, queues both real preview requests,
cancels only the first and checks the second result's geometry. This addition
changes neither the production diff nor the seven agreed paths. After application,
all three preview scenarios executed in that native CI. This does not replace
future workspace/source-state and GUI checks.
[Quality at that SHA](https://github.com/lovlyDev/FormaCAD/actions/runs/42050157179)
also completed successfully.

Workspace source state and folder controls remain unwired; stage 4 is incomplete.
No native/Cargo/C++/protocol/permissions/SQL/version changes.
Specification §5 requires agreement for shared preview/frontend files before application.

See [CI and review artifacts](storage-ci-review-artifacts.en.md) for branch-owned
storage workflow triggers on review-only changes.
