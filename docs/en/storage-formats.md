# Storage format comparison

[Documentation](index.md) · [Русский](../ru/storage-formats.md) · [Program](storage-program.md)

The independent folder backend format has not been selected. Candidates are the
existing v2 manifest/generations representation and experimental project-local
SQLite. The SQL candidate now uses minimal project/history tables and typed history; the global app
database does not supply its synthetic data. This is a benchmark, not a user-facing
open/save route. The saved first baseline used the app schema; it remains separate
and does not establish the corrected pure opener's behavior.

The corrected `sqlite-delete-prototype` uses DELETE rollback journal and FULL
synchronous. Explicit create refuses existing files. A reader checks a bounded
SQLite header, application_id/user_version and absence of WAL/SHM/journal before
connecting read_only/query_only and running quick_check. It never uses immutable,
applies migrations or repairs a journal. Live-WAL purity remains unsupported.
This is a benchmark prototype, not a production writer API. Four regressions
cover byte preservation, absence of app tables, write refusal and controlled
foreign/future/WAL format rejection. Writer startup is timed separately from
metadata save.

## Read boundaries

New `project_storage_v2::reader::FolderSnapshot` captures manifest/project.json,
checking bounded lengths, hashes and metadata references. It does not load unrelated
assets or the event archive. `read_asset` accepts an exact captured descriptor and
checks regular-file type, length, SHA, original manifest and metadata. Explicit
`read_history` validates the archive and its references. `recheck` also detects
project.json corruption with an unchanged manifest. Readers never repair or modify
the source. Missing assets do not prevent metadata capture; explicit asset reads
still fail on missing or damaged content.

This is a saved-byte primitive. A repository adapter must separately retain the
original lifecycle token and OS location identity. Existing `read_folder` retains
full-entry validation. The v2 cursor lives inside the history archive: scanning
that archive is not presented as constant-time history status.

## Reproduction and evidence

`cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --no-default-features --lib project_repository::benchmark::compare_storage_representations -- --ignored --nocapture`

Small/medium/large fixtures have 4/32/128 persisted revisions, chat and repeated
and unique attachments. STEP-labelled payloads are synthetic and establish no CAD
geometry validity. Real Project/history/manifest validation and limits apply.
Raw `STORAGE_BENCHMARK` JSON records have no performance thresholds. A single
observation does not establish a latency distribution.

The Storage comparison workflow saves benchmark.log, CPU/OS/disk metadata and
process-resources.txt. Resource totals include compilation and the entire test
process, not the peak memory of an individual operation. First API open is not
an OS cold-cache or new-process measurement; those gaps are explicitly recorded.
Unsupported complete copy/export/crash recovery routes receive no invented timings.
Final raw results and a format decision require actual execution.

### First executed comparison

[Storage comparison 40867790068](https://github.com/lovlyDev/FormaCAD/actions/runs/40867790068)
succeeded for head `250016c6c9d69b12d7016bf5341a504603b685da`;
the PR merge checkout was `ed96fbffbbb85f959b8db2abb8fa8d7cbba0a500`.
Saved evidence: [raw JSON](../storage-benchmark/250016c-linux.jsonl),
[environment](../storage-benchmark/250016c-environment.txt) and
[whole-process resources](../storage-benchmark/250016c-process-resources.txt).
Ubuntu 22.04 / Linux Azure, AMD EPYC 7763, 4 vCPU; virtual disk with lsblk ROTA=1,
physical storage type unverified. Rust 1.97.1.

| Large fixture, single observation, ms | Full manifest v2 | Metadata reader v2 | SQLite candidate |
| --- | ---: | ---: | ---: |
| First API open | 526.348 | 16.335 | 7.772 |
| History status | 1054.811 | 31.048 | 0.936 |
| Metadata save | 2109.459 | No separate writer | 14.922 |

Large has 128 revisions, 256 references to 128 unique 128-KiB assets, a 32355-byte
archive and 133911-byte metadata. Saving again retained exactly 128 assets.
Validation work differs: SQL status does not hash all assets/archive; SQL opening
uses existing migration/recovery and is not yet a pure-reader adapter. These are
costs of actual primitives, not a winner with equivalent guarantees. Process/cache
cold open, full copy/export, crash recovery and per-operation peak memory remain
unmeasured. This run does not verify later heads or complete stage 2.

Strict CI on `147a0dd` failed at benchmark export_history's explicit_auto_deref;
the original failure remains in repository job 123427829406. Fix `93956b1` removes
the unnecessary dereference without weakening lint.

## Shared integration

The [measurement contract and exact patch](../integration/storage-measurement-session.md)
target the current main baseline. lovlyDev separately authorized the six listed
files on 29 October 2026, and the patch is applied in the task branch. Compilation
and real-worker results remain required. Other Issue #3 files, worker protocol and
Cargo remain outside task edits.
