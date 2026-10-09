# Project folder storage v2

[Documentation](index.md) · [Русский](../ru/project-storage-v2.md) · [Architecture](architecture.md)

The local backend now saves verifiable, self-contained project folders. **Storage format** `storageVersion: 2` is separate from the compatible `Project.schemaVersion: 1` payload and CAD IR v2. It does not declare application 2.0 readiness.

## Contents and transfer

A folder contains `manifest.json`, `assets/<sha256>`, `metadata/generations/<uuid>/project.json` and `history.json`. The manifest names the active generation and explicitly lists files, sizes and SHA-256 values. Project metadata contains immutable revisions, current revision, CAD sources, messages, metadata and attachment references. History contains the logical undo/redo cursor and transition log. CLI credentials, machine permission grants and application settings are excluded.

Model files live in the project's local content-addressed `assets`, rather than depending solely on global `.blobs`. Chat, name and thumbnail saves reuse unchanged assets instead of copying large attachments into every generation. Reads verify checksums. Copying the entire folder preserves the data needed for independent opening without the original SQLite database or global cache. Existing `attachments`, `output`, `workspace` and `.blobs` remain for compatibility; existing user files are not deleted.

Backend `projects::import_folder` reads a selected absolute directory, verifies declared files and creates a local indexed copy. The source folder is read-only. An existing project identity returns a conflict; explicit `save_copy` assigns a new project UUID while preserving revision UUIDs and the undo/redo cursor. This is a copy import, not in-place editing of an external folder. The explicit IPC route is `project_storage_v2::ipc::inspect_project_folder` → review → `import_project_folder`. The native picker returns project name, identity, revision count, existing-index identity status and the SHA-256 of the manifest used for inspection. Import checks that exact manifest before reading its generation and command history; changed source returns `PROJECT_FOLDER_CHANGED`, without changing the source or index. Each captured asset and history file is rechecked while hydrating. `saveCopy` explicitly controls project identity conflict handling. The dialog uses the product name `Forma CAD`; application review labels and errors are translated at the UI boundary. This copies into the managed project root; it does not choose a new storage root or establish an in-place edit lease.

## Atomic save and recovery

1. Acquire an OS advisory lock on `metadata/storage-v2.lock`. A second writer is rejected; process termination releases the lock without deleting another owner's file. PID and acquisition time are recorded.
2. Verify attachments and store them in CAS. Write a fresh immutable metadata generation using `create_new` and `sync_all`. Partial generations are not visible through the active manifest.
3. One SQL transaction saves the project index, undo/redo state, action history and `project_storage_commits`, including the original manifest hash, target manifest and project payload hash.
4. Following SQL commit, atomically replace the active manifest through a temporary file. Preserve the previous verified manifest in `metadata/manifest.previous.json`.
5. Remove the pending journal after publication. A crash after SQL commit is recovered on the next read by publishing **only the journaled** generation. External manifest changes, differing payloads or corrupt files fail instead of silently overwriting data.

`read_backup` independently verifies the previous snapshot. The backend never silently substitutes an older backup for the current model. Failure before SQL commit leaves the prior generation active. Publication failure after SQL commit retains a completion journal; callers must not repeat the operation as though nothing was saved.

The first save of an existing project materializes its original payload into a verified baseline generation before transition. Original database records and legacy files remain. This packages existing document state; it does not recover missing STEP/STL operation history or parametrically convert previous 1.1 projects.

## Boundaries and protection

Manifests reject unknown fields/versions, duplicate paths, traversal, symbolic links and unsafe hashes. Per-file bounds are 40 MiB for an attachment, 32 MiB for metadata and 16 MiB for command history, with a 148 MiB total manifest bound. Undeclared files are not imported. Required attachments are retained; selective exclusion of private data is not implemented yet.

SQLite remains the compatibility index and part of the local commit protocol. Folder independence is demonstrated for reading and copy import. Direct NAS/USB editing, heartbeat, local recovery drafts after device loss, and full SSD/NAS storage-format benchmarks remain separate work. OS locks do not guarantee consistency in cloud-synchronized folders. Assets no longer multiply on metadata-only saves, but checksum verification still reads their bytes; incremental verification caching is not implemented. Old generations and orphan staging files are retained; safe retention/garbage collection is not implemented. `sync_all` and atomic rename do not claim protection against hardware power loss on every filesystem.

## Validation

[Isolated tests](../../apps/desktop/src-tauri/src/project_storage_v2/tests.rs) use temporary directories and test SQLite only: moving a folder without global database/cache, CAS reuse, a crash between SQL and manifest, conflicting external changes, corrupt current generation with readable backup, unsafe paths/versions/bounds, attachment migration failure, OS lock release, and identity-conflict/copy import preserving the source. Dedicated history tests cover portable cursor and action log. User AppData, installation and GitHub are not used for these checks.

[Session access](project-access.md) explains persistent same-root ownership and read-only copy creation.
