# Pure project catalog projection

[Documentation](index.md) · [Русский](../ru/storage-catalog.md) · [Contract](storage-contract.md)

`project_repository::catalog` is a backend API for enumerating saved managed
projects without recovery or write access. IPC `list_projects` returns CatalogReport and `read_project_snapshot` reads one project without recovery. Dashboard still uses a strict compatibility adapter: any unavailable row raises a localized error, without silently filtering entries. Unavailable cards and workspace/model-apply target-read integration remain pending.



## Contract

`list_managed(state, CatalogLimits { max_entries, max_total_payload_bytes })`
returns every selected row in `CatalogReport`, together with the actual JSON-byte
sum of ready `Project` payloads. The host explicitly selects limits; there are
no product defaults. Too many rows fails the whole call rather than returning
a silently truncated successful list. Global SQL failures remain errors.

| Entry | Data | Meaning |
| --- | --- | --- |
| `ready` | Complete saved `Project` | One pure repository snapshot and original-session recheck |
| `unavailable` | Bounded index hints and issue | No invented model substitutes for the real project |

Issues distinguish `recovery_required`, `changed`, `unavailable` and `limit`.
Index hints are not verified model state: invalid IDs or dates do not become
authority. `rowKey` is a `managed-index:{rowid}` string, preserving uniqueness
even above JavaScript's `2^53`. Unavailable rows are not omitted, and healthy
projects remain alongside them. Tokens/openings/leases are never serialized.

## Technical route and limits

A bounded index SELECT reads ID, name, date and payload length rather than all
payload strings. Each admissible row uses `ProjectRepository::managed → capture
→ recheck` of its original snapshot. There is no `projects::get`, reconcile,
writer acquire, migration, index write or pending-journal cleanup. These are
per-project optimistic checkpoints, not an atomic snapshot of the whole catalog.

Raw payload length supplies conservative admission: whitespace-heavy input can
be refused before its shorter canonical serialization. A bounded writer counts
actual ready-project serialization without another complete JSON buffer; a stale
SQL length hint cannot bypass the output budget. Original-source recheck precedes
cloning the admitted `Project`.

The byte budget covers ready Project JSON only, excluding envelopes/index hints,
and is not a peak-memory limit. A raced row still uses the existing bounded
repository capture/recheck working set (up to 128 MiB raw) plus an admitted clone.
Ready entries retain chat, revisions and files for future UI compatibility;
this is not a lightweight metadata-only index or a large-list performance proof.

## Integration and verification

UI integration requires an agreed shared patch for unavailable cards, workspace
preservation and explicit recovery. The current Dashboard also saves thumbnails
automatically: replacing backend listing alone does not make the screen pure.
`PROJECT_CATALOG_LIMIT` and `PROJECT_CATALOG_UNAVAILABLE` are internal for now;
add RU/EN translations at the display boundary before wiring them to UI, keeping
technical details separate. Shared `Project` and intentional `projects::get`
opening behavior remain unchanged.

Nine SQL regressions cover mixed ready/pending/corrupt rows, unchanged
SQL/source/journal bytes, read-only without upgrade, limits, escaped JSON,
size races, invalid IDs, close during SQL await, global database failures and
rowids above `2^53`. Reproduction:

`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --no-default-features --lib project_repository::catalog`

Test presence is not PASS; new-head execution is recorded separately in CI.
Fixtures are temporary and synthetic. Real NAS/USB and user-facing UI acceptance
are unverified.


## Initial IPC contract and remaining integration

The host limits responses to 1000 entries and 128 MiB of ready JSON, not peak memory. `projects/catalog.rs` binds the pure catalog and target read to IPC without changing intentional `projects::get`, persist or recovery. Target reads grant no writer lease and serialize no session token. The new frontend API is not yet connected to App. Existing Project[] callers reject a partial catalog as a whole, retaining the report in PartialCatalogError. This is transitional integration, not a completed user catalog; App still automatically saves thumbnails on home. The next patch must display unavailable cards, retain the workspace on failures and remove background writes.
