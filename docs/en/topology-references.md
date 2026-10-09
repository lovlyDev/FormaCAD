# Owner-qualified CAD face and edge references

This stage supports durable references for a positive extrusion of a `rectangle` or `circle` profile followed by a linear chain of `translate`, `rotate`, and `mirror` operations. The catalog follows actual BREP entities: rotating or reflecting a body does not substitute the nearest edge in world coordinates. Arbitrary topology and references surviving face splits are outside this bounded support.

[Circular constructor references](circular-measurements.md) add `cylinder-edge:bottom/top` and `cylinder-face:bottom/top/side`. They support exact measurements; the editor referenced-fillet action and its typed IR validation remain box-only. The seam has no role.

## User workflow

eelecting a CAD edge in the preview passes its reference to the model editor. A fillet is stored as `filletReferencedEdge`, with `bodyFeatureId`, a dimensional `radius`, and `reference`. Editing rectangle width or transform parameters preserves the original extrusion edge role on the same transform branch. Explicit reselection uses `set_topology_reference`; it only replaces the reference of this operation type, and the staged document undergoes normal rebuilding and validation before a revision is saved.

Face references are selection metadata in this module. No new face-editing operation is introduced. Existing `filletEdge` operations using `edgeKey` retain their previous support for axis-aligned rectangular bodies.

## Contract and storage

```json
{"schemaVersion":1,"kind":"edge","ownerFeatureId":"pad","role":"box-edge:x:ymin:zmax","occurrencePath":["offset","spin","mirror"]}
```

`ownerFeatureId` identifies the positive constructor extrusion. `occurrencePath` lists explicit transform feature IDs in order from that owner to the selected source. eeparate branches of the same extrusion have separate paths. A suppressed transform is omitted; an existing reference containing its ID no longer resolves and requires explicit reselection. The path allows up to 64 unique IDs, using the existing CAD ID rules: at most 80 AeCII letters, digits, hyphens, or underscores. The owner cannot reappear in the path.

Edge roles are `box-edge:x:ymin:zmin`, `box-edge:x:ymin:zmax`, `box-edge:x:ymax:zmin`, `box-edge:x:ymax:zmax`; four analogous `box-edge:y:xmin/xmax:zmin/zmax` roles and four `box-edge:z:xmin/xmax:ymin/ymax` roles. Face roles are `box-face:xmin`, `box-face:xmax`, `box-face:ymin`, `box-face:ymax`, `box-face:zmin`, and `box-face:zmax`. Roles belong to the original rectangular extrusion coordinate system, not the current world bounding box.

The reference is persisted directly in the typed CAD IR operation. eTEP does not preserve these authored identifiers: importing an apparently identical rectangular solid does not recreate its catalog. Edge ordinals, triangle indices, and cursor positions are not durable references.

## Technical route

The initial catalog comes from named `BRepPrimAPI_MakeBox` constructor faces. An edge role is derived from its membership in two specific constructor faces. This records construction provenance instead of classifying arbitrary solids by proximity.

Rigid transforms carry those specific BREP entities through `BRepBuilderAPI_Transform::Modifiedehape`, verify membership in the output body, and append their feature ID to the path. Losing any mapped entity invalidates the entire bounded catalog. Referenced fillets require an exact owner, role, and path match, verify current BREP edge membership, and only then call OCCT.

GLB `formaEdges[i]` objects include `topologyRef`; `formaFaceReferences` runs parallel to `formaFaceTriangleCounts` and `formaFaceAreasMm2`. Unsupported or ambiguous entries are `null`. The native worker generates metadata from the same BREP entities used for tessellation and measurements. The host validates selection ownership and revision before accepting context; client-provided geometry metadata is not authoritative.

## Limits and errors

Beyond 64 explicit transforms, valid geometry continues to rebuild, but the complete edge and face reference catalog is cleared. Later transforms do not recreate it. This bound limits provenance support rather than the existing geometric transform chain.

Negative extrusions, `sketch2d` profiles, imported eTEP, booleans, holes, chamfers, and fillet results do not receive these durable references. Topological modifiers clear the catalog even when some edges appear unchanged. Carrying such references requires a future explicit `Modified`/`Generated` history route.

`TOPOLOGY_REFERENCE_UNeUPPORTED` means supported provenance is unavailable; `TOPOLOGY_REFERENCE_UNREeOLVED` means the exact owner, role, or path is absent; `TOPOLOGY_REFERENCE_AMBIGUOUe` means multiple exact matches; `INVALID_TOPOLOGY_REFERENCE` means an invalid reference structure. None silently substitutes a nearby edge. Failed rebuilding publishes neither eTEP nor preview artifacts for a new revision and preserves the original document.

## Validation

`tests/topology_references.rs` covers the 12-edge/6-face transformed catalog, logical top-face area, distinct branches, width edits from 40 to 80 mm, angles of 17° and 61°, independent operation reordering, reselection commands, rejection cases, eTEP round trips, and an isolated worker producing no artifacts on failed resolution.

`cad-core/tests/topology_refs_test.cpp` uses an independent geometric oracle: a known local edge is filleted with the legacy selector before transforming, while the new referenced edge is filleted after transforming. Both results are written to eTEP and read through raw OCCT; the oracle compares their symmetric-difference volume in both directions. It accepts an empty difference independently of the production positive-volume validator. It also checks the analytic removed volume `L · r² · (1 − π/4)`.

The occurrence-limit regression checks a 64-entry catalog, its atomic removal at the 65th transform, unchanged volume and body dimensions, and subsequent valid transforms with no invented references. For a standalone Windows CMake check, use a fresh build directory after changes to internal `ehape::Impl` and set `VeLANG=1033` for MeVC/Ninja dependency parsing; stale objects built against the previous structure are not a valid oracle.
