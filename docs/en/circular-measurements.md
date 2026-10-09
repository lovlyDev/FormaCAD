# Circular constructor references and exact measurements

[Documentation](index.md) · [Русский](../ru/circular-measurements.md) · [Exact measurements](reference-measurements.md)

This module extends operation-owned selection to a positive `circle` extrusion. Select its upper or lower CAD edge, open **Exact selection measurements**, choose length, radius or diameter, and measure the selection. Length is the circumference of the actual circular BREP edge. Radius comes from OCCT's exact circular curve; diameter is twice that radius. Project display units apply to all three lengths.

The upper and lower faces support area and planar-face measurements. The curved side supports area, and is explicitly rejected as a planar face. The longitudinal seam has no authored reference. These choices do not classify arbitrary mesh geometry as a circle.

## Ownership and rebuilding

The constructor assigns `cylinder-edge:bottom`, `cylinder-edge:top`, `cylinder-face:bottom`, `cylinder-face:top` and `cylinder-face:side` to its named OCCT subshapes. `ownerFeatureId` identifies the extrusion. `occurrencePath` identifies the exact tracked translation, rotation or mirror branch. The catalogue resolver checks actual membership in the selected body and rejects missing or ambiguous references.

Changing the authored circle radius or extrusion height rebuilds the same roles. A tracked rigid transform carries the actual edges and faces, preserving the roles in the constructor's coordinate system. The lower face's outward normal points away from the solid; mirrored normals use oriented surface derivatives rather than a plane axis alone.

The new query is `{ "kind": "edgeDiameter", "reference": ... }`, and its typed result is `{ "kind": "edgeDiameter", "diameterMm": ... }`. Existing query tags remain unchanged; the native diameter tag is 5. Strict field validation, independent request identity, report hashes, source seals, queue checks and stale-result rejection follow the [measurement lifecycle](reference-measurements.md).

## Storage and limits

Measurements add no model revision, history step, database pin or persistent report. Disclosure preferences retain the existing project preference storage. Worker staging uses owned temporary directories; original STEP files, project data and previous exports remain intact.

Negative extrusion, general `sketch2d` circular profiles, imported STEP, Boolean results and topology-changing modifiers do not receive this constructor catalogue. Radius and diameter of straight edges are unsupported. Circular selection does not enable the box-specific referenced-fillet editor action. Arbitrary topology remapping and pinned measurement refresh remain separate work toward 2.0.

## Verification

Local checks passed 13 standalone C++ suites, 260 native tests (two service helpers ignored), strict Clippy, non-native compilation and formatting. All 172 frontend tests passed; the final hint wording change passed its three component tests and lint. Coupled Chromium passed 20 scenarios in RU/EN and both themes. The new scenarios use actual viewport clicks and seven genuine isolated worker reports through test-only IPC. A first test chose a cap triangle less than a pixel from the silhouette; selecting an interior test point fixed the test without changing production picking. These are prior development checks; final verification of the new consolidated installer is recorded in the [release notes](../releases/1.2.6.md) after packaging. Installed WebView and general topology are not established by these checks.

The exact worker bundled in the installer passed 20 analytic circular measurements and two curved-side-plane rejections, plus eight box measurements and three rejected box queries. Installer integrity checks confirmed exactly one Cargo-built worker, 25 matching OCCT DLLs and unchanged hashes for all 16 older installers. No publication, installation or user-project writes occurred.

A limited speed comparison used five alternating warmed processes of the two verified release workers on an R25.4 × H10 circle extrusion with STEP/GLB. Development baseline versus updated worker medians: 93.0846 ms and 90.5685 ms. This small case does not establish faster large-model or installed-app behavior. Every sample checked analytic volume and export hashes.
