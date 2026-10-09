# Exact authored-body and reference measurements

The 1.2.6 inspector layout displays the selected entity and the revision-scoped selection explanation on separate lines with explicit spacing. Its component styling lives in `apps/desktop/src/app/ModelInspector.css`, preserving the same theme colors and model data. This is a display change; it adds no saved preference or measurement record.

[Documentation](index.md) · [Русский](../ru/reference-measurements.md) · [Topology](topology-references.md)

To capture two elements of the same saved body and measure their minimum separation or supported angles, see [exact pair measurements](pair-measurements.md). Pair capture is explicit and temporary; it does not create pinned dimensions.

Rectangle and [circular constructor references](circular-measurements.md) are included in consolidated 1.2.6 source. Earlier local test evidence does not establish the new installer; final package checks are recorded in the [release notes](../releases/1.2.6.md). 2.0 remains unfinished.

Select a named CAD body, edge or face and open **Exact selection measurements** below model properties. Choose a measurement and click **Measure selection**. The selected body provides volume, surface area, face and edge counts, and axis extents. Supported edges provide length; faces provide area or a planar face's area centroid and outward unit normal. Coordinates use CAD X/Y/Z. Project units apply to lengths, squared areas and cubed volumes; normals remain dimensionless. The controls use the editor's shared disclosure animation, project icons and theme colors.

Current face and edge references belong to positive rectangle and circle extrusions and their tracked translation, rotation and mirror operations. A straight box edge has no circular radius. Imported, Boolean and filleted bodies can provide whole-body metrics when the saved typed body rebuilds successfully; their appearance does not create constructor-owned references. Missing or ambiguous references are rejected rather than replaced by a mesh ordinal or nearby entity.

The properties area scrolls independently and occupies at most 65% of the project sidebar. Scroll this area to see a long report and its source information; the model tree retains space above it. Closing the measurement disclosure restores room without hiding model properties.

Local 1.2.6 adds [circular constructor measurements](circular-measurements.md), including exact radius and diameter. Its separate release record states the completed checks.

## Captured input and technical route

The [selection properties slot](../../apps/desktop/src/features/reference-measurements/SelectionModelProperties.tsx) keeps existing whole-model properties separate from selected-body measurements. Contract validation, captured selection, API transport, asynchronous adoption, capabilities and unit conversion each have their own file in [reference-measurements](../../apps/desktop/src/features/reference-measurements/ExactReferenceMeasurement.tsx). Results disappear on the first render of a changed project, revision, body, selection, query or source. A failed repeat for the identical context can show a labelled previous result.

The `measure_model_reference` command captures the expected committed revision, named body, raw document bytes and sealed STEP, preview and imported inputs. Its [pure snapshot reader](../../apps/desktop/src-tauri/src/model_measurement/snapshot.rs) compares SQLite with the immutable folder generation without publishing recovery or clearing journals. Pending recovery returns `MEASUREMENT_RECOVERY_REQUIRED`: reopen the project before measuring it. Metadata changes such as messages do not invalidate unchanged geometry inputs; a change during the snapshot read itself can conservatively require a retry.

[Host lifecycle](../../apps/desktop/src-tauri/src/model_measurement/build.rs) acquires the shared CAD queue, rechecks inputs after waiting and stages bounded fixed-name files in an owned transient directory. The isolated worker verifies document and STEP digests, imported seals and the named body, then uses the [exact BREP resolver](../../cad-core/src/topology_resolver.cpp). Length and area come from OCCT properties. A face plane uses its trimmed area centroid, actual body occurrence orientation and oriented surface derivatives; stored transform-catalogue orientation and a mirrored plane axis are insufficient to establish outward direction.

The [worker client](../../apps/desktop/src-tauri/src/native/reference_measurements/client.rs) validates report checksum, protocol and independent request UUID, engine, body/revision/document/source/import echoes, query and finite typed result. The host repeats input checks before returning. Renderer SHA comparison checks the captured raw document without sending that digest as host authority.

## Storage, cancellation and limits

The body-metrics query accepts only `{ "kind": "bodyMetrics" }`. References, mesh ordinals and other extra fields are rejected, just as mismatched reference kinds are rejected for edge and face queries. Rust represents this as an empty struct variant: an internally tagged unit variant would silently accept extra fields despite the enum's strict-field annotation.

Measurements consume no modify approval, add no revision or history event and store no pin or durable report. Ordinary success, error and cooperative cancellation clean only the owned `.transient/reference-measurements/{project-id}-{uuid}` directory. Guard and deletion failures are logged; arbitrary future abortion or panic cleanup is not guaranteed. User files, prior exports and installers remain unchanged.

**Stop waiting** prevents local result adoption and does not cancel native execution. This component permits one unfinished request per mounted session, including after stopping or changing context. The Tasks indicator can cancel the actual CAD task by its identity; the component never guesses a task from project or operation kind. Closing the panel is not proof that a worker stopped.

The report identifies its evaluation source as a rebuilt authored body. STEP checksums bind committed inputs but do not prove independent geometric equality between rebuilt output and saved STEP. General topology, pinned measurements and selected-face section controls require further work.

## Development verification

The actual React hook runs under StrictMode in six lifecycle tests: its synthetic cleanup does not close the live session or duplicate an explicit request; delayed responses cannot adopt after project, head, selection or sealed-source changes; real unmount closes the session before a delayed response finishes. These tests mock the measurement adapter and establish renderer lifecycle behavior, not native geometry or cryptographic validation. The four module suites passed 14 tests locally during integration, including strict body-metrics queries and normalized native reference property ordering.

The independent C++ stage passed three suites covering signed normals and centroids for all six box faces, rotation, translation, oblique mirroring, resized geometry and nonmutation, plus existing topology and fillet behavior. Full native regression passed 255 tests with two service helpers ignored, including actual workers, imported seals, read-only storage, queue staleness, metadata-only changes, pending recovery and cancellation after verified output. All 163 frontend tests passed. The exact Cargo-worker included in the Windows installer passed eight analytic measurements and three rejected queries (extra body-query fields, foreign owner, straight-edge radius), plus existing build/import/section/referenced-fillet smoke. All 15 prior installers retained their hashes. Chromium uses test-only IPC and genuine worker reports; it does not establish installed WebView behavior or replace actual native host tests.

The integrated actual-App Chromium run passed 18 scenarios, including four measurement scenarios in Russian and English with both themes. An isolated copy of the strict debug worker supplied checksum-verified body, edge, face and plane reports; a 40 × 20 × 10 mm box confirmed 8000 mm³ volume, 2800 mm² surface area, 40 mm edge length and 800 mm² top-face area. Browser tests verified powers in mm/cm/in units, 32 px controls, repeated disclosure motion, bounded sidebar scrolling, previous-result failure labels, stop-waiting bounds and discarded late selection/project responses. Measurement selections use production workspace setters; the coupled topology suite separately exercises real viewer clicks. IPC storage and queue lifecycle are mocked in the browser and verified separately in native tests. Evidence: the prior local verification log.
