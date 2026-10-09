# 3D viewer, measurements, and files

[Documentation](index.md) · [Русский](../ru/viewer-and-files.md) · [Using Forma](using-forma.md) · [Data](data.md)

The viewer displays the current revision's saved geometry. Drag to orbit, right-drag to pan, and use the wheel to zoom. Reset/fit and isometric, top, front, and side views are available. Orthographic and perspective modes keep separate camera states. The workspace also controls the grid, rendering style (solid and edges, solid, wireframe, transparent), and body visibility. Camera and display settings are restored per project after restart. See [Viewer](../../apps/desktop/src/features/viewer/Viewer.tsx), [camera](../../apps/desktop/src/features/viewer/CameraControl.tsx), and [render modes](../../apps/desktop/src/features/viewer/renderMode.ts).

## X-Ray and Ghost

The display menu offers X-Ray and Ghost. X-Ray makes surfaces translucent and renders outlines without depth testing, revealing a separate body inside a closed enclosing body. Hidden contours are deliberately visible rather than physically transparent. Ghost keeps the selected body opaque and dims other bodies. Selecting another body moves the emphasis immediately; without a selection the entire model is dimmed. Hidden bodies remain hidden. Ordinary CAD face/edge selection rules still apply: displaying a hidden outline in X-Ray does not turn it into an available topology reference.

The mode uses the existing project display preference. Theme changes update outlines and selection colors without resetting the camera. The [appearance module](../../apps/desktop/src/features/viewer/appearance/bodyAppearance.ts) owns temporary material clones; [outlines](../../apps/desktop/src/features/viewer/appearance/bodyOutlines.ts) release their geometry and materials when switching modes. Original materials, CAD IR, STEP and revisions remain unchanged. Mesh export replaces display materials with originals and removes decorative outlines, so transparency, X-Ray and Ghost are not saved into GLB. Sections continue clipping surfaces and outlines in every mode.

Verify with a multi-body model containing an enclosed body: compare solid and X-Ray, select each body in Ghost, switch themes and return to ordinary rendering. Unit tests check original material references and single resource disposal. Chromium uses a genuine GLB from the native worker and checks inner geometry, selection, themes and export. See [unit tests](../../apps/desktop/src/features/viewer/appearance/bodyAppearance.test.ts) and [browser verification](../../apps/desktop/e2e/render-modes.spec.ts).

## Selection and measurement

Bodies can be selected. Native CAD previews additionally expose faces and edges and any exact properties supplied by the B-Rep. Mesh imports do not have those exact topology properties. Measurement uses two points for distance and three for angle, radius, or diameter; accuracy depends on the geometry and how points are selected. Rebuilding can change topology, so an arbitrary face or edge selection is not yet a permanent CAD reference. See [face selection](../../apps/desktop/src/features/viewer/faceSelection.ts), [edge selection](../../apps/desktop/src/features/viewer/edgeSelection.ts), and [measurement](../../apps/desktop/src/lib/measurement.ts).

If a model carries motion data, the panel offers play, pause, and stop. The viewer does not turn a static mesh into a parametric kinematic assembly. Frame capture and project thumbnails use the rendered model; without a suitable preview, a card shows a neutral icon. See [playback](../../apps/desktop/src/features/viewer/Playback.tsx), [capture](../../apps/desktop/src/features/viewer/Capture.tsx), and [thumbnail](../../apps/desktop/src/lib/thumbnail.ts).

## Import

| Input | Handling |
| --- | --- |
| STL, OBJ, GLB, 3MF | Polygon mesh for viewing and mesh export. |
| STEP, STP | Exact CAD geometry and preview in a native desktop build. |
| PNG, JPG/JPEG, WebP | Project image attachment; eligible for AI context under the [AI rules](ai.md). |
| PDF, DXF | Drawing attachment; image-based AI first needs a raster image. |

Ordinary imports are limited to 40 MB per file. External OBJ (`mtllib`) and GLB resources are not loaded: the supplied file must be self-contained. Import does not automatically alter an existing exact CAD model. See [file validation](../../apps/desktop/src/lib/files.ts) and [native conversion](../../apps/desktop/src-tauri/src/native/).

## Export

Export can target the full model or one body. STL, 3MF, GLB, and OBJ are generated from the current mesh. STEP requires an exact native body in a desktop build. A mesh-only STL/OBJ cannot become exact STEP without CAD reconstruction. STL and OBJ carry no reliable units; Forma writes their coordinates in millimeters. GLB uses meters. Export neither changes the project nor creates a revision. See [dialog](../../apps/desktop/src/app/dialogs/ExportDialog.tsx) and [body filtering](../../apps/desktop/src/features/viewer/exportBodies.ts).

Use `.cadpack` to move a project's history and attachments, or copy the full app data directory to preserve every setting. See [using Forma](using-forma.md) and [data recovery](data.md).

## Cursor and panning

In 1.2.6 right-drag pans through OrbitControls using the ordinary system cursor, matching left-drag. There is no pointer lock or crosshair. Left-drag rotates, right-drag pans, and the wheel zooms in perspective, orthographic projection, and draft preview. Releasing the button ends the gesture. The render menu aligns with the trigger’s left edge; Radix may shift it near viewport boundaries to keep it visible. Verify both buttons, release, projection, language, and theme switching.

## Selection and file exchange fixes in 1.2.6

In Solid + edges, decorative outlines no longer intercept surface clicks. Faces use a stronger orange overlay; selected edges use a four-pixel stroke. [Edge picking](../../apps/desktop/src/features/viewer/pickCadEdge.ts) ranks the cursor distance to projected CAD polylines within eight screen pixels, independently of units and zoom, and rejects edges hidden behind solid geometry. It runs on click, not every frame. Shared endpoints are ambiguous: click closer to the middle of the intended edge. Display curves are approximated; their exact lengths come from B-Rep.

**STEP export:** Export → STEP → export, confirm according to application settings, and choose the name/folder in the native Save dialog. Cancellation does not record an export. Internal `output` files are intermediate artifacts; users do not need to locate them. Existing STEP files are retained. [Backend](../../apps/desktop/src-tauri/src/cad.rs), [destination picker](../../apps/desktop/src-tauri/src/export_destination.rs).

**Opening STEP:** Import model → STEP/STP → create project → confirm local conversion. Import into an existing project also offers conversion automatically. Wait for Converting STEP: a new revision and 3D scene appear. On error the original attachment remains; retry by clicking the STEP attachment. Rejecting confirmation leaves an attachment without a revision. Multiple files are retained; the first STEP is converted automatically unless a mesh was included, in which case the mesh becomes current. Other STEP files can be converted from the attachment tree. Native builds need no Python. The browser does not convert STEP. [Converter](../../apps/desktop/src-tauri/src/conversion.rs).

**Mesh round-trip:** STL/OBJ/3MF do not retain B-Rep or CAD operation history, so exact CAD selection is unavailable. GLB exported by Forma 1.2.6+ retains validated face/edge data and exact measurements, enabling CAD selection again after import. This does not recover STEP or operation history. Older GLB without those data remains a mesh. [Metadata transport](../../apps/desktop/src/lib/cadTopologyTransport.ts).

For imported geometry without CAD IR, the editor explains the missing history, does not offer a starter sketch as its source, and prevents accidental replacement through Build. Create a separate project for a new sketch. Transfer a complete [CADPACK project](projects.md) to retain operations and revisions.

Verification: export STEP to a selected folder, import into a new project and confirm conversion, compare shape/dimensions, select a face in Solid + edges, click near an edge midpoint and far from edges, run a GLB round-trip, and verify that the STL editor cannot replace the model. Native save dialogs require additional manual Windows verification; browser checks do not replace WebView2 checks.

[Import preparation](../../apps/desktop/src/features/import/projectImport.ts) is separated from App: duplicate names and meshes are validated before project changes; failures leave history untouched.

## Adaptive grid

The existing 550 mm fade distance is the minimum. [grid](../../apps/desktop/src/features/viewer/grid/AdaptiveGrid.tsx) computes world bounds once when geometry changes, including height and displacement from the origin. Large parts grow the fade range; smaller parts shrink it to the minimum. A shader uniform interpolates for 500 ms without rebuilding the grid or resetting the camera. Cells remain 10 mm and major lines 50 mm; measurement scale stays fixed. Themes retain current extent, and reduced motion snaps. Empty/nonfinite bounds fall back safely; the grid cannot intercept CAD selection. Extent is derived from geometry rather than saved in the database.

Verify small→large→small, translated models, empty scenes, both projections/themes, and reduced motion; check monotonic animation, unchanged camera, and CAD selection. [Calculation](../../apps/desktop/src/features/viewer/grid/gridExtent.ts).


Large models can move the camera up to 100000 mm away; Bounds recomputes clipping from geometry. Grid resizing never moves the camera.


## Readable grid at distant zoom

The grid retains its minimum 550 mm extent and smooth model-based resizing. Line spacing now follows perspective/orthographic camera scale and viewport height. Adjacent decimal scales crossfade smoothly; subpixel lines disappear and opacity is capped, preventing dense distant patterns. Close viewing retains the 10 mm cell / 50 mm major baseline. This visual guide changes no model coordinates, snapping, camera or source. `gridScale.ts` computes scale and `gridMaterial.ts` performs GPU antialiasing/blending without rebuilding geometry per frame. Theme changes preserve the scene. Checks cover several distances, orthographic zoom, viewport resizing and both themes.


Render-mode menus align with the entire visible control, including the layers icon. Previously the icon was outside Select, shifting the menu roughly 30 px from the outer shell despite alignment with its inner trigger. The icon now lives inside Select and the whole area is clickable. Tests compare menu bounds with both trigger and shell.
