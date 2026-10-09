# Sections from a selected planar CAD face

[Documentation](index.md) · [Русский](../ru/face-sections.md) · [Numeric sections](model-sections.md)

The selected-face section mode derives its plane from a supported authored CAD face and a signed offset. It intersects the **whole saved STEP**, including bodies hidden only in the viewport. This is a read-only view operation: the model, revisions and exported geometry do not change. This guide describes the integrated local 1.2.6 source; final installer and verification evidence belong to the release record when completed.

## Using the controls

1. Open a saved native CAD project with an exact STEP/STP source. Choose **Select CAD face** and click a planar face of a supported positive rectangle extrusion or a circular extrusion cap. Tracked translation, rotation and mirror branches retain supported references. A cylindrical side, mesh-only face or unreferenced imported face cannot define this plane.
2. Open the scissors **Section view** panel and choose **Selected CAD face** under **Section source**. Keep **Clip model with section plane** enabled. The other source, **Numeric or free plane**, retains XY/XZ/YZ/free-normal controls and its automatic calculation route.
3. Enter **Offset from selected face** in the project's display units. The backend receives millimetres: 1 cm becomes 10 mm, and 1 in becomes 25.4 mm. Positive offset follows the outward normal; negative offset moves inward. For a box whose top is z=10 mm, top offset −5 mm gives z=5 mm. The bottom's outward normal points −Z, so its offset −5 mm also gives z=5 mm.
4. With **Update face section automatically** enabled, a stable input starts calculation after a 400 ms pause. **Calculate section** requests the current valid selection without that debounce. Disable automatic updates to calculate only on demand. Switching automatic updates off does not cancel an already running worker.
5. While a different face, offset or saved context waits, its previous plane and curves are hidden. Clipping starts only after a verified current report arrives. Calculating again for the same unchanged context keeps its verified plane during the request, avoiding a flash; a failed recalculation clears it. Changing project, head, source, authored program or displayed scene also invalidates adoption. One unfinished invocation is retained per mounted face-section hook; mode switching also waits for the existing numeric-section invocation. Explicit calculation is bound to the captured context and cannot transfer to another project before launch. The shared CAD queue remains the authority across components.
6. Uncheck clipping to restore the full view. Closing the panel alone leaves its enabled state intact. A valid empty intersection has zero length and no curves. Offset zero may return a coincident face boundary; moving outside the selected body can still intersect other bodies in the saved STEP.

The panel uses the editor's disclosure motion, project Checkbox/Select/Button components and Lucide icons. Its surfaces, text and result lines follow light/dark themes and reduced-motion preferences. Displayed exact length uses project units. No filled section cap, section area or drawing export is generated.

## Plane authority and execution

The renderer invokes `section_model_reference(projectId, expectedRevision, bodyId, query)`. The strict query contains an operation-owned face reference, `offsetMm` and `deflectionMm`; it contains no renderer-authoritative plane or mesh ordinal. The host shares the [measurement snapshot](reference-measurements.md) reader: pure SQLite reads and immutable folder/index comparisons, current revision/body checks, raw saved-document bytes and source/preview/import seals. Pending recovery is rejected without publishing recovery or clearing journals.

After acquiring the [CAD queue](cad-task-queue.md), the host rechecks captured inputs and stages fixed-name files in its owned `.transient/face-sections/{project-id}-{uuid}` directory. Worker operation `section_reference` validates raw document/source hashes and sizes plus imported STEP seals. It rebuilds the named authored body, resolves the exact planar BREP face and obtains its trimmed area centroid `C` and signed outward unit normal `N`. The derived plane is `P = C + offsetMm × N`. Unsupported, missing or ambiguous references fail without a nearest-face or axis-plane fallback.

The worker reads the sealed **whole** `source.step` and intersects it with that derived plane. Native BREP integration supplies lengths; bounded samples supply display curves. It rechecks staged inputs before writing temporary `face-section.json`. The client independently verifies request UUID, exclusive response fields, report hash, revision/body/document/source/import/query bindings, finite geometry, curve totals, point-plane membership and closed flags. The host performs final snapshot and cancellation checks before returning.

The report's `evaluationSource` is `rebuiltAuthoredFaceAndSealedStep`: the plane comes from the rebuilt authored body, while curves come from the committed STEP. Matching seals establish input integrity, **not independent geometric equality** between those representations. Viewport hidden-body preferences do not filter the native STEP section.

## Preferences, cancellation and bounds

The existing project view preferences add `.section.mode` (`manual` or `selectedFace`), `.section.faceOffsetMm` and `.section.faceAutomatic`, alongside open/enabled/axis/numeric-offset/normal preferences. For the main view the prefix is `forma.ui.project.<projectId>.view.main`. Shared persistence updates localStorage immediately and flushes `set_ui_preferences` to SQLite `settings/ui_preferences` after 800 ms; startup fills missing keys. These are machine view preferences, not project payload edits. The selected reference, computed plane, report and curves are not persisted as a pin or model constraint. After reopening, select a supported current face again; restored mode/offset alone do not supply a face reference or report.

Selection changes, closing/unmounting the view and disabling its mode do not prove native cancellation. Obsolete results cannot adopt. Use the task indicator's actual CAD task identity to cancel a queued or running operation. The worker timeout is 120 seconds. Ordinary completion, failure and cooperative cancellation remove only the owned workspace; guarded cleanup failures are logged, and arbitrary Future abortion or panic cleanup is not guaranteed.

Signed offset is finite within ±10,000 mm. Each **derived world-plane origin coordinate** must also remain within ±10,000 mm; valid offset alone does not guarantee this. Values are rejected rather than clamped. Deflection is 0.001–1 mm; limits are 4,096 curves, 100,000 points, an 8 MiB report, a 40 MiB source and existing imported-asset budgets. Empty total length is valid zero; individual returned curves have positive length.

Kernel/report coordinates are CAD millimetres. The model loader scales native GLB metres back to millimetres, so workspace overlays use `cadToWorld(x,y,z)=(x,z,−y)` without division by 1,000. Normals use the same axis rotation without length scaling.

Shared snapshot failures can return `MEASUREMENT_RECOVERY_REQUIRED` or `MEASUREMENT_STALE`; reopen a project that needs recovery. Section failures include `INVALID_SECTION_PLANE`, `SECTION_FAILED`, `SECTION_LIMIT`, `SECTION_STALE`, asset-integrity errors and task cancellation. Display messages are localized; technical details remain separate.

## Verification route

`cad-core/tests/selected_face_section_test.cpp` provides an independent analytic whole-STEP oracle, including a box/cylinder section of `120 + 16π` mm, signed cap normals, transformations and valid empty results. Native worker/host tests cover binding, source integrity and read-only lifecycle; frontend hook/schema tests cover stale adoption and bounded reports. Browser interaction uses real face clicks with a test-only IPC boundary and genuine isolated worker reports. These checks do not establish an installed WebView, arbitrary topology remapping or a completed 2.0 release. Consult the release record for finalized results.
