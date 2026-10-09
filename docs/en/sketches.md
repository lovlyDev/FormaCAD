# 2D sketches
[Documentation](index.md) · [Русский](../ru/sketches.md) · [Architecture](architecture.md)

I use a typed `sketch2d` feature in the native Windows CAD IR v2 workflow. A sketch stores a workplane (`xy`, `xz`, or `yz`), a 3D plane origin, points and lines with stable IDs, and geometric constraints. An outer polygon with optional inner holes can be extruded into an exact BREP solid. The regular CadQuery compatibility build does not execute this feature.

Open the model source editor in a project and choose **Create 2D sketch** to start with a rectangular profile. Expand **Edit 2D sketch** in the feature tree to move points, edit coordinates and plane origin, add horizontal/vertical/length constraints, or remove constraints. Build the model to solve the constraints and save a new revision. The SVG shows draft point positions; the saved 3D model is the validated solved result.

The first solver supports fixed points, horizontal and vertical lines, line lengths (literal or linked to a named parameter), and coincident points. Each sketch has 3–32 points, up to eight closed non-intersecting line loops, and up to 64 constraints. Conflicting constraints, zero-length edges and invalid outlines fail before a revision is saved. Curves, arcs, and general external geometry references are future work.

The [CAD IR types](../../apps/desktop/src-tauri/src/cad_ir/mod.rs) define the feature and typed commands. The bounded [solver](../../apps/desktop/src-tauri/src/cad_ir/sketch.rs) computes local coordinates; the [native executor](../../apps/desktop/src-tauri/src/native/document.rs) sends the outline to the [OpenCascade prism builder](../../cad-core/src/polygon_prism.cpp). Geometry is kept out of the UI state. This separation follows the model/entity/constraint idea documented by [SolveSpace](https://github.com/solvespace/solvespace/blob/master/exposed/DOC.txt); the solver and CAD integration here are independent implementations.

Construction lines and their constraints have a [dedicated guide](sketch-construction.md). Total limits are 32 points, 64 lines and 64 constraints; only profile lines form the outer boundary and holes.

Local 1.2.6: [contours and holes](sketch-profiles.md), [point constraints and profile tools](sketch-constraints.md), [parameters and dimension binding](cad-parameters.md), [diagnostics and solved preview](sketch-diagnostics.md).

## Smooth dragging

The gesture lives in [interaction](../../apps/desktop/src/components/sketch/interaction/useSketchDrag.ts). Pointer-down snapshots the screen transform, point coordinates, and grab offset. Movement updates one local point at most once per frame; CAD source, constraint analysis, SQLite, and 3D preview are not recomputed on each mousemove. Release commits one draft change. Escape and lost capture cancel; losing window focus commits. SVG framing and point radius stay unchanged throughout and after the gesture. “Fit sketch to view” explicitly reframes and stores the frame per project/sketch. Use it after numerical edits outside the frame. Solved preview remains read-only until coordinates are applied; constraints are still checked when building.

Verify slow and fast movement, leaving and returning to the SVG, release continuity, Escape cancellation, both themes and languages. There must be no intermediate CAD-source commits or new revisions.

## Editor and shape consistency

Every disclosure opening runs a new height/opacity animation, including repeated opens and restored open sections. Content stays mounted to preserve drafts and state. Reduced motion disables animation; interacting finishes it so gestures cannot depend on an intermediate height. Shared implementation: [AnimatedDetails](../../apps/desktop/src/features/model-editor/AnimatedDetails.tsx).

Construction geometry, point coincidence, and contours use separate responsive cards with aligned fields/actions and Lucide icons. Model editor lists use the shared Radix Select with keyboard support, themed menus, and safe empty-value mapping for numeric dimensions. Deleting a selected point falls back to an existing tool choice. Checkboxes use the shared Checkbox.

New starter sketches are unconstrained so dragging does not silently restore a rectangle. Existing constraints are preserved. When solving changes drawn coordinates, a dashed outline shows the actual constrained contour. Before building active sketches, explicitly accept solved coordinates or keep the drawn profile and remove that sketch’s constraints. Building remains blocked until resolved; read-only analysis discards stale responses. Conflicts/errors also block silent builds. 3D preview can show the constrained shape for comparison. Sketch users are listed: one profile is distinct from the complete model with other profiles and unions.

Verify three disclosure cycles, restored state, reduced motion, keyboard point/numeric-source selection, deleted point choices, and both resolution actions after dragging a constrained rectangle. Repeat both languages/themes. [Build review](../../apps/desktop/src/features/model-editor/sketch-review/useSketchBuildReview.ts), [comparison](../../apps/desktop/src/features/model-editor/sketch-review/shapeDifference.ts).


## Explicit coordinate links

Add a named parameter in Model editor, open Edit 2D sketch and Coordinate parameter links. Select a point or sketch origin, axis and parameter. The coordinate is `valueMm × scale + offsetMm`. Adding a link computes an offset that preserves the current shape; then edit factor and offset. Edit linked axes through their parameters; linked points cannot be dragged. Unlinking materializes the current effective coordinates.

The empty editor offers Create linked base and triangle: shared width/depth, base height and triangle height drive both profiles, the upper profile origin and extrusion length. Independent sketches are never linked implicitly. Reproducible example: [linked-profiles.cad.json](../fixtures/linked-profiles.cad.json).

CAD IR v2 stores optional `operation.bindings` entries: `id`, `target: point | origin`, `axis`, `pointId` for points, and `value: {kind: parameter, parameterId, scale, offsetMm}`. Point axes are x/y, origin axes x/y/z. SQLite revision sources and CADPACK preserve links; STEP/mesh formats do not carry parameter history. Drafts use existing preferences storage and create no revision until Build.

Rust `cad_ir/sketch/bindings.rs` resolves coordinates before validation/execution and inserts fixed-axis equations into the shared diagnostics/BREP solver. Duplicate targets, missing references and nonfinite/out-of-range ±10000 mm coordinates are rejected; a sketch permits at most 64 combined links/constraints. Direct mutation commands reject linked coordinates with PARAMETER_BOUND. Removing points prunes their links; splitting an edge creates an independent vertex. Conflicting geometric constraints require an explicit fix and never save partial geometry. Verification changes width/depth and both heights, yielding 50400 mm³ instead of 12000 mm³; both themes/locales and unlink-without-motion are checked.


## Disclosure motion and tool alignment

Every editor section, including CAD source, uses `AnimatedDetails`: click/keyboard update the saved state while mounted contents animate height/opacity for 260 ms before hiding. Opening and closing animate every time; rapid reversal starts from the current height. Reduced motion disables movement. Pointer gestures finish animation to keep sketch framing stable. Existing SQLite preference storage is unchanged.

Sketch disclosure aligns with the feature name/ID and uses Lucide icons. Tool labels override the modal bottom margin: construction, coincidence and coordinate-link buttons align with adjacent control bottoms. Narrow layouts wrap controls without overlap. Validation measures intermediate heights for repeated open/close, reversal, both themes/locales and field/button rectangles.

Sketch tool dropdowns and action buttons share a 32 px height. The scoped rule overrides the shared Select `min-height: 38px`; verification compares both top and bottom edges.

[Typed AI edits and link commands ](command-api.md): revision-bound batches and explicit coordinate unlinking without movement.
