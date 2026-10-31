# Pure project catalog projection

[Documentation](index.md) · [Русский](../ru/storage-catalog.md) · [Contract](storage-contract.md)

`project_repository::catalog` is a backend API for enumerating saved managed
projects without recovery or write access. IPC `list_projects` returns CatalogReport and `read_project_snapshot` reads one project without recovery. Dashboard consumes the full report, including unavailable cards; workspace/model-apply/events use pure target reads.



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
Ready entries retain chat, revisions and files for UI compatibility;
this is not a lightweight metadata-only index or a large-list performance proof.

## Integration and verification

The managed catalog UI consumes the report described below. Catalog-limit and unavailable errors have RU/EN translations; technical details remain separate. Shared Project and intentional projects::get behavior remain unchanged.

Nine SQL regressions cover mixed ready/pending/corrupt rows, unchanged
SQL/source/journal bytes, read-only without upgrade, limits, escaped JSON,
size races, invalid IDs, close during SQL await, global database failures and
rowids above `2^53`. Reproduction:

`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --no-default-features --lib project_repository::catalog`

Test presence is not PASS; new-head execution is recorded separately in CI.
Fixtures are temporary and synthetic. Real NAS/USB and user-facing UI acceptance
are unverified.


## Initial IPC contract and remaining integration

The host limits responses to 1000 entries and 128 MiB of ready JSON, not peak memory. `projects/catalog.rs` binds the pure catalog and target read to IPC without changing intentional `projects::get`, persist or recovery. Target reads grant no writer lease and serialize no session token. Remaining legacy Project[] callers retain the strict compatibility adapter, rejecting partial reports rather than hiding rows. Dashboard itself consumes the report directly.


## Catalog in Dashboard

Dashboard reads CatalogReport and displays both ready projects and separate unavailable cards with status, bounded index hints and pure-read retry. String rowKey values never pass through numeric conversion. Unverified rows receive no fabricated model, edit/export/delete action or automatic-recovery button. Technical details are expandable separately. Search includes an unavailable row’s name/ID hint.

Listing no longer creates or saves thumbnails in the background; saved images or the default icon are displayed. Card and direct-route opening read only the selected project through read_project_snapshot; unrelated unavailable rows cannot block a healthy target read. During a direct-route target read, the previous scene and its own UI persistence namespace remain visible with model editing blocked. If reading fails, the active workspace is preserved and its URL is restored; no scene reset or redirect home occurs. A shared generation and original-workspace comparison reject late results after another choice or local update. Model-apply and revision events also use target reads while retaining their existing guards. Catalog and Project[] snapshot caches stay distinct; accepted mutations/events invalidate both using one helper. Pin and rename first read the current target before calling the existing writer API.

This is the managed catalog, not an independent external-folder writer. Explicit recovery UI remains a separate integration; Retry performs no automatic repair.

UI check on 31 October 2026: build and lint passed; 227 unit tests across 79 files passed with one worker. The parallel run hit an existing Sketch2d test timeout; the complete suite was rerun serially. The [browser report](../storage-catalog/ui-check.json) covers all four statuses, search/retry/details, string row keys and RU/EN with both themes on one mount without changing fixture project storage. [RU dark](../storage-catalog/ru-dark.png) and [EN light](../storage-catalog/en-light.png) screenshots were inspected. This is the actual Dashboard with synthetic props, not native IPC/recovery or CAD-scene preservation acceptance.
