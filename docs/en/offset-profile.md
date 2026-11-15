# Offset profile

[Documentation](index.md) · [Русский](../ru/offset-profile.md) · [M4](modeling-ii.md)

This command offsets an authored extrusion profile using exact OCCT geometry. The original profile and extrusion stay in history; the new extrusion retains the original distance expression, including a parameter reference.

## Workflow

Open a model with an editable authored extrusion. Choose Tools → Offset profile. Select the body by name and full ID; multiple bodies with no selection require an explicit choice. The dialog shows the source profile and full ID. Enter a nonzero offset in millimetres and choose extended-edge intersection or rounded outer corners. Positive offset expands material; negative offset contracts it. Fractional values are accepted.

Review the real 3D preview and measurements. Build becomes available only after successful geometry validation. Approve the permission request. Cancel preserves the project. Explicit history buttons perform undo/redo; the created revision remains in history.

## Kernel, commands and data

Route: features/offset-profile → apply_ir_commands → staged clone guarded by expectedRevision → CAD IR OffsetProfile → native/offset_profile → cad-core/src/offset_profile.cpp → OCCT. The new PlanarProfile owns a planar BREP region with finite positive area and perimeter. The positive-volume Solid contract is unchanged.

CAD IR stores sketchId, signed Dimension offset and explicit join (intersection/rounded). Supported sources are rectangles, circles, solved sketch2d regions with holes and previous offset profiles. Parameters and dependencies survive regeneration and independent body copying. API extrusion and revolution can consume the planar result.

The outer contour offsets by d and every hole by −d. Closed contours, BREP, hole containment, intersections and unchanged contour counts are checked. A disappearing hole, collapse, multiple regions or a hole leaving the outer contour fails explicitly. Source geometry is copied before OCCT computation. Circular corners remain exact arcs, without polygonal approximation.

The add_feature(OffsetProfile) + add_feature(Extrude) + set_body_source batch creates one revision only after worker success. Previous feature semantics are retained without forward references. The shared Command API can explicitly serialize previously omitted defaults, such as construction: false; an already saved earlier revision program is never rewritten. Failure rolls back metadata, artifacts and history. Storage schema and SQL migrations are unchanged.

## Validation

C++ checks BREP and STEP round-trip for positive/negative extrusion in XY/XZ/YZ with translated origins, circles, rounded corners, holes and revolution. Rectangle 40×20, +2, H10: 10560 mm³ / 3472 mm²; −2: 5760 mm³ / 2192 mm². Rectangle 40×20 minus hole 10×6: +2 gives 10440 mm³ / 3608 mm²; −2 gives 4360 mm³ / 2392 mm². Circle R10, +2, H10: 1440π mm³ / 528π mm². Rounded rectangle +2: 10400+40π mm³ / 3280+48π mm².

The built Windows app with embedded production frontend passed UI → real IPC → host → OCCT acceptance: RU/EN × dark/light — 4/4 PASS, without JavaScript errors. Actual canvases, zero/empty offsets, collapse and disappearing-hole errors, cancellation without a revision, one successful revision, retained distance expression and prior revision program, and undo/redo retaining history were checked.

| UI case | Volume, mm³ | Area, mm² | Faces / edges |
|---|---:|---:|---:|
| Rectangle +2, RU/dark | 10560 | 3472 | 6 / 12 |
| Profile with hole −2, RU/light | 4360 | 2392 | 10 / 24 |
| Circle +2, EN/dark | 4523.893421 | 1658.760921 | 3 / 3 |
| Rounded corners +2, EN/light | 10525.663706 | 3430.796447 | 10 / 24 |

Local checks passed: 28 CTest, 281 Rust lib tests (2 existing ignored), a real worker transaction with full rollback and STEP, 309 Vitest, fmt, strict default/native-occt Clippy, lint/build and catalog/document checks. Full browser e2e passed 71 cases / 89 existing skips; the original project-card case also passed 3/3 after the navigation fix. The full cross-platform OCCT CI matrix was not run. Reproduction: [offset-profile-native.mjs](../../scripts/acceptance/offset-profile-native.mjs). Use an isolated profile of the built native-worker-client app with embedded frontend (no devUrl), real OCCT worker and loopback CDP http://127.0.0.1:9336. Set FORMA_NATIVE_ACCEPTANCE=isolated and run `node scripts/acceptance/offset-profile-native.mjs http://127.0.0.1:9336 <absolute-report.json-path>`. Only fixture projects are created in that profile; JSON/PNG are saved alongside the report. Fixtures wait for visible thumbnail publication; this is not immediate read-after-write verification or a fix for Issue #32. Failed IPC calls are not retried.

## Limits

UI requires a body whose active source is an authored extrusion of a supported profile. Boolean results, imported bodies and arbitrary CAD faces are never implicitly substituted for a profile. The API supports revolution, but this dialog does not create revolutions. Automatic rebinding of new faces and a separate OffsetProfile F9 editor are not provided. Contour counts are preserved: even mathematically valid disappearance of a hole requires a different explicit sketch edit. Input polygon profiles are bounded to 32 points and 7 holes. Offset range is ±10000 mm with magnitude above 1e−7 mm. The operation cannot be separately suppressed as a solid modifier; use explicit history. No Blender code or resources were used.
