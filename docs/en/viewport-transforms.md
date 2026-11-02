# Viewport and modal transforms (M2)

[Documentation](index.md) · [Русский](../ru/viewport-transforms.md)

## Purpose

M2 brings Blender-style 3D viewport workflows to FormaCAD: G/R/S move, rotate and scale with exact typed values, F9 adjustment of the last operation, multi-selection, hiding and isolation, standard views, pie menus and snapping. The behavior is an independent reimplementation from a description, without Blender code, keymap files or assets (see the [parity note](../parity/blender/viewport-transforms.md)). Every geometry change goes through the regular Command API: expected-revision check, `modify_project` permission, the CAD queue, an OCCT rebuild in the isolated worker and an atomic save of a new revision.

## User actions

Shortcuts work while the 3D view area has focus. Commands use physical keys (`KeyboardEvent.code`), so they behave the same with Russian and English layouts. Text fields and dialogs keep their own input.

### Move, rotate, scale

1. Click a body. Shift+click adds or removes a body; the last marked body is active.
2. Press **G** (move), **R** (rotate) or **S** (scale). A status line at the bottom shows the operation, constraint and current value; the camera does not orbit during the operation.
3. Move the mouse: translation follows the screen plane through the pivot, rotation the angle around the pivot, scale the distance ratio to it.
4. Refine:
   - **X/Y/Z** constrains to an axis. Repeating cycles global axis, body-local axis and no constraint; **Shift+axis** keeps the plane without that axis.
   - Typing sets an exact value: `10`, `10mm`, `2,5см`, `1 m`, `1in`, `45`, `0.5rad`, `150%`. Minus toggles the sign, Backspace deletes. Before a value is typed, G/R/S switch the operation.
   - **Shift** gives precise motion (×0.1), **Ctrl** steps of 1 mm / 5° / 0.1 (it inverts the increment snapping setting).
5. **Enter**, **Space** or **LMB** confirm; **Esc** or **RMB** cancel. Cancel restores the bodies and creates no revision. While the operation is active, global shortcuts (including Ctrl+Z) do not run.

Confirmation creates one revision with a `transform` operation for each selected body. One **Ctrl+Z** undoes it completely.

### F9: last operation parameters

**F9** opens a panel at the bottom left of the view. For the last transform made in this session it shows the X/Y/Z offset in mm, the rotation angle and axis, or the scale factor and axis. **Apply** recomputes the operation from the original position: replacing 10 mm by 20 mm gives 20 mm, not 30 mm. The adjustment replaces the final logical Undo step, so one Ctrl+Z still returns to the state before the original transform. If the project changed after the operation (another edit, Undo/Redo, another revision), the panel reports that it cannot be adjusted.

### Selection and visibility

- **A** selects all visible, unlocked bodies, **Alt+A** clears, **Ctrl+I** inverts.
- **B** box: drag a rectangle to add bodies; with **Shift** bodies are removed. Esc or RMB cancel.
- **1/2/3** selection filter: bodies, CAD faces, CAD edges (faces and edges need a native model).
- **H** hides selected, **Shift+H** hides others, **Alt+H** shows all, **Numpad /** temporarily isolates the selection; repeating leaves isolation and restores earlier hiding. Hiding is shared with the Outliner eye.

### Views and navigation

- **Numpad 1/3/7** front, right, top; with **Ctrl** back, left, bottom. Pressing again restores the view after orbiting.
- **Numpad 5** perspective/orthographic, **Numpad .** frame selected, **Home** frame all.
- **T/N** toolbar and sidebar, **Alt+Z** X-ray.
- Settings → Input can emulate the numpad with number keys and choose mouse navigation: Forma (LMB orbit, RMB pan) or Blender-compatible (middle button orbit, Shift+middle pan).

### Pie menus, pivot and 3D cursor

- **Z** shading (solid with edges, solid, wireframe, transparent, X-ray, ghost).
- **.** (period) pivot point: bounding box center, median point, 3D cursor, active element, individual origins. With numpad emulation `.` acts as Numpad . (frame selected), and this menu is not available from the keyboard.
- **Shift+S** snapping: increment snapping, exact surface snapping for the cursor, cursor to the selected edge midpoint, to the selected circle center, to the world origin.
- **Shift+RMB** places the 3D cursor on a surface. With exact snapping enabled the tessellation hit is only the ray hint: OCCT computes the final face intersection on the committed model.

Menu items are chosen by click or number; Esc closes the menu.

## Technical route

- `features/keymap/` holds the command registry, remap validation and the single route (`useViewportKeymap`). The separate T/N/Numpad handler from M1 is replaced by it.
- `features/transform/` is the pure session model (pivots, constraints, input, matrices). `features/transform/viewport/` is the renderer adapter: `renderBasis` converts CAD Z-up mm to the Y-up scene (`(x, y, z) → (x, z, −y)`, as the native preview writes it) and back; `sceneTargets` builds targets from CAD body ids (mesh names) and changes preview matrices only; `ModalTransform` owns pointer and keys during the operation; `commitTransform` builds `transform_body` commands and calls `apply_ir_commands` with the expected revision.
- `features/adjust-last/` is F9: `last_transform` reads the durable history head, `amend_last_transform` replaces matrices by command index; the panel rebuilds the original session instead of accumulating values.
- `features/selection/` contains the selection model and box select (`projectBodies` projects body triangles with the same camera frame; keys are CAD body ids, not triangle numbers). Multi-selection lives in `stores/workspace.ts` (`selectedBodies`, with `selected` as the active body).
- `features/visibility/useBodyVisibility.ts` implements hiding and isolation over `visibilityModel`.
- `features/pie-menus/` holds pie menus; `features/snapping/` increment snapping, the `snapApi` adapter and the 3D cursor.
- Host: `model_snap::snap_model_reference` captures the committed revision like exact measurements (document, `source.step`, sealed imported STEP assets), runs the worker `snap_query` operation in a temporary `.transient/reference-measurements` folder, verifies the response (bounds, checksum, request identity, revision, body, all seals, query echo, finite point and unit normal) and rechecks that the project did not change. The project and its log are not modified. The client additionally checks project, revision, body and reference.
- C++: `cad-core/src/affine_transform.cpp` applies a bounded 3×4 matrix to the BREP with determinant and semantic-reference checks; `cad-core/src/snap_reference.cpp` computes arc-length edge midpoints, analytic circle centres and ray intersections with trimmed faces with outward normals.

## Data storage

- Geometry: each confirmed transform is a new immutable revision with a CAD IR `transform` operation; F9 appends a physical revision and replaces the final logical Undo slot. No SQL schema change.
- Interface profile (`forma.ui.*`): pivot (`transform.pivot`), increment snapping (`transform.increment`), exact cursor snapping (`snap.exact`), mouse navigation (`navigation.mousePreset`), plus the selected and hidden bodies per project as before.
- Not stored: temporary isolation, the 3D cursor position and F9 state (after a restart F9 is unavailable for an older operation).

## Limitations

- Whole bodies are transformed. Face/edge/vertex edit mode and vertex mapping are not implemented: C++ has no vertex semantics yet, so there is no vertex snapping either.
- Orientation is global. Because transforms are baked into the BREP, a body's local frame equals the global frame; "local axis" differs only in its label.
- Exact snapping covers the existing semantic catalogue (rectangular and cylindrical construction and reference-preserving transforms). For imported STEP without semantic references the cursor uses the tessellation hit, i.e. it is approximate. During G itself there is no exact geometric snapping, only increments; exact snapping places the 3D cursor used as a pivot.
- Box select also picks occluded bodies (like "select through"); circle and lasso selection are not connected.
- The ruler/protractor still measures surface points; connecting it to exact snapping is the next step.
- F9 is available only for the last transform of the current session and only when every selected body received an operation (the host skips identity matrices).

## Verification

- Rust: `snap_worker` (2), `transform_worker` (2), `snap_host` (2: face point `(3, 4, 10)` with normal `+Z` and edge midpoint `(0, −10, 10)` on the committed model with the project unchanged; rejection of a stale revision, unknown body and zero ray).
- TS: CAD↔render basis (Y/Z moves and rotation about an offset pivot), pointer sampling, modal keys with a Russian layout, increment snapping, F9 fields (10 → 20 mm without accumulation), box projection, exact snap adapter.
- Playwright `e2e/viewport-transforms.spec.ts`: the real interface with the real OCCT worker rebuilding every revision (the host IPC is emulated). It checks Esc without a revision, a denied permission, G X 10 Enter with a Russian layout (new geometry moved by 10 mm), F9 10 → 20 mm, one Ctrl+Z, rotation in light and dark themes, a pie menu, H/Alt+H and the preview frame rate (≈60 FPS with software WebGL). Run with `FORMA_E2E_WORKER=<path to forma-cad-worker.exe>` and `FORMA_OCCT_ROOT=<OCCT build>`; without a worker the test is skipped.
- Native previews are shown in CAD coordinates (the loader no longer centres them) so a committed move stays visible; non-native files are centred as before.
