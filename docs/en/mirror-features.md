# Mirror modeling operations

[Documentation](index.md) · [Русский](../ru/mirror-features.md) · [M4](modeling-ii.md)

The operation repeats a selected range's material changes by reflection while retaining dependencies on its original features. Original history remains in the document; the result is a separate feature.

## Workflow

Open an authored CAD model and choose Tools → Mirror features. Select a body by name and full ID, then the first operation. The range includes it and subsequent operations through the body's active source; the dialog shows the ordered list. The first operation is never selected automatically.

Choose world normal axis X/Y/Z and a plane offset in millimetres, including fractional values. Review the real 3D preview, click Build and approve permission. Cancel creates no revision. Standard Undo/redo controls make explicit history changes.

## Semantics and route

The range must be a contiguous suffix of the active chain, up to 64 operations. Supported nodes include solid generators and material changes: Booleans, holes, fillets/chamfers, face extrusion, shell, split, patterns and preceding feature mirrors. A Boolean's target left branch is its base. Affine transforms are outside the range; operations following a transform can use its result as their base.

A range beginning with a generator has an empty material base. Otherwise its base is the solid before its first feature. OCCT determines added material as after − before and removed material as before − after, reflects these regions and applies Fuse/Cut to the active body. One range can therefore contain both additions and removals. Original parameters remain dependencies for regeneration.

Route: features/mirror-features → apply_ir_commands → staged clone guarded by expectedRevision → cad_ir/mirror_features → native/mirror_features → cad-core/src/mirror_features.cpp → OCCT. Invalid ranges, broken references, invalid planes, no effect and disconnected multiple-solid results fail explicitly. Operations are nondestructive and validate BREP and finite positive measurements.

CAD IR MirrorFeatures stores bodyFeatureId, optional baseFeatureId, ordered featureIds, planeOriginMm and planeNormal. add_feature + set_body_source publish in one revision after worker success. Storage schema is unchanged. Explicit suppression returns the original input body. Independent body copying remaps baseFeatureId and featureIds as well, keeping the copy independent of original history.

## Control geometry

A 40×20×10 box with an R2/H4 boss at X=10, mirrored at X=0, yields 8000+32π mm³, 2800+32π mm², 10 faces and 18 edges. A through-hole R2 at X=10 yields 8000−80π mm³ and 2800+64π mm² after reflection, with 8 faces.

A combined R2/H4 boss and R1/H10 hole at X=5 yields 8000+12π mm³ and 2800+68π mm² after reflection. An R10 sphere generator mirrored at X=5 yields 2250π mm³ and 600π mm². STEP round-trip, unchanged source geometry, no-effect/disconnected failures, zero normal and NaN are covered.

Verified in the built Windows app through real IPC, host and OCCT worker, without mocked IPC: RU/EN × dark/light — 4/4 PASS, no JavaScript errors. Boss: 8100.530965 mm³ / 2900.530965 mm² (10 faces, 18 edges); holes: 7748.672588 mm³ / 3001.061930 mm² (8 faces, 18 edges); combined range: 8037.699112 mm³ / 3013.628300 mm² (12 faces, 24 edges). Sphere at fractional plane X=5.05: 7092.066613 mm³ / 1891.238777 mm² (2 faces). No-effect and disconnected failures block Build. Explicit cancel preserves the revision; success adds one revision and retains original features; undo/redo retain history.

Reproduction: [mirror-features-native.mjs](../../scripts/acceptance/mirror-features-native.mjs). Use an isolated profile of the built native-worker-client app, real OCCT worker, embedded assets (no devUrl), loopback CDP http://127.0.0.1:9336 and FORMA_NATIVE_ACCEPTANCE=isolated. Run `node scripts/acceptance/mirror-features-native.mjs http://127.0.0.1:9336 <absolute-report.json-path>`. The script creates fixture projects in that profile, checks actual canvases and writes JSON/PNG alongside the report. Baseline reads wait for the visible thumbnail after fixture publication; this is neither an immediate read-after-write test nor a fix for Issue #32. Failed IPC calls are never retried.

Local checks passed: 27 CTest, 280 Rust lib tests (2 existing ignored), a real worker transaction with error rollback and STEP round-trip, 301 Vitest; cargo fmt, strict default/native-occt Clippy, lint, production build and catalog/document checks.

## Limits

One solid is required before and after the range; an empty base checks only the generator result. The range's net added and removed regions are repeated. Disconnected copies need a separate body workflow. UI planes use world axes; the API accepts arbitrary normals. Automatic naming or rebinding of new result faces is not promised. No Blender code, resources or keymap were used.
