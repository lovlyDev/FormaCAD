# Modeling and CAD IR v2

[Documentation](index.md) · [Русский](../ru/modeling.md) · [Sketches](sketches.md) · [Version status](status.md)

This guide describes the current local source tree. The published installer may be older; see [project status](status.md). A project, model document, and revision are distinct. A project groups history and files; a document describes parameters, features, and bodies; a revision records one saved state and its build result.

## Model construction

### Torus (M3)

Immediately after creating a torus, press F9 in the viewport to adjust both radii. A successful adjustment replaces the last Undo step while retaining saved revisions. If the project or history changes, reopen the panel.

`make_torus` builds a solid OCCT torus at the origin around the Z axis. The major radius measures the distance to the centre of the tube section. The minor radius measures the tube radius. Both use millimetres. The major radius is capped at 10,000 mm. The minor radius and the difference between the two radii must exceed the OCCT tolerance. These constraints preserve an open hole and exclude degenerate and self-intersecting shapes.

In the desktop app, open an empty project or a project with a saved CAD IR v2 model. Choose Add → Torus, enter both radii, and select Build. The fields accept bare values in millimetres and values with units such as `1 cm` or `0.5 in`. The torus is added as a separate body at the model origin.

`AddFeature` and `AddBody` run as one transaction. The document stores `majorRadius` and `minorRadius` on the `torus` operation. Rust validates the radii and delegates construction to the worker. A successful build saves one revision. A validation or worker error preserves the previous model and history. `forma_torus_test` compares volume, area and extents with analytical values, checks BREP validity after a STEP round-trip, and verifies rejection of invalid radii.

A CAD IR v2 document contains `schemaVersion: 2`, `revisionId`, and `parameters`, `features`, and `bodies` arrays. Element IDs remain stable across edits. A parameter has a name and a canonical millimeter value. A feature dimension is either a literal value or a reference to a parameter, so changing one parameter rebuilds dependent features. A body names its final feature with `sourceFeatureId`. Multiple bodies can be independent or participate in Boolean operations. See the [types](../../apps/desktop/src-tauri/src/cad_ir/mod.rs) and [validation rules](../../apps/desktop/src-tauri/src/cad_ir/validation.rs).

### Datum geometry in the kernel (M3)

The `forma::datum` module builds OCCT construction geometry in model coordinates. A plane is a square face with a centre and normal, an axis is a segment with a midpoint and direction, and a point is a vertex. The kernel checks coordinates, direction and size. Construction elements have their own topology and require no positive volume.

In the Add menu, choose Datum plane, Datum axis or Datum point. Enter the coordinates in the model coordinate system. For a plane, also enter a normal and side length. For an axis, enter a direction and length. CAD IR stores `datumPlane`, `datumAxis` or `datumPoint` separately from bodies. The worker checks the geometry before the revision is saved. A project with only datums produces no body files. Datums appear alongside the model and leave solid mass and body STEP export unchanged.

Press F9 in the viewport immediately after creating a datum to adjust its parameters within the same Undo step. The previous revision is retained. A change to history makes the adjustment request stale.

To anchor a datum, select a planar face or straight edge and choose it under Datum reference. Earlier datum planes appear in the same list. Offset follows the original normal or tangent from the face centroid, edge midpoint or plane origin. Angle tilts the normal or direction toward a local tangent. F9 changes the offset, angle and size while retaining the source.

CAD IR stores the reference and its parameters in a `datumOnReference` operation. The worker resolves the semantic reference on each build and records the computed placement in preview metadata. Curved faces, circular edges and unsupported references are rejected. A failed anchor preserves the current revision. `forma_datum_test` checks measurements and BREP after a separate STEP round-trip. `forma_datum_anchor_test` checks exact positions and rejects stale references.

### Face extrusion (M3)

In face selection mode (`2`), select a planar box face or cylinder cap with a semantic reference. Press `E` to enter a signed length, or drag the arrow along the face normal and review the estimated distance in the same dialog. Positive distance adds material. Negative distance cuts into the body. Enter applies the distance after the usual project confirmation. Escape cancels the draft. If the camera looks straight along the normal, use `E` for exact input.

CAD IR stores the semantic face and signed distance in `extrudeFace`. Rust passes them to OCCT's `BRepFeat_MakePrism`. One command batch adds the feature and sets the body source. An unavailable face, an empty result or the wrong volume change leaves the previous revision intact. The resulting solid has no v1 semantic catalog for downstream face or edge picks. Kernel tests cover both signs, invalid inputs, dimensions, volume and STEP round-trip.

### Selected-edge fillet and chamfer (M3)

Press `3` to switch to edge selection. Click an edge, then Shift-click to add or remove edges on the same body. Press `Ctrl+B` and choose Fillet or Chamfer. The dialog shows the selected edge count. Drag the slider or enter a positive radius or setback in millimetres. The OCCT preview rebuilds in a separate view. Choose Build and confirm the project edit to commit, or press Escape to discard the draft.

CAD IR stores the source body feature, first semantic edge reference, optional additional references and the common radius or setback in `filletReferencedEdge` or `chamferReferencedEdge`. Existing single-edge revisions still load. The worker resolves every edge against the original BREP and builds the contours together. The host updates the body in one transaction and rolls back the command batch on invalid input. The tool accepts up to 64 resolved edges, including box edges and circular cylinder rims. The result has no v1 semantic catalogue for subsequent edge blends. Tests check cylinder volume, BREP validity, STEP round-trip, rejected inputs and rollback.

### Exact topology selection

Pick a face and press `L` to select its owning body. Pick a face or edge and press `Shift+G` to select similar elements on the same body. Alt-click an edge to select its tangent chain. The corresponding controls appear in the 3D toolbar after a pick. The original pick stays active for tools that accept one element.

The native worker reconstructs the committed BREP. Similarity compares geometry type, area or length and available analytic radii. The dimensional tolerance is the larger of 0.01% and 0.000001 in the relevant units. A tangent chain follows a shared vertex with one continuation within 0.001 radians. The result applies to the same visible committed scene. Selection creates no revision. The worker caps a body at 20,000 candidates and a result at 1,024 items.

### Shell (M3)

In face selection mode (`2`), select a face with a semantic reference and choose **Tools → Shell**. The selected face is checked as the first opening. Check any additional openings on the same body and leave at least one face in place. Enter a positive wall thickness. Walls go inward by default. Enable **Build walls outward** to grow the outside dimensions. Choose Build and confirm the edit.

The `shell` operation stores a solid source, a signed wall offset and unique semantic face references for the openings. OCCT builds the walls with `BRepOffsetAPI_MakeThickSolid`. A stale reference, invalid result or inward wall that leaves no cavity rolls back the transaction. Tests cover inward and outward walls, multiple openings, dimensions, volume, BREP and STEP round-trip.

### Linear and circular patterns (M3)

Select a body in the 3D view and choose **Tools → Pattern**. For a linear pattern, choose the first CAD axis and signed spacing, then optionally add rows along a different axis. For a circular pattern, choose the CAD axis, set its origin in model coordinates and enter a signed angular step. **Repeat the hole only** is available when the body's active feature is a hole. Build and confirm the project edit to save the pattern as one operation.

`patternLinear` repeats a body along one step vector or on a grid with two independent vectors. `patternCircular` repeats a body around a model-space axis with a signed angular step. The count includes the source and is limited to 128 instances. Body copies stay as independent solids in a compound. When the current source is an active hole feature, the same commands can repeat its cutter and cut the stock once. A hole pattern that removes no additional material fails without saving the edit.

### Duplicate body (M3)

Select a body and press `Shift+D` or choose **Tools → Duplicate body**. Enter the name and offset in model coordinates. The initial X offset uses the selected body's width. Confirm the edit to save the new body and its history together.

`duplicate_body` copies the required features and bound parameters, gives them new identifiers, and rebinds semantic topology references to the copy. STEP assets stay shared by their immutable content hashes. Editing a copied parameter affects only the copy. Existing feature and body IDs stay the same. The transaction rejects ID collisions. Host tests verify volumes and extents after exporting and reopening the committed STEP.

### Separate solids (M3)

Select a compound body and press `P` or choose **Tools → Separate solids**. Confirm the edit to get independently selectable bodies in one revision. The first solid retains the original body ID. This operation uses the existing solids in the BREP compound and keeps their placements. It accepts 2 to 128 solids. An older preview without a solid count must be regenerated before the tool becomes available.

`separate_solids` creates an `extractSolid` feature for each output. The worker checks the original count on every rebuild. A count mismatch fails the transaction. For changes that reorder the source solids, recreate the separation to keep the intended parts.

### Boolean operation with a retained tool

With at least two visible, unlocked bodies in a saved native model, choose **Tools → Boolean operation**. Select the target, the separate tool body and **Boolean union**, **Boolean cut** or **Boolean intersection**. Review the OCCT preview and click **Build** to save the operation. The target changes and keeps its body ID. The tool keeps its original ID, geometry and history and can be selected separately.

The Command API adds a two-input Boolean feature and changes only the target body's output in a single transaction. OCCT builds in non-destructive mode. Empty or invalid results fail without a new revision. A full revision export includes both bodies. A retained tool can overlap the result, especially after union or intersection. Measure each body separately when comparing volumes. Tests verify volumes, BREP validity, STEP round-trip and transaction rollback.

### Body groups (M3)

Use **Body groups** in the model tree to create a named group from the selection. The group button selects its visible and unlocked members. Use the adjacent controls to add selected bodies, remove members, or delete the group. Deleting the group keeps its bodies.

CAD IR stores groups as optional lists of body IDs in the saved revision. `group_create`, `group_add` and `group_remove` update membership through the Command API and participate in Undo. A body can belong to multiple groups. Older documents open with no groups. The STEP geometry retains the same solids and dimensions.

Current schema operations:

| Group | Operations | Result |
| --- | --- | --- |
| 2D basis | `sketch2d`, `rectangle`, `circle` | A profile for extrusion; the interactive sketch currently supports one polygon loop. |
| Solids | `sphere`, `cone`, `torus`, `extrude` | A solid; extrusion refers to an earlier profile. |
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
