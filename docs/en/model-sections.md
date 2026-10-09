# Exact model sections

Exact sections are available in the main 3D viewport through the scissors “Section view” button. The controls, graphical clipping and result lines are integrated into the scene. The backend intersects the committed exact BREP with a plane, reading saved STEP in an isolated worker and returning curve fragments with natively integrated lengths. Sections are view state: they do not cut the saved model, change its feature tree, create a revision or modify exported geometry. This is not a completed drawing or section-area tool.

## Using the viewport controls

1. Open a project with a saved STEP/STP revision and wait for the model. The button is available in a native desktop build when the current source has a SHA-256 digest. Mesh-only models, browser mode and comparison views show a disabled button with an explanation.
2. Click the scissors in the viewport toolbar. Opening the panel also enables sectioning. Under “Section source”, choose “Numeric or free plane” for XY, XZ, YZ or “Free plane” using the project's custom Select. The other source provides [sections from a selected planar CAD face](face-sections.md).
3. Enter the offset in millimeters. It is measured from the CAD origin along the normalized normal, rather than from a selected face. The first XY offset is the loaded model's bounding-box center Z; changing the axis retains the current offset. Adjust it when the plane does not intersect the model.
4. For a free plane, enter the unitless X/Y/Z normal components. A zero vector shows a warning; valid normals are normalized for calculation and clipping. Selected planar faces now have their own source mode; dragging a plane manipulator and separate angle inputs remain unavailable.
5. In numeric/free mode graphical clipping updates immediately. After a 400 ms pause, exact intersection is requested automatically; the panel shows pending/error state or total exact length in millimeters. This mode has no separate calculation button. Selected-face mode provides “Calculate section” and its own automatic-update checkbox; clipping there waits for a verified current report. A new request waits for any in-flight request to finish, then computes the latest plane.
6. Uncheck “Clip model with section plane” to remove clipping and result lines. Closing the panel alone keeps sectioning enabled. Reopening it with the scissors enables it again. An empty intersection has its own message and is not a geometry error.

`Viewer` → `useSectionView` → `SectionControls` / `SectionScene` live in `apps/desktop/src/features/viewer/sections`. The hook binds each request to project identity, current revision, source SHA and every plane parameter. `sectionApi` validates the response shape and SHA; results are published only for the current request key. Changing plane/source/project/revision hides old exact curves and length immediately until the replacement arrives. Disabling sectioning or unmounting cannot publish a late response. An already running stale worker is not automatically cancelled: the hook waits for it and discards obsolete data. Explicit cancellation is available through the CAD queue.

## Preferences, scene and appearance

For the main view, `usePersistentState` stores `forma.ui.project.<projectId>.view.main.section.open`, `.enabled`, `.axis`, `.offsetMm` and `.normal`. Changes first update localStorage; the shared `flushPreferences` sends `set_ui_preferences` after 800 ms to SQLite's `settings` table, `ui_preferences` record. Startup fills missing local keys using `get_ui_preferences`. These are machine view preferences, not CAD operations or revisions. A read-only project can save section preferences without modifying its payload or folder. Exact reports are not persisted and are recomputed after reopening with sectioning enabled.

The shared panel also stores `.mode`, `.faceOffsetMm` and `.faceAutomatic`; [selected-face sections](face-sections.md) describe their units, explicit calculation and verified-only clipping. A selected reference or report is not persisted as a model pin. Automatic recomputation on reopening applies to enabled numeric mode; face mode needs a new valid selection and its enabled automatic setting or explicit calculation.

`SectionScene` temporarily enables renderer local clipping, applies a plane to model materials and filters raycasts in the hidden half-space. CAD edges use the same half-space through `sectionClipping`. Disabling or changing the scene restores the previous material clipping planes, raycasts and renderer setting. Sampled report points become separate line segments; old BufferGeometry is disposed. The overlay is drawn over the model and does not intercept selection raycasts. No section cap is generated: this clips the original scene rather than creating a closed cut solid.

The panel reuses `EditorDisclosure`, the custom Checkbox/Select and Lucide Scissors/LoaderCircle icons. Aligned controls are 32 px high; surfaces/text follow the theme and section lines use suitable purple colors in light/dark mode. Panel entry is 200 ms, exit is 140 ms, and disclosure/chevron movement uses the editor's shared 260 ms animation. Retained content completes exit on repeated use. Global reduced-motion settings shorten movement. These animations apply to the panel; plane/clipping updates do not interpolate the CAD geometry.

## Plane and result contract

`section_model` accepts `projectId`, `expectedRevision`, `originMm`, `normal` and optional `deflectionMm` (default 0.01 mm). XY/XZ/YZ are normals `[0,0,1]`, `[0,1,0]`, `[1,0,0]`. A free plane uses any bounded nonzero normal; its origin is in CAD world millimeters. The backend normalizes the direction. Changing a plane offset or angle defines another request against the same saved revision.

```json
{"sourceSha256":"<saved source digest>","geometry":{"plane":{"originMm":[0,0,5],"normal":[0,0,1],"deflectionMm":0.01},"curves":[{"id":"section-edge-1","lengthMm":40,"pointsMm":[[-20,-10,5],[20,-10,5]],"closed":false}],"totalLengthMm":120}}
```

Curve IDs are temporary identifiers for one source/plane result. Fragments are not silently joined into contours, assigned persistent topology, interpreted as outer/inner boundaries or used to infer section area. An intersection can be empty and still valid: `curves: []`, `totalLengthMm: 0`. A tangent point without a positive-length edge also produces no curve. The exact BREP source determines the intersection and native length; `pointsMm` are approximate display samples. Their chord lengths must not replace the reported integrated `lengthMm`.

## Execution and storage

The host validates the requested plane and current revision before queuing. It resolves that revision's exact source attachment and SHA, waits for the application-wide CAD permit, checks the revision again, and stages only checksum-verified source bytes in an app-level `.transient/sections` workspace. Read-only projects can be inspected because the workspace is outside the project folder. Required permissions for model edits are not consumed by this read-only operation.

The worker uses the existing content-addressed input resolver and independently checks the STEP size/hash. OCCT intersects the BREP with an infinite plane; dedicated C++ `SectionResult` holds curves separately from volume-bearing `Shape`, allowing a valid empty intersection. Worker protocol operation `section_step` writes only temporary `section.json` and the response envelope. The host validates request correlation, report checksum and source identity, then checks the current revision again. A stale result or cancelled request is rejected. A scoped guard removes the entire temporary workspace and releases the CAD queue on success, failure and cancellation. The original STEP, database project payload and revision history remain unchanged.

The worker timeout is 120 seconds with the existing process cancellation/termination route. A cancellation token already set before execution prevents launching the worker. The global CAD queue can cancel a queued or running section by task ID/project without granting a model write.

## Bounds and display approximation

Plane origins are finite and within ±10,000 mm. Normal components are finite and within ±1,000,000 with vector length at least 10⁻⁹. The requested display deflection is 0.001–1 mm. There are at most 4,096 intersection edges and 100,000 display points, with an 8 MB report limit and a 40 MB STEP input limit.

Display sampling uses bounded adaptive subdivision at quarter/midpoint samples over C1 curve intervals, with a depth limit of 24 and explicit point/interval budgets. It is a visual approximation, not a mathematically certified maximum deviation for every possible imported curve. Budget exhaustion returns an error instead of truncated lines. Exact lengths are computed from BREP curves independently of display density.

Coordinates are CAD millimeters. The application viewer currently scales imported GLTF meters back to millimeters, so lines outside the imported model group use `[x,z,-y]`; normals use `[nx,nz,-ny]`. Raw GLTF coordinates use an additional division by 1,000. Mixing those two coordinate spaces produces displaced or undersized section overlays.

Errors use `INVALID_SECTION_PLANE`, `SECTION_FAILED`, `SECTION_LIMIT`, `SECTION_STALE`, `SECTION_CANCELLED`, `SECTION_UNAVAILABLE` and existing asset-integrity codes. Their display is localized while technical details remain separate. The backend does not persist planes as model operations; the UI persists view preferences in SQLite as described above.

## Verification and limitations

`tests/model_sections.rs` checks analytic box perimeter in XY/XZ/YZ and an oblique plane, hollow-cylinder outer/inner curve lengths, different display densities, valid empty intersections, invalid planes, source-checksum tampering, cancellation and absence of STEP/GLB output. `tests/model_section_projects.rs` proves saved database payload/source bytes/project folder are unchanged, temporary workspaces are removed, and a head change while queued rejects before worker publication. `cad-core/tests/section_test.cpp` separately validates native plane guards and analytic intersections. No installed WebView2, cross-platform section UI or completed section drawing export is claimed by these backend tests.

See [viewer](viewer-and-files.md), [imported STEP](imported-step-features.md), [project storage](project-storage-v2.md), and [architecture](architecture.md).
Frontend `sectionPlane.test.ts` verifies millimeter coordinate mapping and hidden/crossing CAD-edge clipping. Integrated controls and shared motion do not replace separate installed-WebView2 and real-clipping checks in both themes. Exact intersection covers the complete saved STEP; filtering by selected/hidden body is not implemented. See [CAD queue](cad-task-queue.md).
