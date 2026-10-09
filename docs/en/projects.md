# Projects, history, and transfer

[Documentation](index.md) · [Русский](../ru/projects.md) · [Using Forma](using-forma.md) · [Data](data.md)

A project is a local container for a model, revisions, and attachments. The dashboard shows cards with title, revision count, and a thumbnail of the current model. Search filters by title. A star pins a card near the top; cards animate when reordered. The thumbnail is captured from rendered geometry for its revision and changes when the model changes. Without renderable geometry, a neutral illustration appears. See [Dashboard](../../apps/desktop/src/app/Dashboard.tsx) and [thumbnail.ts](../../apps/desktop/src/lib/thumbnail.ts).

## Project actions

Create a project on the dashboard and open its card. Rename changes its title without destroying history. Duplicate creates an independent project for further work. Pinning changes only card order. Before deletion, Forma explains the consequences and requires you to **type the exact current project name**; confirmed deletion is irreversible. The project's records, revisions, and associated local files are removed. Binary attachments still referenced by another project remain until their last reference disappears. Export a `.cadpack` or copy the full data directory before deleting. See [projects.rs](../../apps/desktop/src-tauri/src/projects.rs) and [dialogs](../../apps/desktop/src/app/App.tsx).

## Revisions and restoration

Every saved state has its own revision; older revisions are not overwritten. History lets you inspect, compare, and restore a prior state. Restoration appends another revision, leaving earlier changes available. For native CAD IR, comparison uses stable IDs for parameters, features, and bodies; available stored exact metrics are also shown. If new geometry fails validation, no successful revision is published. See [modeling](modeling.md) and [comparison UI](../../apps/desktop/src/app/RevisionComparison.tsx).

## `.cadpack` and full backup

Exporting a project as `.cadpack` produces a portable package with a manifest, project description, history, and referenced attachments. Import validates the package and creates an independent project without replacing an existing one. Limits are 150 MB per package, 2,048 attachments, and 16 MB of project metadata. [cadpack.rs](../../apps/desktop/src-tauri/src/cadpack.rs) defines the format and checks. This transfers **one project**, not all global settings, installed CLI/Python paths, or other projects. For a workspace backup, close Forma and copy the whole [data directory](data.md), including SQLite and `projects/`.

Projects and preferences live under the application data directory with a stable identifier; installing a newer version over an older one should preserve it. In-app updates first create a SQLite backup. Directory locations, restoration steps, and state-persistence limits are documented in [data and recovery](data.md).

## Pure reads for copying and exporting

Copying a current managed project and exporting `.cadpack` use a [shared project/history snapshot](project-snapshots.md) from one SQL snapshot, with folder-generation checks and a recheck after attachment reads before publication. These operations do not implicitly recover the source. Reopen a project that requires recovery, then retry. Measurements retain their compatible error codes. Saving a new copy inserts a new project into shared SQLite; the source project remains unchanged, but the complete database file need not remain byte-identical. This changes the managed backend, not external-folder in-place opening or editable-history conversion of a genuine 1.1 project.
