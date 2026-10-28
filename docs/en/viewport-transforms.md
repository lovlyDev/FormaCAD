# Viewport and transforms: M2 preparation

[Documentation](index.md) · [Русский](../ru/viewport-transforms.md)

## Status

This describes independent M2 modules, not a completed user workflow. Their development is authorized before the M1 merge. Shell integration follows a rebase onto `main` containing M1. The application version stays unchanged. The registry installs no listeners, previews do not edit a CAD document, and the snap contract does not call a worker by itself.

## Controls and input

`features/keymap/` contains an independently authored command registry and physical `KeyboardEvent.code` routing. G/R/S and axis keys therefore retain their assignments with a Russian keyboard layout. Modal ownership precedes global commands. Text fields, dialogs, IME, AltGraph and consumed events retain their input. A command is dispatched only if the caller declares it available. Remaps are validated as a complete set: known commands, bounded physical codes and no conflicts within a context. Numpad emulation takes priority over digit selection filters. Mouse presets are separate: Forma uses left-drag orbit/right-drag pan; the Blender-compatible preset uses middle/Shift-middle. Camera integration is pending.

Numeric input supports mm, cm, m, inches, Russian decimal commas, degrees/radians and dimensionless scale/percentages. Results use mm, degrees and factors. Empty or incomplete input differs from zero; invalid units and excessive values cannot be confirmed. `=` expressions are rejected until the later formula milestone. No `eval` is used.

## Preview and confirmation

`features/transform/` separates frames, pivots, constraints, preview matrices and the interaction state machine. Calculations support combined bounds, mean origins, the active origin, cursor and individual origins. Frames must be right-handed and orthonormal; a zero normal never silently becomes a global frame. Repeated axis input cycles the chosen orientation, body-local frame and no constraint. Shift-axis excludes that direction for translation and scale. Shift precision is integrated per pointer segment to prevent jumps when toggled.

A session captures project, revision, scene token and copies of original matrices in canonical CAD coordinates measured in mm. Existing Y-up render space stays unchanged; the adapter must convert its display basis before supplying matrices. Pure poses for seven standard views live in `features/viewer/view-commands/`; top/bottom views have a nonzero orthogonal up vector. G X `10mm` emits a translation intent of `[10, 0, 0]`. This does not yet prove a BREP translation. Confirmation emits a batch only once; cancellation and new no-op changes emit none. A changed project, revision or scene cancels stale interaction. Persistence must go through a separate Command API adapter, expected-revision checks, permissions, queue, native rebuild and atomic save. The renderer adapter, F9 `amend_last` and one-step Ctrl+Z verification are not yet connected. `features/adjust-last/` prepares argument replacement relative to the original snapshot: 10 → 20 mm does not accumulate to 30 mm, and replacing by 0 mm restores the original position. This is currently only a pure model checking the last revision, history generation and transaction; the backend does not automatically accept such a plan.

`cad-core/src/affine_transform.cpp` prepares a separate native primitive: a bounded 3×4 matrix copies the source BREP, checks nondegeneracy, a positive determinant, the OCCT result and semantic-reference preservation. Reflection is not silently treated as negative scale. Invalid matrices or lost topology provenance fail explicitly. C++ checks geometry independently from visual matrices. The CAD IR `TransformBody` command adds a `transform` operation and updates the body source in a staged document copy. Existing `apply_ir_commands` retains revision/permission checks and atomic application; the new worker executes the operation through this native primitive. A failing second command leaves the original document unchanged. The modal renderer adapter is not yet connected.

## Selection, visibility and snapping

`features/selection/` holds semantic ID membership and the active item. Add, toggle, subtract, select-all and invert respect the filter, visibility and selection lock. Scene tokens prevent selection from migrating onto new mesh indices. Box/circle/lasso mathematics intersects projected points, segments and triangles, mapping hits back to semantic BREP IDs rather than triangle ordinals. A large face is also selected when it contains the entire region. Projection, camera clipping and occlusion must come from the renderer for the captured scene; the module does not fabricate them.

`features/visibility/` separates normal hidden bodies from temporary isolation. Leaving isolation restores earlier hidden bodies while retaining explicit visibility changes made during isolation. Empty selection never executes hide-others. Invalid persisted values and missing bodies are pruned. No BREP, revision or CAD permissions change.

`features/snapping/` separates mathematical grid/increment constraints from an exact OCCT response contract. The reader checks request/project/revision/body/scene identity, document/source hashes, reference kind, finite coordinates and unit normals. Late or foreign results are rejected. The current contract covers edge midpoints, circle centres and faces; worker requests and semantic vertices are not yet implemented. Ray intersections with tessellation are not declared exact BREP snapping.

## Data, limitations and validation

Pure TS models write no project, profile, SQL or user files; the CAD IR command uses the existing host route with permissions and rebuilding. Storage adapters, F9 UI, pie menus, viewport tools, native queries and shell integration remain necessary for M2 acceptance. Pure-function tests cannot prove native G X 10mm, Esc without a revision, one-step Ctrl+Z, F9 or 50 FPS.

Focused TS tests cover routing/conflicts, dimensions, lifecycle, immutable snapshots, offset-pivot matrices, region geometry, visibility and stale snap responses. The C++ test checks determinant-based volume, bounds, exact named-edge length, immutable source, independent `BRepCheck_Analyzer`, STEP round-trip and invalid matrices. Actual outcomes and pending end-to-end checks are recorded in Issue/PR. Native builds/tests use parallelism 2; pre-PR checks follow the individual commands in specification §9.1.

The prepared route passed 16 CTest checks and 13 native worker tests (11 existing and 2 new). The new test builds original and moved bodies in separate worker runs, then rebuilds the referenced plane from sealed document/STEP inputs. The X-coordinate difference is 10 mm within 1e-7. Invalid matrices leave no STEP/GLB. This verifies command/worker behavior, not the keyboard workflow in the UI.
