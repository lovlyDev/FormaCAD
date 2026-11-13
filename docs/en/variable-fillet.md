# Variable radius fillet

[Documentation](index.md) · [Русский](../ru/variable-fillet.md) · [M4](modeling-ii.md)

## Purpose and workflow

Select one open CAD edge on an authored body. Press Ctrl+B and select “Variable radius fillet” under “Edge treatment”. The dialog requests exact start/end coordinates from OCCT in CAD X/Y/Z millimetres. They are the underlying BREP curve endpoints, never a world-coordinate sort: moving a body retains the radius binding.

Enter positive start/end radii up to 10000 mm. Fractional values and the length units supported by numeric input are accepted. “Swap endpoint radii” explicitly reverses the law. Review the real preview, select “Build” and grant permission. Empty/invalid inputs block application; native errors are translated. Cancel adds no revision. Standard history controls perform explicit undo/redo without rewriting the saved fillet revision.

## Route and storage

The features/variable-fillet module captures project, revision and exact edge provenance. Endpoint reads use measure_model_reference with query.kind=edgeEndpoints: the host binds the report to committed document, STEP and revision; the worker rebuilds the authored body and OCCT resolves TopologyReference. Context changes hide stale coordinates. Failed IPC calls are not silently retried.

The add_feature + set_body_source packet passes through apply_ir_commands and a staged transaction. CAD IR VariableFillet stores bodyFeatureId, reference, radiusStart and radiusEnd; radii support literal and parameter dimensions. Rust native/variable_fillet.rs has its own CXX bridge to cad-core/src/variable_fillet.cpp. The source BREP and geometry are deep-copied. MakeFillet applies a linear rolling-ball radius law along the spine; contour orientation is matched to exact vertices. Shared checked validation requires a valid BREP with finite positive volume and area. Impossible geometry cannot publish staged state.

The operation is appended to the existing revision CAD IR. Earlier operations remain dependencies. Suppression passes through its source; independent body copies remap dependencies and private parameters. No storage migration is needed.

## Limits and validation

One open edge with unambiguous saved provenance is supported. Closed edges and OCCT's automatic tangent-chain expansion are explicitly rejected. Unsupported/ambiguous references never fall back to the first edge. Fillet-generated topology receives no invented stable names. A dedicated F9 editor is not included yet.

A linear radius law does not imply circular cross-sections in transverse planes; integrating those circles is an incorrect exact-volume oracle. CTest checks constant radius analytically, variable-radius volume bounds, independent sections near both endpoints to verify direction, BREP validity, volume/area STEP round-trip, translation and negative cases. Rust checks dependency/copy semantics and complete rollback for impossible radii through a real worker transaction. Built-application acceptance results are recorded below and in PR #50; CTest alone does not establish UI/IPC acceptance.

## Windows native acceptance

Built client with embedded assets (http://tauri.localhost), original IPC, host and OCCT: RU/EN × dark/light — 4/4 PASS, no IPC substitution or unhandled JavaScript errors. Script: scripts/acceptance/variable-fillet-native.mjs; isolated temporary profile, fixtures created through standard commands, visible thumbnail publication before baseline reads.

On a 40×20×10 mm box the actual mouse-selected edge is box-edge:z:xmax:ymin. Exact endpoints: (20, −10, 0) and (20, −10, 10) mm.

| Start → end radius | Volume mm³ | Area mm² | Faces / edges |
|---|---:|---:|---:|
| 1 → 3 mm (RU dark/light, EN dark) | 7990.523716955377 | 2789.3301342970203 | 7 / 15 |
| 1.25 → 3.5 mm (EN light) | 7986.765923888611 | 2786.932710141804 | 7 / 15 |

Checks cover empty/zero input, 50/50 mm failure, cancel without a revision, swapping radii, a genuinely visible WebGL preview, UI permission, exactly one revision, unchanged earlier program and operation prefix, undo to 8000 mm³ and redo to the original result. Read-only inspect_model measures committed output; tolerance against the independent C++ probe is 0.001 mm³/mm². STEP round-trip and complete rollback snapshots are tested separately through a real worker transaction. This does not establish a full cross-platform OCCT matrix.

Each of the four cases also switches theme through the standard settings control and restores it: the DOM canvas remains connected, camera and displayed revision remain unchanged. Final native scenario: 4/4 PASS. Local checks passed: cargo fmt --check, strict default/native-occt Clippy with -D warnings, Rust lib 283 PASS / 2 existing ignored, real worker transaction 1 PASS, CTest 29/29, Vitest 311/111 files, full browser e2e 71 PASS / 89 existing skips, lint/build, check:i18n (1679), check:docs (158), check:migrations and check:version. Original e2e, hover and timeouts are unchanged. Stage CI is recorded separately in PR #50.
