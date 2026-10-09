# Architecture
[Documentation](index.md) · [Русский](../ru/architecture.md)

For feature flows and module boundaries, see the [code map](internals.md), [modeling](modeling.md), [AI](ai.md), [projects](projects.md), and [viewer](viewer-and-files.md) guides.

| Layer | Responsibility | Source |
| --- | --- | --- |
| React / Zustand | Workspace, UI state, approval prompts | [App](../../apps/desktop/src/app/App.tsx), [store](../../apps/desktop/src/stores/workspace.ts) |
| Three.js | Rendering, camera, selection, motion | [Viewer](../../apps/desktop/src/features/viewer/Viewer.tsx) |
| Tauri / Rust | Validation, file access, processes, SQLite | [Backend](../../apps/desktop/src-tauri/src/lib.rs) |
| Native CAD worker | Isolated CAD IR v2 execution with C++/OpenCascade; exact STEP and GLB preview | [Worker](../../apps/desktop/src-tauri/src/bin/forma-cad-worker.rs), [kernel](../../cad-core/CMakeLists.txt) |
| Compatibility worker | Restricted CadQuery execution for older models | [Python worker](../../apps/desktop/src-tauri/scripts/model_program.py) |
| Updater | Signed downloads, preferences and backup | [UI](../../apps/desktop/src/components/Updates.tsx), [backend](../../apps/desktop/src-tauri/src/updates.rs) |

The [CAD IR v2 document](../../apps/desktop/src-tauri/src/cad_ir/mod.rs) uses stable parameter, feature and body IDs, typed operations and backward references. [Validation](../../apps/desktop/src-tauri/src/cad_ir/validation.rs) precedes native execution. The [feature dependencies](../../apps/desktop/src-tauri/src/cad_ir/dependencies.rs) support suppression and per-body rollback; [revision comparison](../../apps/desktop/src-tauri/src/cad_ir/diff.rs) reports structural changes. The bounded [2D sketch solver](../../apps/desktop/src-tauri/src/cad_ir/sketch.rs) resolves local point constraints before the [native executor](../../apps/desktop/src-tauri/src/native/document.rs) builds exact BREP geometry. Selected rectangular-solid edges use a deliberately limited [semantic selector](../../cad-core/src/box_edge_selector.cpp) and are re-resolved during rebuild. Older v1 documents still use the [compatibility adapter](../../apps/desktop/src-tauri/src/cad_document.rs); a [plate-with-hole example](../fixtures/plate-hole.cad.json) demonstrates that older schema. See [sketches](sketches.md) for supported constraints and limits.

Revision.program stores either the JSON document or compatible Python source. The document schema version is independent of the application version. Revisions are immutable; restoration creates a new revision. Geometry failures do not publish a successful revision.

The CLI receives only the authorized model context. The [native planner](../../apps/desktop/src-tauri/src/agents/repair.rs) checks candidate geometry in the isolated worker and permits at most two correction attempts under the original requirements for a structured kernel error. Backend commands own filesystem paths and process lifecycles. Geometry and agent processes are separate from the web UI. See [security](../../SECURITY.md) and [data storage](data.md).
