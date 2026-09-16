# Architecture
[Documentation](index.md) · [Русский](../ru/architecture.md)

| Layer | Responsibility | Source |
| --- | --- | --- |
| React / Zustand | Workspace, UI state, approval prompts | [App](../../apps/desktop/src/app/App.tsx), [store](../../apps/desktop/src/stores/workspace.ts) |
| Three.js | Rendering, camera, selection, motion | [Viewer](../../apps/desktop/src/features/viewer/Viewer.tsx) |
| Tauri / Rust | Validation, file access, processes, SQLite | [Backend](../../apps/desktop/src-tauri/src/lib.rs) |
| CAD worker | Restricted CadQuery execution, STEP/GLB | [Worker](../../apps/desktop/src-tauri/scripts/model_program.py) |
| Updater | Signed downloads, preferences and backup | [UI](../../apps/desktop/src/components/Updates.tsx), [backend](../../apps/desktop/src-tauri/src/updates.rs) |

The [CAD document](../../apps/desktop/src-tauri/src/cad_document.rs) is versioned JSON with stable feature IDs, typed operations and backward references. Supported operations are rectangle, circle, extrude, translate and boolean. Validation precedes conversion into restricted internal Python. A [plate-with-hole example](../fixtures/plate-hole.cad.json) demonstrates dependencies.

Revision.program stores either the JSON document or compatible Python source. The document schema version is independent of the application version. Revisions are immutable; restoration creates a new revision. Geometry failures do not publish a successful revision.

The CLI receives only the authorized model context. Backend commands own filesystem paths and process lifecycles. Geometry and agent processes are separate from the web UI. See [security](../../SECURITY.md) and [data storage](data.md).
