# Sketch diagnostics and solving

[Docs](index.md) · [RU/EN](../ru/sketch-diagnostics.md)

The [read-only analysis command](../../apps/desktop/src-tauri/src/sketch_analysis.rs) takes a typed sketch and named parameters, uses the same solver, and creates no revision, CAD files or SQLite writes. After solving, it validates the profile and returns coordinates, loop/hole counts, area and per-constraint residuals in millimeters.

[Numerical diagnostics](../../apps/desktop/src-tauri/src/cad_ir/sketch/analysis.rs) estimate the rank of the normalized Jacobian at the current configuration. Degrees of freedom equal coordinate count minus rank; redundant equations equal equation count minus rank. This is a local numerical estimate, not proof of a unique solution. Consistent redundant constraints are accepted; positive degrees of freedom do not invalidate a correct profile.

Conflicts return residuals and unsatisfied constraint IDs, but no solved points or degrees of freedom. Residuals do not identify a minimal conflicting subset. Structural errors and profile errors have distinct states. Sketch bounds remain in force; computation runs in `spawn_blocking`, without the kernel. Checks are described in [tests.rs](../../apps/desktop/src-tauri/src/cad_ir/sketch/analysis/tests.rs); these checks and the command tests passed locally on Windows.

In the desktop app, analysis starts 250 ms after the last sketch or parameter edit. Stale responses are hidden. The panel distinguishes fully constrained, underconstrained, conflict, invalid structure and invalid profile states. **Show solved sketch** switches only the 2D preview and disables dragging there. **Use solved coordinates** explicitly copies the solution into the draft and restores ordinary editing. Building still saves the new 3D model. Browser mode reports that native analysis is unavailable and does not substitute approximate results.

The [hook](../../apps/desktop/src/components/sketch/useSketchAnalysis.ts) debounces edits, skips browser requests and ignores old request completion. [Lifecycle tests](../../apps/desktop/src/components/sketch/useSketchAnalysis.test.tsx) exercise debounce and response races. Preview stays in editor state; analysis does not change the project, SQLite or 3D scene.

[Command tests](../../apps/desktop/src-tauri/tests/sketch_diagnostics.rs) check the 1104 mm² two-hole area, invalid-profile state, unchanged operation and rejection of duplicate/nonfinite parameters.
