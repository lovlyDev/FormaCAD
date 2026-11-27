# Repository read contract

[Documentation](index.md) · [Русский](../ru/storage-contract.md) · [Formats](storage-formats.md)

`project_repository::backend::ProjectRepository` explicitly selects a managed
AppState/ID or a chosen v2 folder. There is no global current backend. This internal
API does not yet replace user-facing open/save or grant a writer lease.

| Operation | Managed | Folder v2 |
| --- | --- | --- |
| managed / folder | ID/path validation without writer opening | Pure metadata opening and retained OS identity |
| capture | SQL project/journal, folder, original ReadSession | Manifest/project metadata, opening token, OS identity |
| capture_with_history | Archive from the same SQL snapshot | Manifest already binds archive SHA; bytes stay lazy |
| read_asset | Exact descriptor, size/hash, original recheck | Bounded asset, size/hash, manifest/metadata, original opening |
| read_history | Requires capture_with_history | Explicit read/validation of the bound archive |
| recheck | Original handle/token, SQL/folder state | Original handle/opening, identity, manifest and metadata |

`RepositorySnapshot` is not serialized; backend variants and authority are private,
with only an immutable `project()` view. Another handle cannot accept its snapshot
even for the same ID. Managed archives return borrowed `Cow<[u8]>`, avoiding an
unnecessary second full copy. Metadata-only SQL snapshots reject archive reads
with existing `HISTORY_INVALID_STATE`; a fresh token cannot authorize an old result.

`FolderRepository` retains a distinct opening with a private UUID and active flag.
Close and Drop permanently invalidate snapshots. Reopen never reuses the token;
identical IDs across folders stay separate. Metadata reads load no unrelated
assets/history and create no files, locks, index, migrations or recovery. Explicit
asset reads reject damaged content; metadata does not certify that content.

Tests in `project_repository/{folder,backend}/tests.rs` cover location/backend/opening
isolation, close/reopen/drop, directory replacement, manifest drift, descriptor/hash
checks, unchanged sources and closure during SQL await. Current-head CI remains
required. These are optimistic capture/recheck observations, not exclusion of
external changes after recheck, NAS/USB guarantees or installed UI acceptance.

Prepare/publish, writer leases, explicit recovery and independent model writes are
still subsequent program work. Read tokens cannot provide them. The existing typed
history validator has been extracted for future writes; that does not authorize
bypassing CAD/permission checks or directly updating SQL.
