# Native modeling parity and rigid transforms

This inventory records practical geometry paths, rather than declaring every CadQuery method name a separate user workflow. The native CAD IR v2 route remains incomplete for 2.0. Python compatibility is retained until the missing scenarios have equivalent typed operations and verified geometry.

## Current matrix

| Existing input/scenario | Native construction | Required invariant / regression | Status and limits |
| --- | --- | --- | --- |
| Blank template | Empty document | No fabricated body | Existing route |
| Box / plate | `rectangle` → `extrude`, optional `hole` | Width × depth × height minus drilled cylinders; STEP volume | Existing route; holes run along world Z |
| Cylinder / axial bore | `circle` → `extrude`, `hole` | π(R² − r²)h; STEP volume | Existing route |
| L bracket | Base/upright rectangular extrusions, translate, union, holes | Joined solid, requested wall thickness and bounds | Existing primitives allow construction; this inventory alone is not a completed golden-template parity test |
| Open enclosure | Outer rectangular extrusion, translated inner extrusion, cut | Bottom/wall thickness and open top | Existing Boolean construction; general face-selected `shell` remains unavailable |
| Sphere/cone, composed organic fixture | Native sphere/cone plus translate and Boolean union | Analytic primitive volume, valid resulting BREP | Existing primitive tests; arbitrary organic requests are not inferred automatically |
| Polygon/profile with inner holes | Constrained `sketch2d` → `extrude` | Solved area × signed distance; correct workplane | Existing tested route with lines, construction geometry and affine coordinate links |
| Repeated rectangular gear teeth | One tooth → translate → explicit `rotate` nodes → unions → central hole | Independent circle/rectangle integration; bounds 51 × 51 × 8 mm; STEP round-trip | New rigid-transform integration test reproduces the 16-tooth legacy fixture |
| Positioned / rotated body | `translate`, `rotate` | Volume/area unchanged, independently calculated AABB | New arbitrary-axis rotation and offset-axis tests |
| Mirrored asymmetric body | `mirror` | Volume/area unchanged; reflected position verified with Boolean overlap | New plane-origin and oblique-normal tests; it produces a reflected copy, not an automatic union |
| Whole-edge fillet/chamfer | `fillet`, `chamfer` | Valid BREP, expected volume reduction, failure at excessive size | Existing bounded operations |
| Selected edge fillet | `filletEdge` | Unique box-edge resolution across parameter changes | Limited box-edge selector; no general persistent topology guarantee |
| STEP / mesh import and export | Existing exact STEP conversion/inspection and mesh viewer/export routes | Geometry and units retained | `importStep` is a content-addressed typed source with native modifiers above it; an assembly remains one aggregate body; meshes do not become BREP |
| Sweep, loft, shell; arcs/splines; twist extrusion; face-selected pockets/counterbores; parametric arrays | No complete typed native route | Representative valid/degenerate cases, provenance, STEP round-trip | Open parity gates; unsupported requests must not fall back to generated Python |

The compatibility interpreter's method allowlist includes additional syntactic CAD methods (`model_program.py`). An allowed method name is not evidence that its full argument/selector combinations work, nor that a replacement exists. The six actual template states are blank, box, plate, bracket, enclosure and cylinder. Template parameter changes still use their existing route; introducing transform nodes does not silently rewrite old projects.

## User actions and typed contract

Add a modifier using the same Command API as other operations: `add_feature` followed by `set_body_source`, in one staged command batch. A rotation uses:

```json
{"type":"rotate","bodyFeatureId":"pad","axisOriginMm":[10,0,0],"axisDirection":[0,0,1],"angleDeg":90}
```

A mirror uses:

```json
{"type":"mirror","bodyFeatureId":"rotation","planeOriginMm":[0,0,0],"planeNormal":[1,0,0]}
```

Directions/normals are unitless and normalized by OCCT. Origins are world-space millimeters. Angles are explicit degrees, never stored in a length parameter's `valueMm`. The kernel copies the source shape. A mirror changes the body placement; to retain both original and reflected bodies, keep two named body outputs or explicitly union them. There is no automatic union.

`set_rotation` has `featureId`, `axisOriginMm`, `axisDirection`, `angleDeg`. `set_mirror_plane` has `featureId`, `planeOriginMm`, `planeNormal`. They modify an existing matching node without changing its ID or dependency. Both modifiers support suppression by passing through their original source. Changing a generic length with `set_literal` does not edit angle or plane coordinates.

Origins are finite and bounded to ±10,000 mm; direction components to ±1,000,000 with length at least 10⁻⁹. Rotation angles are finite and within ±360°. Zero and full turns are valid for an explicit repeated-pattern sequence. Invalid vectors, wrong source type, forward/missing dependencies and unknown fields fail before the worker. The C++ boundary independently rejects invalid geometry arguments with `INVALID_TRANSFORM`; the normal structured error boundary displays the failure while preserving technical details.

## Execution, persistence and verification

CAD IR validates the whole staged batch. Dependency traversal rebuilds required earlier features. The isolated worker calls dedicated C++ rotation/mirror modules through the bounded CXX boundary, validates BREP and emits STEP/GLB and checksums. The existing project transaction persists only a validated result. Source documents, modifier IDs and numeric values travel with revisions and full CADPACK projects; STEP retains geometry, not authored transform history. Preview or a rejected command batch does not mutate the original document.

`tests/rigid_transforms.rs` checks analytic box volume/area, eight-corner Rodrigues AABB, normalized arbitrary axes, offset origins, mirrored location through exact Boolean intersection, suppression, command rollback, wrong node types, NaN/infinity/near-zero/oversized vectors and angle bounds. The 16-tooth cog's expected volume comes from independently integrating the disk overlap with each rectangular tooth; it does not compare the kernel against itself. STEP reopening must retain volume, area, bounds, face count and edge count.

This work does not add angle expressions, editable parametric array counts, general persistent face/edge references, independent imported assembly components, or the remaining advanced operations. Cross-platform release tests and authenticated external AI scenarios are separate gates. See [commands](command-api.md), [modeling](modeling.md), [AI review](ai-candidate-review.md), and [architecture](architecture.md).
## Revolution of a closed profile

`revolve` builds a native BREP solid from a solved `sketch2d` profile, including inner closed loops. Add the node and set its body output in the same staged command batch:

```json
{"type":"revolve","sketchId":"profile","axisOriginMm":[0,0,0],"axisDirection":[0,0,1],"angleDeg":360}
```

The profile plane may be XY, XZ or YZ. The world-space axis origin is in millimeters and its direction is unitless. The axis must lie in the profile plane. The profile may touch the axis but cannot cross it; holes remain internal voids after revolution. The signed angle is nonzero and within ±360 degrees. The initial route accepts polygonal `sketch2d` profiles, not implicit rectangle/circle nodes or arcs/splines. A rectangle in an XZ sketch at radii 2–6 mm and height 10 mm creates a ring with volume 320π mm³; a 90° revolution produces one quarter of that volume.

`set_revolution` edits `featureId`, `axisOriginMm`, `axisDirection` and `angleDeg` without changing the sketch dependency or feature identity. Angles and directions are not length parameters. There is no suppression passthrough for this solid-producing feature. Preview and failed command batches leave the saved project unchanged. Revisions and full CADPACK retain the authored profile/axis/angle; STEP contains the resulting geometry only.

The shared bounded profile-face builder serves both extrusion and revolution. Rust validates field bounds and profile type, the C++ boundary independently checks coplanarity and axis crossing, then OCCT validates the resulting BREP before STEP/GLB publication. Errors use the structured `INVALID_REVOLUTION` boundary. `tests/revolution.rs` independently verifies analytic ring and internal-void volume/area, three planes, both full-turn signs, quarter turns, STEP reopening, rollback, invalid axes/angles/types and isolated-worker artifacts. `tests/sketch_profiles.rs` checks extrusion holes after the shared builder change. Sweep, loft, shell, curves, independent imported assembly components and general persistent topology remain separate parity gates.

See [imported STEP features](imported-step-features.md) for the verified input contract, storage and import/edit limitations.
