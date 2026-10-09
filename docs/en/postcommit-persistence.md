# Persistence after the durable commit

Project metadata and history commit in the same SQLite transaction. Before that commit, a validation, access, history, or staging failure remains an error. Immutable staging files written before an unsuccessful commit may remain unreferenced; they are not removed by temporary model-workspace cleanup.

After the SQL commit succeeds, the revision is saved. Publishing its folder manifest can fail independently. The storage journal and immutable generation remain available for reconciliation when the project is opened again; the application returns the committed project rather than reporting a failed model change. Cancellation after this boundary cannot roll back the saved revision.

## Mirrors and path protection

`projects/mirrors.rs` updates `project.json`, `workspace/model.py`, and `workspace/model.parameters.json` after the durable commit. These files are convenient mirrors; SQLite and the validated storage generation remain authoritative. Source filenames retain the existing compatibility convention even for typed CAD documents.

Every mirror path still passes through `security::guarded`. A missing root, rejected symbolic link, serialization error, failed write, or failed rename produces a diagnostic warning. A rejected path is skipped; the application never retries through an unguarded path. Failures of one mirror do not prevent attempts to update the others. Mirror errors do not delete storage generations, clear a pending publication journal, or restore a consumed permission grant.

Before publishing the manifest, the project path is checked again relative to the application root. Mirror paths are also checked relative to that root, including the project directory itself. A rejected publication path preserves the recovery journal. These checks do not claim an operating-system capability that eliminates every possible concurrent filesystem replacement race.

## Revision notifications

The existing `project://revision-created` notification follows a committed change of the current revision pointer. It therefore continues to notify existing consumers about undo and redo, despite its historical name. Saving messages, a project name, or other metadata with the same revision pointer produces no additional revision notification.

The model-apply host persists without an application handle and its IPC wrapper emits the committed outcome once. A subsequent AI assistant-message save does not repeat that event. A no-op or failed model apply produces no revision notification. This decision describes backend event intent; testing the pure decision or SQL transaction does not verify delivery inside an installed WebView.

## Validation route

The unit fixture uses temporary SQLite and managed project directories. A private callback compiled only for unit tests temporarily moves its own generation directory immediately after the actual SQL commit, making publication fail. The assertions require committed head and history, a retained recovery journal, and successful real reconciliation after restoring only that fixture generation directory. An actual assistant-message metadata save must then preserve the revision list and the single history edit. Separate event-decision assertions cover initial blank projects, new heads, unchanged metadata, undo, and redo.

An independent mirror-only fault replaces its temporary workspace directory with a junction (Windows) or symbolic link (Unix) to another temporary directory after successful manifest publication and before mirror updates. It checks that the external source sentinel remains unchanged and no parameter mirror is written there. It requires committed success, an already published generation with the new head, and no pending journal. This distinguishes a protected mirror error from a publication failure.

The callbacks are per request and absent from the production hook structure. No worker executable, callback, or arbitrary project output path can be supplied through IPC. These tests cover persistence semantics; they do not establish geometric validity, general fault recovery, or cross-platform installed acceptance.
