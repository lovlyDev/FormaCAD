# Thicken a selected face

[Documentation](index.md) · [Русский](../ru/thicken.md) · [M4](modeling-ii.md)

## Purpose and workflow

The operation creates a separate solid from one explicitly selected CAD face. The original body retains its geometry and source. Shell instead removes selected faces from the original solid to leave a cavity.

Open an authored model, enable face selection and pick a face in the viewport. Choose Tools → Thicken face. The dialog shows the body ID and exact face role; it never automatically chooses the first face.

Enter a signed thickness in millimetres. Positive values follow the oriented face normal; negative values reverse it. Check the real 3D preview, click Build and approve the permission request. Empty and zero thickness disable Build. Cancel creates no revision. Use standard explicit Undo/redo afterward.

## Technical route

features/thicken captures the project, revision, body and a copy of TopologyReference. A changed project, revision or reference route invalidates that capture; the operation never moves to another body. Preview validates the same document described by the add_feature + add_body batch.

apply_ir_commands checks expectedRevision and uses a staged clone. CAD IR ThickenFace stores bodyFeatureId, reference and offset (literal or parameter). Rust native/thicken.rs calls the separate cad-core/src/thicken.cpp module. OCCT resolves the exact reference, copies the selected face and runs MakeThickSolidBySimple. The result must be a single closed solid: OrientClosedSolid sets orientation, and the common checked validator verifies BREP and finite positive volume and area. Source geometry is not edited.

The new feature and body publish in one revision only after worker success. Unsupported, stale or ambiguous references fail explicitly. Invalid thickness and impossible thickening produce translated errors with separate technical details.

## Storage and history

The operation is stored in the current revision's CAD IR with original dependencies and a new body. It uses the existing project format without SQL migrations. Bodies are not automatically fused. With negative thickness, the new body may lie inside the original; assembly totals sum body volumes and do not represent their Boolean union.

Thickening generates a new body. Pass-through suppression that would substitute the entire original solid for the plate is forbidden. History changes require explicit commands. A failed staged operation preserves the prior project snapshot.

## Geometry checks

A 40×20×10 mm box's upper face at ±2 mm produces a 40×20×2 mm plate: 1600 mm³, 1840 mm² and 6 faces. Positive offset yields Z=10…12; negative yields Z=8…10. The lower face at +2 mm yields Z=−2…0. The source stays at 8000 mm³.

An R10/H20 cylinder side at +2 mm yields 880π mm³ and 968π mm²; −2 mm yields 720π mm³ and 792π mm². Each sleeve has 4 faces. STEP round-trip, BREP validity, area and volume are checked. Collapse at −12 mm, zero, NaN, out-of-limit values and invalid references are rejected.

The Rust/worker transaction test verifies atomic body creation in one revision and full rollback for a cylinder-face reference absent from the box. Whole-assembly STEP round-trip preserves 9600 mm³, 4640 mm², 12 faces and 24 edges. A rotated face with an explicit occurrence also preserves 1600 mm³ and a 2 mm thickness along its rotated normal.

The built Windows application with embedded frontend passed real UI → IPC → host → OCCT in RU/EN and both themes: 4/4 without JavaScript errors. Fixtures use real native commands; the face is picked by an actual viewport click. The selected-face text confirms body · box-face:zmax. Zero and empty input disable Build; cancellation leaves the revision unchanged. Approved Build adds one revision: original 8000 mm³, plate 1600 mm³ / 1840 mm², assembly 9600 mm³ / 4640 mm². EN/light also covers −2 mm. Undo restores one body and 8000 mm³; redo restores two bodies and 9600 mm³, retaining the saved thicken revision.

The repeatable [thicken-native.mjs](../../scripts/acceptance/thicken-native.mjs) script uses the isolated host/worker and CDP setup in the [M4 guide](modeling-ii.md). From the root in PowerShell set $env:FORMA_NATIVE_ACCEPTANCE='isolated', then run node scripts/acceptance/thicken-native.mjs http://127.0.0.1:9336 <absolute-path-to-report.json>. Its output directory must exist. The script modifies only the separate test profile, saves a report and screenshots, never replaces IPC and does not retry failed commands. The pick point stays clear of the existing central M3 extrusion gizmo, retaining real hover on the same CAD face.

Validation: 278 Rust lib PASS / 2 existing ignored; real host/worker transaction PASS; 26/26 CTest; 299/299 Vitest; cargo fmt --check and strict Clippy in native-occt and default configurations; lint, production build and check:i18n/docs/migrations/version PASS. No installer was built.

## Limits

Selection accepts one face with supported exact topology provenance. Face sets and free shells are not accepted. Imported or rebuilt geometry without that provenance disables the command; a triangle ordinal never substitutes for a CAD reference. OCCT may reject self-intersections and excessive inward offsets. New result surfaces do not receive automatically rebound named roles. Thickness is entered in the dialog; a dedicated F9 editor is not added.

Concurrent storage read issues remain tracked in [Issue #32](https://github.com/lovlyDev/FormaCAD/issues/32#issuecomment-6825300767). Acceptance after thumbnail setup does not establish a read-after-write fix.
