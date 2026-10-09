# Sketch profiles and holes

[Docs](index.md) · [RU/EN](../ru/sketch-profiles.md) · [Sketcher](sketches.md)

CAD IR v2 profile lines may form up to eight closed loops: one outer boundary and up to seven holes. Construction lines stay outside the profile. Total bounds remain 32 points and 64 lines/constraints. No new contour field is needed: topology follows stable start/end point IDs. Older single-loop sketches load without migration.

[Loop parsing and classification](../../apps/desktop/src-tauri/src/cad_ir/sketch/profile.rs) check each polygon, intersections and nesting. The outer loop contains every hole; holes cannot touch each other or the boundary, or nest inside other holes. Disconnected islands require separate bodies. Outer winding is normalized counterclockwise, hole winding clockwise. The [OpenCascade builder](../../cad-core/src/profile_prism.cpp) constructs one planar face with inner wires and exact extrusion in XY/XZ/YZ, including negative distances. B-Rep is checked before export. JSON uses the regular revision save path; SQLite is unchanged.

Classification tests are in [profile_tests.rs](../../apps/desktop/src-tauri/src/cad_ir/sketch/profile_tests.rs), kernel tests in [profile_test.cpp](../../cad-core/tests/profile_test.cpp). Classification, native worker and all nine C++ core tests passed locally on Windows.

Expand the sketch editor, enter contour X/Y, width and height, and select **Add rectangle contour**. Coordinates use the workplane. Place it strictly inside the outer boundary to make a hole. The new contour has four points, four edges and horizontal/vertical constraints. Change point coordinates, split edges or remove vertices using the [editing guide](sketch-constraints.md). Removing a contour requires another contour; attached construction lines are also cleaned up. Removing the outer boundary may invalidate remaining geometry; inspect diagnostics before building.

The [two-hole example](../fixtures/sketch-holes.cad.json) has a 40×30 mm outer rectangle and two 8×6 mm holes: 1104 mm² area and 5520 mm³ volume at 5 mm extrusion. [Native integration](../../apps/desktop/src-tauri/tests/sketch_profiles.rs) checks volume and STEP/GLB in all three planes and both directions, and no export when a hole crosses the boundary. Touching contours, intersections, nested holes and disconnected islands are rejected. Arcs and circles remain unsupported.
