# Sketch construction lines

[Documentation](index.md) · [Русский](../ru/sketch-construction.md) · [Sketches](sketches.md)

## Purpose and workflow

In local version 1.2.6 I added straight construction lines. They help dimension distances between points, such as a rectangle diagonal. A guide participates in constraints but adds no edge or face to the extruded solid. Construction lines may cross the profile.

Open the model source editor → **Edit 2D sketch**, choose the start and end points, then click **Add construction line**. Equal endpoints and the line limit disable the button. The guide appears dashed; its ID and endpoints appear below the controls. Add length, horizontal or vertical constraints in the line list. A diagonal dimension together with a fixed width can drive the profile height. Build the model to solve and save the result. The SVG displays draft coordinates, rather than a solved preview.

**Remove construction line** removes its own constraints. Points shared with the profile and other line constraints remain. If an imported document has points used only by the removed guide, those points and constraints referencing them are also removed. A failed build preserves the latest successful revision and geometry.

## Format, execution and storage

In CAD IR v2 a line stores optional `construction: true`. Omission means a profile edge, so older documents load without migration. Point, line, feature and body IDs remain stable; the schema stays at 2. The regular build path stores JSON in `Revision.program`, so revisions and `.cadpack` include the guides. Settings, SQLite and project storage do not change. Application versions before 1.2.6 do not support the new field.

The [editor](../../apps/desktop/src/components/sketch/SketchConstructionEditor.tsx) calls [draft editing helpers](../../apps/desktop/src/lib/sketchConstruction.ts). Source edits and AI documents use the same [CAD IR validation](../../apps/desktop/src-tauri/src/cad_ir/validation.rs). [Structure checks](../../apps/desktop/src-tauri/src/cad_ir/sketch/structure.rs) validate IDs, references, limits and directed closed loops of profile lines. The [solver](../../apps/desktop/src-tauri/src/cad_ir/sketch/solver.rs) uses constraints on all lines. The [sketch executor](../../apps/desktop/src-tauri/src/cad_ir/sketch.rs) selects only the profile after solving, while [outline checks](../../apps/desktop/src-tauri/src/cad_ir/sketch/outline.rs) reject self-intersections. OpenCascade receives only the profile to build B-Rep and export STEP/GLB. No additional kernel or SolveSpace dependency is introduced.

The architecture follows the Construction Geometry section of the [SolveSpace reference](https://solvespace.com/ref.pl): constraint geometry is separated from the manufacturing profile. Forma implements this independently without copying reference code.

## Limits and verification

A sketch allows at most 32 points, 64 lines and 64 constraints. Every point must belong to at least one line. Profile lines form one outer boundary with up to seven non-touching inner holes; see [profiles](sketch-profiles.md). Guides can share points or have independent endpoints, but the UI creates them between existing points. Separate points can be entered in JSON. Zero-length solved lines, missing references, open profiles and incompatible dimensions are rejected. Toggling a profile edge into construction is not yet exposed in the UI.

[Rust unit tests](../../apps/desktop/src-tauri/src/cad_ir/sketch/tests.rs) cover driving height through a diagonal, independent guide endpoints, profile crossings, the legacy format, invalid references, limits, profile topology and dimension conflicts. [Worker tests](../../apps/desktop/src-tauri/tests/sketch_construction.rs) check exact volumes of 1000/3000 mm³, no diagonal B-Rep edge, and no STEP/GLB on conflict. [Frontend tests](../../apps/desktop/src/lib/sketchConstruction.test.ts) check JSON, stable IDs and constraint cleanup; browser verification covers both languages and themes.

## Example model

The [CAD IR JSON example](../fixtures/sketch-construction.cad.json) uses named parameters for a 20 mm width and a √1300 mm diagonal. Constraints solve a 30 mm height; a 5 mm extrusion yields 3000 mm³. Copy the JSON into the source editor and build with the native application. Changing the diagonal drives height; a diagonal shorter than the width cannot satisfy the constraints and does not save a revision. The worker test uses this same fixture, so the example is verified by the real kernel.

On this Windows PC, a local comparison of 5000 solves in each of three rounds measured 0.016–0.028 ms for the development baseline solver and 0.016–0.020 ms after splitting on the same rectangle; a diagonal guide took 0.022–0.025 ms. This solver microbenchmark excludes worker startup, OpenCascade and export. The variation does not establish a speedup, but no material slowdown was observed for this scenario.

Profiles also support inner holes; see [current profiles](sketch-profiles.md), [named parameters](cad-parameters.md) and [solver diagnostics](sketch-diagnostics.md).
