# Modeling and CAD IR v2

[Documentation](index.md) · [Русский](../ru/modeling.md) · [Sketches](sketches.md) · [Version status](status.md)

This guide describes the current local source tree. The published installer may be older; see [project status](status.md). A project, model document, and revision are distinct. A project groups history and files; a document describes parameters, features, and bodies; a revision records one saved state and its build result.

## Model construction

A CAD IR v2 document contains `schemaVersion: 2`, `revisionId`, and `parameters`, `features`, and `bodies` arrays. Element IDs remain stable across edits. A parameter has a name and a canonical millimeter value. A feature dimension is either a literal value or a reference to a parameter, so changing one parameter rebuilds dependent features. A body names its final feature with `sourceFeatureId`. Multiple bodies can be independent or participate in Boolean operations. See the [types](../../apps/desktop/src-tauri/src/cad_ir/mod.rs) and [validation rules](../../apps/desktop/src-tauri/src/cad_ir/validation.rs).

Current schema operations:

| Group | Operations | Result |
| --- | --- | --- |
| 2D basis | `sketch2d`, `rectangle`, `circle` | A profile for extrusion; the interactive sketch currently supports one polygon loop. |
| Solids | `sphere`, `cone`, `extrude` | A solid; extrusion refers to an earlier profile. |
| Processing | `hole`, `fillet`, `filletEdge`, `chamfer`, `translate` | A derived result from an existing body feature. |
| Geometry combination | `boolean` with `union`, `cut`, `intersect` | Union, subtraction, or intersection of two results. |

Feature order matters: references must point to available earlier results of the correct type. Semantic edge targeting for `filletEdge` currently covers recognized rectangular-solid edges, not arbitrary topology. General stable topology references remain on the [roadmap](roadmap.md). See [the current edge selector](../../cad-core/src/box_edge_selector.cpp).

## Local editing

Dimension changes in the parameter panel edit the document locally. `set_parameter` changes a named parameter; `set_literal` changes an unbound numeric dimension. If a dimension is bound to a parameter, edit that parameter instead. Other commands add parameters, features, and bodies; change a body's source feature; rename or suppress features; and edit sketch points, plane, and constraints. [Command application](../../apps/desktop/src-tauri/src/cad_ir/commands.rs) checks the expected revision and validates the complete batch before publication, preventing a stale edit from overwriting a newer revision.

Suppression temporarily excludes a supported feature from the build while validating dependencies. Rolling a body back changes its final source feature without erasing later features from the document. The [dependency logic](../../apps/desktop/src-tauri/src/cad_ir/dependencies.rs) defines these rules. The source editor is available for compatible models, but native CAD IR does not become arbitrary Python; the routes are explained in [architecture](architecture.md).

## Validation and persistence

The shared editor/AI/command application route, temporary workspace ownership, permissions, cancellation and commit checks are described in [checked host model transactions](model-apply-transactions.md).

After commands, Rust checks structure, IDs, dimensions, and dependencies. A separate [CAD worker](../../apps/desktop/src-tauri/src/bin/forma-cad-worker.rs) runs [OpenCascade](../../cad-core/CMakeLists.txt), builds and validates B-Rep geometry, and prepares exact STEP plus a GLB preview. Only a successful result becomes a new revision. A geometry failure leaves the previous revision available. See [native execution](../../apps/desktop/src-tauri/src/native/document.rs). Structured error codes are translated at the UI boundary, while technical details remain available separately.

Revisions are immutable. Restoring an older state creates another revision on the current history. Comparison by stable ID shows added, removed, and changed parameters, features, and bodies; stored exact geometry can also supply metrics. It is not a full surface-to-surface geometric diff. See [history](../../apps/desktop/src-tauri/src/cad_ir/history.rs), [structural diff](../../apps/desktop/src-tauri/src/cad_ir/diff.rs), and [comparison UI](../../apps/desktop/src/app/RevisionComparison.tsx).

## Compatibility and limits

Older CadQuery documents remain openable through the [compatibility route](../../apps/desktop/src-tauri/src/cad_document.rs) and [restricted Python worker](../../apps/desktop/src-tauri/scripts/model_program.py). Native schema v2 does not need Python to build. Document `schemaVersion` is independent of the application version. An imported mesh supports viewing and mesh export, but does not automatically become an exact parametric B-Rep or STEP solid. See [viewer and files](viewer-and-files.md).

Use the CAD IR and native worker tests in [development](development.md) to verify modeling changes. For user actions, also verify that a failed build creates no successful revision and that restoration adds a new one. The interactive sketch and constraint limits have their own [guide](sketches.md).

## Editing parameters and sketches

Local 1.2.6 exposes named parameters in the feature tree. Bind line dimensions to parameters, analyze the sketch and explicitly apply solved coordinates to the draft. See [parameters](cad-parameters.md), [contours and holes](sketch-profiles.md), [constraints and editing](sketch-constraints.md), [diagnostics](sketch-diagnostics.md). Saving 3D geometry still requires ordinary revision regeneration; analysis leaves the saved model unchanged.

Review before building: [model editor](model-editor.md) and [exact 3D preview](model-preview.md). A draft does not become saved geometry until a successful build.

[Typed AI edits and link commands ](command-api.md): revision-bound batches and explicit coordinate unlinking without movement.

[AI candidate review ](ai-candidate-review.md): interactive geometry and structural comparison before explicit acceptance.
