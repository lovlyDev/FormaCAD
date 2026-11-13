# M4 — modeling tools II and data

[Documentation](index.md) · [Русский](../ru/modeling-ii.md) · [Plan](../architecture-2.0.md)

Branch task/m4-modeling-tools starts at main 8cb40fd (1.7.0-beta.1), Issue #21. This turn adds offset-profile on top of split and mirror-features. The previously added ThickenFace remains in the branch; renewed acceptance of thicken is next. This records current progress; it does not declare all of M4 complete.

## Modules and acceptance

Separate modules cover split, thicken, mirror features, offset profile, variable fillet, IGES, PBR/glTF appearance, geometry table, history timeline, saved views and smart selection. Each feature needs the actual C++/Rust, staged Command API and UI route, bilingual documentation, measurements, BREP validation, exchange round-trip where applicable and transaction rollback.

History changes must be explicit; ambiguous references require manual choice. Storage and recovery Issues #27–#34 belong to wenlixxq. No placeholder controls are introduced.

## Split body by plane

Open a project containing an editable authored body. Choose Tools → Split body by plane. Select the body (name plus full ID), normal axis X/Y/Z and plane offset in millimetres. Multiple bodies with no selection require an explicit choice. Review the preview, click Build and approve the permission request.

The original body keeps the positive half along the normal. A new body with the suffix (-) receives the negative half. Both are saved in one revision; original features remain as dependencies. Cancelling the dialog leaves the project unchanged. Undo/redo use the standard explicit history controls; the saved split revision is retained.

Route: features/split → apply_ir_commands → staged clone guarded by expectedRevision → cad_ir::SplitPlane → native/split → cad-core/src/split.cpp → OCCT Common with two half-spaces. Each part passes BREP validation and finite positive measurement checks; their volumes must sum to the original volume. Sides are explicit and do not depend on topology enumeration order.

The command batch adds two SplitPlane features, replaces the original body's source and adds the second body atomically. Data uses the existing project CAD document format without a storage migration. Failed validation never publishes staged state. Empty numbers, nonfinite coordinates, a zero normal, tangent and missed planes are rejected. Errors are translated while retaining technical codes.

Limits: the UI uses world-axis planes; the API accepts arbitrary nonzero normals. Face-attached planes are not implemented here. A part may contain several solids when the input is disconnected. Automatic rebinding of old topology references is not promised. IGES acceptance belongs to its own upcoming module.

### Geometry and real native acceptance

C++: a 40×20×10 mm box cut at X=5 yields 3000 and 5000 mm³, bounds 15×20×10 and 25×20×10 mm; the unchanged source remains 8000 mm³. An oblique plane through the centre yields 4000 mm³. An R10, H20 cylinder at Z=5 yields 1500π and 500π mm³; an R10 hemisphere measures 2000π/3 mm³ and 300π mm². A part's STEP round-trip preserves 3000 mm³. Tangent, missed and invalid-normal cases are covered.

The transaction test uses the real worker to verify rollback after a missed plane, a single revision for both parts and a whole-assembly STEP round-trip: 8000 mm³, 3200 mm², 12 faces and 24 edges.

The built Windows application with embedded frontend was tested in all four RU/EN × light/dark combinations. IPC is not mocked: UI actions reach the real Tauri host and OCCT worker. Fixtures are seeded using real native commands and one-use grants; splitting itself uses the UI. Verified volumes are 3000/5000 mm³, total volume 8000 mm³, area 3200 mm² and 12 faces. A missed plane or empty number disables Build and leaves the revision unchanged. Undo restores one body and 2800 mm²; redo restores two bodies and 3200 mm². Three rapid preview cycles and dialog cancellation do not create a revision.

## Empty-intersection diagnostic from M3

Disjoint solids and face/edge-only contact produce no common solid. The kernel returns EMPTY_INTERSECTION; the preview displays “The intersection is empty.” while preserving technical details. Other invalid-volume failures retain their original meaning.

Route: cad-core/src/booleans.cpp → native Shape/worker report → features/model-preview/previewError.ts → ru.json/en.json. Invalid preview disables Build; a failed staged transaction leaves the saved snapshot unchanged. No persisted schema is added.

C++ covers disjoint solids, face contact and a valid self-intersection of a 40×20×10 box (8000 mm³). Rust checks the error code and rollback; frontend tests check the translatable key and retained details. Real UI checks in all four language/theme combinations show the clear error for two disjoint bodies totalling 9000 mm³ without creating a revision.

## Preview stability

Native acceptance exposed an event-connection error after rapidly closing a preview. The shared StableCanvas retains an actual DOM node for asynchronous React Three Fiber initialization. Both preview and main viewport use it; exceptions are not suppressed. Dedicated tests cover late initialization after unmount. Repeated native checks with rapid preview cycles completed without JavaScript errors.

## Read-after-write conflict

Source audit at 8cb40fd identifies a window in list_projects/get: the SQL commit exposes a pending journal while the writer still owns ProjectLock. Reader reconciliation can attempt the same immediate writer lock. Current inspect_model uses a pure snapshot and returns PROJECT_SNAPSHOT_RECOVERY_REQUIRED for pending recovery; its reported writer-lock error has not been reproduced independently.

The [report](https://github.com/lovlyDev/FormaCAD/issues/32#issuecomment-6768465150) and [explicit inspect_model route clarification](https://github.com/lovlyDev/FormaCAD/issues/32#issuecomment-6768476962) were handed to the Issue #32 owner. Storage source files were not changed; no retry delay masks the conflict. Controlled race reproduction and the storage fix remain with its owner.

## Remaining acceptance

Other M4 modules have not yet been implemented or checked. Every new geometry operation requires negative cases, BREP validity, measurements, exchange round-trip where applicable, rollback and built-app acceptance.

The PR remains draft until M4 is complete. No version, lockfile, installer or release changes.

## Repeating native split acceptance

The [split-native.mjs](../../scripts/acceptance/split-native.mjs) script runs separately from browser tests that mock IPC. Build the frontend and debug host with native-worker-client and embedded assets (TAURI_CONFIG with build.devUrl=null), plus the native-occt worker. Place both executables together. Use separate empty FORMA_DATA_DIR and WEBVIEW2_USER_DATA_FOLDER directories, OCCT libraries on PATH, and WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9336 --remote-debugging-address=127.0.0.1. Start that isolated application and wait for its dashboard.

From the repository root in PowerShell, set $env:FORMA_NATIVE_ACCEPTANCE='isolated', then run node scripts/acceptance/split-native.mjs http://127.0.0.1:9336 <absolute-path-to-report.json>. The output directory must exist. The script creates fixtures, changes locale/theme and saves a report with screenshots. Never connect it to a user's working profile. The reviewer prepares the empty profiles and separate process; the isolated variable acknowledges that preparation and does not itself verify the process path.

### Pointer input and the stable event source

The stable event source is the canvas ancestor. React Three Fiber disables hit testing on its internal container by default when given an external eventSource. Here, that prevented direct canvas hover: the pointer hit the ancestor even though the canvas was already visible and stable. StableCanvas explicitly preserves pointer-events: auto on the internal container; canvas events bubble to the same stable ancestor. The late-initialization guard remains active after closing the view, and caller sizing styles are retained.

Regression coverage checks hit testing and event bubbling, style forwarding and late connection after unmount. The original persistence e2e (“workspace survives reload with camera, draft, hidden body and display mode”) and workspace e2e (“viewport clicks and tools preserve camera zoom”) retain hover and the 90000 ms timeout. They check wheel zoom, tool changes and camera restoration after reload. Camera persistence format, geometry and project history are unchanged.

### Additional native Canvas acceptance

After the hit-testing fix, the built application passed RU/EN × light/dark: 4/4 with no JavaScript errors. Checks cover main and preview canvas hover, wheel zoom, split volumes of 3000/5000 mm³, undo/redo and no extra revisions. The repeatable script waits for a settled camera, then checks that wheel input toward the model reduces its distance to the target.

Fixture setup explicitly waits for the visible project-card thumbnail, confirming completion of the background metadata write. This does not test immediate read-after-write. Before that wait, two inspect_model reads failed with PROJECT_SNAPSHOT_CHANGED; polling list_projects during thumbnail publication also reproduced “Project storage is locked by another writer”. After thumbnail completion the same project reads successfully: 8000 mm³, 2800 mm². [Observations were handed to the storage owner](https://github.com/lovlyDev/FormaCAD/issues/32#issuecomment-6825300767); causality is not claimed without tracing. No product retries, delays or storage changes were introduced; the native script does not retry failed IPC.

The original local persistence/workspace e2e tests failed 2/2 on hover before the fix and passed 2/2 afterward without test changes. The first full local run separately failed locked-pan EN; isolated interaction repeats passed 8/8, and the next full run passed 71 tests with 89 existing skips. Browser native-fixture skips do not establish OCCT functionality; the built-app native matrix covers that separately.

[Thicken a selected face](thicken.md) — a separate M4 module; native matrix 4/4, original body retained, new body 1600 mm³.

[Mirror modeling operations](mirror-features.md) — native acceptance in progress.

## Offset profile

[Offset profile](offset-profile.md) — a separate exact extrusion-profile offset module. BREP, STEP, staged rollback and the built app in RU/EN × dark/light passed (4/4); measurements are recorded in its guide.
