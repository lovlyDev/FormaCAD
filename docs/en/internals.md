# Code map and data flows

[Documentation](index.md) · [Русский](../ru/internals.md) · [Architecture](architecture.md) · [Modeling](modeling.md)

This map helps locate implementation changes; it does not replace the user guides. The source tree contains the desktop React npm workspace, Tauri/Rust backend, C++ CAD kernel, Python compatibility route, validation scripts, and build workflows. [package.json](../../package.json) defines top-level commands; the [Tauri entry point](../../apps/desktop/src-tauri/src/lib.rs) registers plugins and commands. Application version numbers are synchronized separately across npm, Cargo, Tauri, and the lockfile.

## Where to change a feature

| Area | Main files | Responsibility |
| --- | --- | --- |
| Workspace | [App.tsx](../../apps/desktop/src/app/App.tsx), [workspace.ts](../../apps/desktop/src/stores/workspace.ts), [persistence.ts](../../apps/desktop/src/lib/persistence.ts) | Navigation, UI state, startup recovery. |
| Projects | [Dashboard.tsx](../../apps/desktop/src/app/Dashboard.tsx), [projects.rs](../../apps/desktop/src-tauri/src/projects.rs), [cadpack.rs](../../apps/desktop/src-tauri/src/cadpack.rs) | Cards, project lifecycle, bundle transfer. |
| CAD IR | [mod.rs](../../apps/desktop/src-tauri/src/cad_ir/mod.rs), [commands.rs](../../apps/desktop/src-tauri/src/cad_ir/commands.rs), [validation.rs](../../apps/desktop/src-tauri/src/cad_ir/validation.rs), [dependencies.rs](../../apps/desktop/src-tauri/src/cad_ir/dependencies.rs) | Format, edits, references, invariants. |
| Sketches | [sketch.rs](../../apps/desktop/src-tauri/src/cad_ir/sketch.rs), [Sketch2dEditor.tsx](../../apps/desktop/src/components/Sketch2dEditor.tsx) | Constraint solving and 2D profile editor. |
| Native geometry | [worker.rs](../../apps/desktop/src-tauri/src/native/worker.rs), [document.rs](../../apps/desktop/src-tauri/src/native/document.rs), [C++ core](../../cad-core/src/) | Process boundary, B-Rep construction, STEP/GLB. |
| Older models | [cad_document.rs](../../apps/desktop/src-tauri/src/cad_document.rs), [legacy.rs](../../apps/desktop/src-tauri/src/modeling/legacy.rs), [model_program.py](../../apps/desktop/src-tauri/scripts/model_program.py) | CadQuery compatibility and restricted execution. |
| AI and processes | [agents](../../apps/desktop/src-tauri/src/agents/), [processes](../../apps/desktop/src-tauri/src/processes/), [permissions.rs](../../apps/desktop/src-tauri/src/permissions.rs) | CLI, context, progress, cancellation, grants. |
| Viewer | [Viewer.tsx](../../apps/desktop/src/features/viewer/Viewer.tsx), [CameraControl.tsx](../../apps/desktop/src/features/viewer/CameraControl.tsx), [model.ts](../../apps/desktop/src/lib/model.ts) | Three.js, camera, selection, meshes. |
| Files | [files.ts](../../apps/desktop/src/lib/files.ts), [files.rs](../../apps/desktop/src-tauri/src/files.rs), [artifacts.rs](../../apps/desktop/src-tauri/src/artifacts.rs), [conversion.rs](../../apps/desktop/src-tauri/src/conversion.rs) | Import, export, attachments, conversion. |
| Data and updates | [storage.rs](../../apps/desktop/src-tauri/src/storage.rs), [updates.rs](../../apps/desktop/src-tauri/src/updates.rs), [Updates.tsx](../../apps/desktop/src/components/Updates.tsx) | SQLite, migrations, version check and install. |

## Model-edit flow

A parameter or sketch panel creates a command → the [Tauri API](../../apps/desktop/src/lib/api.ts) invokes `apply_ir_commands` → [modeling.rs](../../apps/desktop/src-tauri/src/modeling.rs) applies the batch against the expected revision → CAD IR validates structure and dependencies → the [native worker](../../apps/desktop/src-tauri/src/bin/forma-cad-worker.rs) builds and validates the result → the successful document and artifacts are stored in a new revision → React receives the updated model. A failure does not replace the current successful revision. AI changes first pass through an additional planning and validation stage; see the [AI guide](ai.md).

SQLite is authoritative for project metadata and settings; file artifacts live under `projects/`. Both are needed for a complete backup. For schema changes, add a **new** SQL migration in [migrations](../../apps/desktop/src-tauri/migrations/) and do not alter an applied file; line endings must be LF. `npm run check:migrations` verifies this. See [data](data.md).

## Build and documentation flow

Add new user-facing strings to both [ru.json](../../apps/desktop/src/i18n/ru.json) and [en.json](../../apps/desktop/src/i18n/en.json) immediately. After every feature, update both topical guides and navigation links; follow the [documentation process](documentation.md). Local checks are defined by [package.json](../../package.json), the [CI workflow](../../.github/workflows/ci.yml), and [development](development.md). The [release workflow](../../.github/workflows/release.yml) runs only for a new tag; publication of consolidated 1.2.6 is authorized after final checks. [Status](status.md) separates verified local builds from planned platform packages.

## Construction geometry modules

The [line editor](../../apps/desktop/src/components/sketch/SketchConstructionEditor.tsx) is separate from workplane and point editing. [Draft changes](../../apps/desktop/src/lib/sketchConstruction.ts) preserve shared points during deletion. Rust sketch code is divided into [structure checks](../../apps/desktop/src-tauri/src/cad_ir/sketch/structure.rs), [numerical solver](../../apps/desktop/src-tauri/src/cad_ir/sketch/solver.rs) and [outline checks](../../apps/desktop/src-tauri/src/cad_ir/sketch/outline.rs). The [guide](sketch-construction.md) covers storage, build flow and limitations.

## Sketcher 1.2.6

The editor separates contour, point, constraint, canvas and diagnostic tools in `src/components/sketch/`; `CadParameterEditor` owns parameter UI. Pure draft edits and reference cleanup live in `src/lib/sketch*` and `cadParameters`. Read-only `analyze_sketch` shares system/solver/profile and returns rank and residuals without kernel or SQLite. Builds send normalized contours through CXX into `profile_prism.cpp`. See [profiles](sketch-profiles.md), [constraints](sketch-constraints.md), [parameters](cad-parameters.md), [diagnostics](sketch-diagnostics.md).
