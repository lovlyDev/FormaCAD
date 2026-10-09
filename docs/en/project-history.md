# Undo and redo for model transactions

Undo and redo operate on complete successful model actions. One accepted AI model, one CAD command batch, applying several parameters, or restoring an older revision forms one step. Chat, renaming, thumbnails and export do not add model steps or clear redo.

## User workflow

Undo and redo controls are available for an open local native project when the corresponding action exists. Modification approval follows the shared permission settings. Undoing the first model returns an empty workspace while preserving saved revisions and files. Redo opens the saved geometry again. Building after undo starts a new logical line: the previously undone action is no longer available through redo, but its immutable revision remains available for ordinary restore.

If another operation changed the revision or a build/preview is active, undo cannot overwrite the newer model. Wait for that operation and refresh the current state. The browser demonstration without the native backend does not promise persistent undo/redo.

## Technical route

`project_history` separates its state machine, SQL persistence, portable archive and IPC. `history_status` returns `canUndo`, `canRedo`, and `expectedRevision`. `undo_model`/`redo_model` receive the project ID and expected revision, including `null` for an empty model. Under `AppState.writes`, they verify the expected revision and absence of a running task, then prepare a project copy and a private `PreparedHistory`. The `modify_project` approval is consumed only after checking saved artifacts.

Undo/redo of an existing model copy its parameters, source and exact STEP/GLB references into a new revision with a fresh UUID and the previous current revision as parent. Only `revisionId` changes in CAD IR, followed by document validation. BREP is not rebuilt: the operation reuses a previously committed immutable snapshot with SHA-256-verified files. Face/edge selection belongs to the current revision; old selection is not a valid reference for the new revision. Returning to the empty model sets the current pointer to `null` while retaining all old records.

`projects::persist_with_history` uses the same staged route as ordinary saves. The project payload, cursor, history event and recovery journal are written in one SQL transaction; the generation's history archive is staged before commit. A failure before commit rolls back all SQL changes. Publishing the committed generation and crash recovery are covered by the [project storage guide](project-storage-v2.md). An undo operation is never inferred from its prompt text: only a private prepared intent may move the logical cursor.

## Storage and compatibility

The new `0003_project_history.sql` migration adds `project_history` and append-only `project_history_events`; applied migrations remain unchanged. State records the original revision references for each step, cursor, actual current revision and generation. Each save verifies that the logical cursor snapshot matches the actual model, ignoring only its new CAD IR revision ID. Deleting a project removes its SQL history through foreign-key cascading.

For older projects without a cursor row, the initial line follows the parents of the current revision. Abandoned siblings are not guessed as redo actions. The generation's `history.json` uses its own format version 1 and carries state and journal without installation IDs, secrets, grants or system paths. Copies may change the project UUID while preserving revision UUIDs. Import checks the format, a 16 MiB size limit, up to 30,000 events, IDs, event continuity, current model and cursor. Foreign or corrupt history is rejected before committing the project; importing over an existing cursor is forbidden. A missing legacy archive permits reconstructing the original revision line, but cannot recover a lost redo cursor.

## Verification and limits

`tests/project_history.rs` covers an actual two-command CAD batch as one step, undo/redo of the entire batch, the initial empty state, branching after undo, immutable snapshots, metadata/chat saves without a new step, stale requests, preparation errors, SQL rollback after journaling, modified prepared snapshots and copying to a new project UUID. Archive checks include an invalid cursor and no committed imported project after rejection.

Undo does not remove an already exported external file or reverse an external AI CLI request. The project limit remains 10,000 revisions; new revisions created by undo and redo count toward it. Viewport navigation and an unsaved editor draft are not model transactions. Persisting the cursor does not replace the storage crash-test matrix or platform acceptance for 2.0.

### Portable journal integrity

Archive validation also checks that every undo/redo event materializes the declared target's full model snapshot, ignoring only the CAD IR revision ID. Its new revision must have the event's previous current revision as parent; a journal may not reuse a previously materialized revision. After the first legacy baseline event, ordinary edits obey the same parent continuity. Revision lookup is indexed, so validating bounded journals does not scan every revision for every event. Existing but incorrect target IDs, duplicate materializations, mismatched history checksums, missing descriptors, missing bytes and version/descriptor mismatches are rejected before import writes.

## Full and conversation-redacted project packages

`export_project_bundle` accepts optional `redactConversation`; omitted or `false` preserves the previous full-package behavior. When `true`, a separate `cadpack/redaction` module prepares a copy without messages, revision prompt text, export records (including external output paths), or the thumbnail. Native/mesh projects also exclude attachments not referenced by any retained revision's source, preview or program base. All revisions, their stable IDs and parent chain, named parameters, model programs and required geometry attachments remain unchanged; `history.json` retains the undo/redo cursor. Nothing in the original project is changed.

This option removes conversation metadata, not all sensitive information. Project names, CAD names/identifiers, parameter values, filenames, model programs and geometry may contain private information and remain in the package. Arbitrary legacy CadQuery scripts can refer to extra files by name: their attachments are retained because dependency inference would risk breaking the model. Inspect these sources and files before sharing. Package import does not execute legacy scripts automatically.

Tests export and import full and redacted ZIP packages, compare preserved attachment bytes and model parameters, perform undo/redo after importing the history into a new project UUID, and confirm that conversation, prompt, external-export-path and thumbnail markers are absent from the redacted archive. These checks concern archive integrity and saved model state, not anonymization or cross-platform geometry acceptance.

Imported STEP-as-feature dependencies are preserved by their verified SHA-256 references in every retained CAD IR v2 program, including older revisions. Undo/redo verifies these STEP/STP files as well as direct source/preview/program-base files before consuming modification approval.

## Pure reads for copying and exporting

Copying a current managed project and exporting `.cadpack` use a [shared project/history snapshot](project-snapshots.md) from one SQL snapshot, with folder-generation checks and a recheck after attachment reads before publication. These operations do not implicitly recover the source. Reopen a project that requires recovery, then retry. Measurements retain their compatible error codes. Saving a new copy inserts a new project into shared SQLite; the source project remains unchanged, but the complete database file need not remain byte-identical. This changes the managed backend, not external-folder in-place opening or editable-history conversion of a genuine 1.1 project.
