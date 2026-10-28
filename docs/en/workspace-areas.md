# Areas, workspaces and editors

[Documentation](index.md) · [Русский](../ru/workspace-areas.md) · [Projects](projects.md) · [Settings](settings-and-updates.md)

A project workspace is a tree of editor areas. Model shows the model tree, files, viewport and AI. Inspect adds history and properties; Sketch retains the viewport and opens parameters for the existing operation editor. These presets rearrange panels without modifying geometry.

## Arranging editors

Choose an editor in each area header: viewport, model, files, parameters, AI, properties, history, tasks, activity or logs. Each editor has one live instance. Choosing an editor already open elsewhere swaps the editors. An existing viewport keeps its state during the swap.

Split buttons add an area to the right or below. Join removes the selected area and gives its parent rectangle to the neighboring subtree. The last area cannot be removed. Drag separators, or focus one and use arrows for 2% steps, Shift+arrows for 10%, and Home/End for the permitted limits. Each area has a minimum of 180×120 CSS pixels; nested split minima accumulate. The workspace scrolls when the window is smaller than the minimum tree.

Maximize an area using its header or Ctrl+Space; repeat to restore. F6 and Ctrl+Tab move focus between areas, with Shift reversing direction. Tab reaches controls inside areas. Text fields retain their own keyboard behavior.

Workspace tabs switch presets, create a copy of the current arrangement, rename it or reset its layout. User names are never translated. Each project supports up to 20 workspaces and 10 areas, one for each live editor. Bottom buttons open or close history, tasks, activity and logs.

Project tabs switch between opened projects. Closing a tab never deletes its project. Camera, message draft and area settings are keyed by project ID and reused on reopening. Existing restrictions during active operations remain in effect.

## Model tree, properties and AI

The model tree shows bodies and available typed operations, searchable by name and ID. The eye changes visibility only. The lock prevents picking a body in the tree and viewport; it is an interaction setting, not a CAD edit lock or project-access permission. Selecting an operation supplies its context to Properties when that editor is open. Double-click opens the existing parameter/operation editor, with the usual reviewed build path.

Selecting geometry does not replace AI with Properties. Choose Properties in any area header or use Inspect. Existing exact measurements and their limits remain: [reference measurements](reference-measurements.md), [pair measurements](pair-measurements.md). While a new model is loading, properties use the displayed committed geometry.

F3 opens the command palette; Ctrl+K remains available. Search includes existing commands, settings pages, editors, projects, typed-document bodies/features/parameters/revisions and enabled plugin actions. Arrows and Enter choose a command or project; clicking executes a result. Up to 100 matches are shown; refine the query for a large list. Entity search opens the relevant editor without modifying geometry. Command shortcuts use physical key codes and work with Russian layouts.

## Viewport panels and settings

T toggles viewport tools and N toggles the sidebar while the viewport editor has focus. Header buttons provide the same actions. The sidebar shows the selected element's display transform, view actions and existing plugin actions. Display transforms are explicitly distinguished from exact B-Rep properties.

Interface settings control density and text size. Input enables number-row numpad emulation. Navigation controls orbit around selection, zoom toward the pointer and zoom/pan sensitivity. Pointer zoom uses a plane through the orbit center, not an exact CAD snap. View settings do not create revisions. Theme changes apply immediately to the interface, WebGL background and grid.

Tasks uses the existing CAD queue and cancellation API. Activity shows recorded project events; Logs shows recorded project errors. These editors do not read full host diagnostic logs, which remain local.

## Technical route and persistence

The [tree model](../../apps/desktop/src/features/workspace-areas/layoutModel.ts) validates `layoutVersion: 1`, size, depth, IDs, editor types and ratios. Corrupt or future layouts display a safe preset; unknown data never executes code. [Layout geometry](../../apps/desktop/src/features/workspace-areas/layoutGeometry.ts) computes rectangles and recursive minima. The [shell](../../apps/desktop/src/features/workspace-areas/WorkspaceShell.tsx) renders editor instances in a flat list with stable keys: changing the split tree does not change their React parent or remount Canvas. Previously opened hidden editors stay mounted until the project closes.

Layout uses the existing [UI preferences route](../../apps/desktop/src/lib/persistence.ts), keyed as `forma.ui.project.<id>.areas.v1.workspaces` and `.active`. Camera, T/N panels, selection locks, navigation and project tabs have separate keys. Desktop mirrors preferences through `set_ui_preferences`. This is application-profile storage keyed by project, not a new portable project format; exporting layout through cadpack is not claimed. SQL, CAD IR, worker protocol and model history are unchanged.

Existing geometry editors still follow typed IPC → Command API → staged rebuild → worker/OCCT → validation → committed revision. Area, visibility, theme and focus changes do not run that transaction. Project and plugin permissions remain enforced.

## Validation and limits

Unit tests cover layout validation/round-trip, instance identity, bounded growth and nonoverlapping geometry with minima. Browser scenarios exercise split/join, editor switching, maximize, keyboard control, persistence, RU/EN and both themes; they compare the canvas object, WebGL2 context, context-loss state and camera after layout/theme changes. This is Chromium evidence, not installed WebView or macOS/Linux acceptance.

This change does not implement modal G/R/S, F9, node editing, Lua, new CAD operations or plugin-api 1.0. Application version remains unchanged in this task PR; a designated release owner handles versioning and release separately.
