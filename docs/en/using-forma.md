# Using Forma

[Documentation](index.md) · [Русский](../ru/using-forma.md) · [Current status](status.md)

This guide describes the **current local source**, not only the latest published installer. See [project status](status.md) for the version difference. Local startup prerequisites are in the [development guide](development.md).

## Getting started

Create a project on the dashboard or open an existing one. A project card shows its name, modification date, revision count, and a saved model image for its current revision. The star pins a project; pinned cards move to the front of the list with an animation. Search finds projects by name. Opening a card returns to that project's workspace. Code: [Dashboard](../../apps/desktop/src/app/Dashboard.tsx), [thumbnail](../../apps/desktop/src/lib/thumbnail.ts).

The workspace has files, model tree, history, and properties on the left; a 3D viewer in the center; and an AI assistant on the right. Panel widths can be changed. UI state is saved per project and restored on the next launch. The top bar provides search, settings, and export. [App](../../apps/desktop/src/app/App.tsx) connects these areas; the [workspace store](../../apps/desktop/src/stores/workspace.ts) holds active application state.

## Rename, duplicate, and delete

Card actions rename a project without changing its revisions or files. Duplication makes an independent project with the existing files and history. Deletion requires the **exact project name**, then removes the project, its revisions, attachments, and related local files. Shared binary attachments referenced by other projects remain until their last reference is deleted. Deletion cannot be undone; export a `.cadpack` or make a [backup](data.md) first. Implementation: [UI](../../apps/desktop/src/app/App.tsx), [project backend](../../apps/desktop/src-tauri/src/projects.rs).

## Files and history

A project can contain models and attachments. STL, OBJ, GLB, and 3MF open as meshes; STEP/STP is converted into exact geometry and a preview in a native desktop build. PNG/JPEG/WebP are stored as images; PDF and DXF as drawing attachments. Normal import is limited to 40 MB per file; external OBJ/GLB resources are not loaded. See [viewer and files](viewer-and-files.md) and the [import checks](../../apps/desktop/src/lib/files.ts) for format details.

History contains immutable revisions. Restoring an older revision **appends a new revision** instead of erasing later entries. Comparison shows changes to available parameters, features, bodies, and exact saved CAD metrics. After source or parameter edits, new geometry is built before a successful revision is exposed; a build failure keeps the previous result. See [modeling](modeling.md) and the [comparison view](../../apps/desktop/src/app/RevisionComparison.tsx).

Use `.cadpack` export on the dashboard to move one project to another computer, then import that file there. The bundle contains the project description, history, and referenced attachments; import creates a separate copy. It does not replace a full backup of all application preferences. Archives are limited to 150 MB and 2,048 attachments and are checked before import: [format and validation](../../apps/desktop/src-tauri/src/cadpack.rs). For a complete workspace transfer, see [data and recovery](data.md).

## Language, theme, and settings

Russian is the default; English can be selected. The choice persists, and dates and numbers follow the selected locale. Light and dark themes use their own surface and WebGL scene colors, with a purple accent in both. Settings also select the AI provider, the Python/CadQuery path for older models, and confirmation behavior. Code: [localization](../../apps/desktop/src/i18n/index.ts), [theme](../../apps/desktop/src/lib/theme.ts), [settings](../../apps/desktop/src/app/dialogs/SettingsDialog.tsx).

The browser preview (`npm run dev`) is useful for interface work but cannot launch local CLIs, use the native CAD worker, or install updates. Verify those actions in the desktop build. Continue with [modeling](modeling.md), [AI and permissions](ai.md), and [viewer and export](viewer-and-files.md).
