# Exact measurements between two CAD elements

[Documentation](index.md) · [Русский](../ru/pair-measurements.md) · [Single selection measurements](reference-measurements.md)

This capability measures two supported edges or faces of **one saved named body**. It does not modify the model, create a revision or attach a permanent dimension. Local release verification and installer readiness are recorded separately; this guide does not establish completion of 2.0 or installation on the user's computer.

## Select and capture a pair

Open **Exact pair measurements** below model properties. Select a CAD edge or face in the committed 3D scene and click **Capture first selection**. Select the second element of that same body and click **Capture second selection**. Choose a measurement and click **Measure captured pair**. The slots display the captured element kind and its authored feature/role identifiers. Selecting another element does not silently replace a captured slot; use the capture button explicitly. **Clear captured pair** clears both slots.

Both references must come from the same displayed saved body and revision. Real clicks carry the loaded scene object's UUID; capture requires that UUID to match the parent's current committed, ready, interactive scene. Switching project, head revision, body, raw program, source identity or committed scene invalidates the slots and hides the report immediately. Preview/draft scenes are not measurement authority. Imported geometry or a visually similar edge cannot supply a guessed authored reference. Re-select and capture the elements after their saved geometry changes.

| Measurement | Supported pair | Meaning |
|---|---|---|
| Minimum distance | Edge/edge, face/face or mixed edge/face | Shortest distance between the actual finite, trimmed BREP elements. Identical, touching or intersecting elements can correctly return zero. |
| Outward face-normal angle | Two planar faces | Angle between outward normals, from 0° to 180°. Opposite normals give 180°, rather than an unsigned acute plane angle. |
| Acute straight-edge angle | Two straight edges | Angle between unoriented supporting lines, from 0° to 90°. Reversing an edge's BREP orientation does not turn 0° into 180°. |

The bounded reference catalogue covers positive rectangle/circle extrusions and supported tracked translations, rotations and mirrors. Circular cap edges and the cylindrical side can participate in minimum distance. Circular-edge angles and curved-face normal angles are unavailable; no guessed tangent or representative curved-face normal replaces them. Booleans, fillets, unowned imports, seams, negative circle extrusions and foreign transform branches do not gain synthetic references. The native resolver makes the final decision even when client metadata claims an authored route.

For the currently supported uniformly transformed constructors, available normal angles are generally 0/90/180° and straight-edge angles 0/90°. This stage does not add arbitrary nonorthogonal sketch entities, cross-body pairs, point/vector measurements, constraints or persistent pinned dimensions.

## Results, units and waiting

Distance and the two minimizing witness points are returned in CAD millimetres. Display follows the project's mm/cm/inch setting and current number locale; coordinates remain CAD X/Y/Z. Angles always display degrees and do not receive length-unit conversion. Witnesses are a representative minimizing pair, which may not be unique; they are not stable geometric anchors or newly selectable entities.

Only one unfinished request is allowed in the mounted measurement session. **Stop waiting** prevents adoption of its later result; it does not claim that the worker has stopped. The UI waits for that request to settle before starting another. Unmounting also prevents adoption without cancelling native work. The shared backend CAD queue remains authoritative across UI remounts.

A retry for the exact same captured context can retain the prior confirmed value with an explicit previous-result label while the retry runs or fails. Changing the captured pair, measurement kind or saved context hides that result on the first render. Errors are localized; technical details remain available to the application error boundary. Unsupported geometry is an explicit error, never a plausible zero or a mesh approximation.

The disclosure uses the Model Editor's shared animation and Lucide icons, 32 px controls, responsive layout and theme colors. Its `forma.ui.project.<id>.pairMeasurement.open` preference uses the existing localStorage-to-SQLite preference route with an 800 ms debounce. Captured references, selected pair measurement kind, results and witnesses are temporary session state: reopening a project requires capturing the pair again. No measurement report, reference pin, revision/history row or CAD constraint is persisted.

## Captured input and exact route

Each real viewport edge/face click carries the loaded committed object UUID. Capture requires that selection UUID to match the currently displayed, aligned native scene. A residual selection from a retired object cannot be captured even before viewer cleanup effects run. This scene token stays in renderer session state; it is neither a persistent topology ID nor an IPC geometry authority.

The [pair measurement feature](../../apps/desktop/src/features/pair-measurements/PairMeasurement.tsx) separates capture, strict schema, API transport, async adoption and unit formatting into individual modules. `measure_model_pair` accepts project ID, expected revision, named body ID and an ordered query containing the two topology identities. It accepts no renderer coordinates, normals, scalar, second body, paths or ordinal-derived selection.

The [read-only host workflow](../../apps/desktop/src-tauri/src/model_pair_measurement/build.rs) captures raw authored document bytes, committed source identity and required imported seals through the shared pure-SQL snapshot reader. It does not publish pending recovery, clear journals or write model history. Pending recovery requires reopening the project. The host acquires the shared CAD queue, rechecks captured inputs after waiting, stages bounded fixed-name inputs in an owned transient directory, runs the isolated worker, verifies its result, and repeats cancellation/head/source/import checks before returning it. Temporary staging is cleaned up; it is not a persistent dimension database.

The worker rebuilds the selected authored body. Each owner/role/occurrence path independently resolves through the exact bounded catalogue and membership in that body's actual BREP. The [separate C++ module](../../cad-core/src/pair_measurement.cpp) uses `BRepExtrema_DistShapeShape` for finite-entity minimum distance, signed outward planar normals for face angles and exact line directions for acute edge angles. OCCT exceptions do not cross the application boundary.

Reports are limited to 64 KiB and bind project, revision, body, raw-document/source hashes and sizes, ordered references, imported seals, request UUID and OCCT protocol. Query/result discriminators and extra fields are checked strictly. Zero is valid. Witness coordinates must be finite within ±1e9 mm, and their separation must match the scalar within `max(1e-6 mm, distance × 1e-9)`. Different result variants cannot be substituted for one another.

The evaluation source is `rebuiltAuthoredBody`. The source STEP seals bind the committed inputs, but these measurements evaluate the body reconstructed from authored IR. They do **not** independently prove geometric equality between that rebuilt body and the saved STEP. The report is also not a comparison against renderer mesh coordinates, which use a different axis mapping.

## Verification scope

The [independent native oracle](../../cad-core/tests/pair_measurement_test.cpp) covers a 40×20×10 box: opposing face distances 40/20/10, touching and identical references zero, parallel edges 10, diagonal edge separation √500, mixed edge/face distance 10, outward normal angles 0/90/180° and acute edge angles 0/90°. These comparisons repeat after non-origin rotation, translation and oblique mirror, and after resizing to 52×30×17.

Cylinder radius 8/height 25 checks cap circles, cap faces and mixed separation 25, side/cap touching zero and opposed normals 180°. Transformed and resized cylinders exercise the same semantics. Foreign second owners, wrong occurrence paths, seam roles, invalid kinds, curved angles and topology-changing outputs must be rejected. Worker/host, UI real-click, stale-response, localization, theme and packaged-artifact results belong to the release verification record; native analytic coverage alone is not a claim that all those layers have passed.
