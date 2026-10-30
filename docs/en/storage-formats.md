# Storage format comparison

[Documentation](index.md) · [Русский](../ru/storage-formats.md) · [Program](storage-program.md)

The target format is selected in the [decision](storage-format-decision.md); the production backend is not connected yet. Compared candidates are the
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
SQL copy/export preflight conservatively budgets JSON escaping at six bytes per
text byte. Valid near-limit archives can therefore be refused before the common
16 MiB archive limit. Both candidates use the same small/medium/large fixtures;
this is a prototype restriction, not a new general limit on user history.

## Read boundaries

### Checks required before format selection

The existing v2 crash-safety audit identified two boundaries. Shared CAS objects
are created directly at their final `assets/<hash>` names: a partial write can
leave an invalid object that a retry refuses. The old published manifest does
not turn that object into a new commit. A future fix must write and verify an
owned temporary file before installing a final name without replacing existing
CAS. Readers do not remove or recover leftovers. File `sync_all` alone does not
establish directory-entry durability under power loss.

Legacy recovery compares the source manifest before taking the storage lock.
Publication needs a repeated comparison under that lock, and journal cleanup
must identify the pending target actually handled. Otherwise a newer external
commit can be replaced by an older one. This finding is not a recovery fix.
The SQL candidate is not proven either: its normal opener refuses journals;
explicit recovery and crash boundaries still need separate execution. Shorter
metadata-save time alone cannot select the format.

A new opt-in crash benchmark kills its own child test process at four boundaries:
manifest staging/after publish and SQL transaction/after commit. The unfinished
SQL transaction must leave a real hot-journal header and changed database pages;
the reader must refuse without writes. A separately selected SQLite recovery
then runs only on the test-owned fixture and verifies old metadata, the exact
history archive and CAS. After commit, new metadata must survive. Output separates
process wall time, read/recovery and explicit SQL recovery. Initial execution is recorded below;
this is process death, not power loss, production recovery or evidence of an
actual CAD model commit/USB/NAS.

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
an OS cold-cache measurement. A new test route starts a separate child test
executable and verifies PID, nonce, metadata hash, history and one asset; process
startup time is separate from reader operations. OS cache remains uncontrolled.
The `verified_save_copy` prototype creates a new UUID, transfers history/assets,
rechecks the source and reads back the destination. Sources remain unchanged;
failure can leave owned staging without a published project. Regressions cover
corrupt assets, changed SQL sources and budget rejection before asset reads.
The approved [CADPACK test hook](../integration/storage-cadpack-benchmark.md)
times the actual writer, sync, reader and content validation as
`cadpack_round_trip`. Destinations are disposable synthetic fixtures, not a
production copy/export transaction. Executed copy/export observations appear below; crash
recovery, cold-cache and production publication receive no invented timings.
Final raw results and a format decision require actual execution.

### Executed crash boundaries

[Storage comparison 41134254937](https://github.com/lovlyDev/FormaCAD/actions/runs/41134254937)
succeeded on `d08a99efb07ff6e3427ba2eb8e9b6d5222d8668a`.
[Raw JSON](../storage-benchmark/d08a99e-linux.jsonl),
[environment](../storage-benchmark/d08a99e-environment.txt),
[whole-process resources](../storage-benchmark/d08a99e-process-resources.txt).
All four boundaries passed. Before commit, SQL left a real hot journal and
changed pages; the reader refused without modifying files. Explicit recovery
took 5.300 ms and restored original metadata/history. Before publish, manifest
retained its old commit; after commit both representations returned new metadata
with unchanged assets. Further harness hardening requires a separately confirmed
kill request and closes pools on errors; its result needs a new SHA check.

### Executed copies, CADPACK and child processes

[Storage comparison 41133991241](https://github.com/lovlyDev/FormaCAD/actions/runs/41133991241)
succeeded on `4f8b394f3312454ba69093e02b481204c8169d93`, PR merge checkout
`3b890cc06d31324264ebec34da66b063ea4d3bfa`. Saved
[JSON](../storage-benchmark/4f8b394-linux.jsonl),
[environment](../storage-benchmark/4f8b394-environment.txt) and
[whole-process resources](../storage-benchmark/4f8b394-process-resources.txt).
Strict Clippy with native-worker-client passed. The benchmark proved 6 CADPACK
round-trips and 15 separate process starts for original sources and independent
copies. Both large backends produced identical metadata/history hashes and 128
unique assets from 256 references; CADPACK size was 16973180 bytes. Verified
prototype copy: manifest 4364.363 ms, SQL 676.135 ms. These are single observations
on virtual AMD EPYC 9V45 / 4 vCPU / Rust 1.97.1 with uncontrolled OS cache.
This run executed only the opt-in benchmark; all unit tests and Quality on a
newer head require separate evidence. The crash benchmark was added later.

### Isolated SQLite comparison

[Storage comparison 40945699832](https://github.com/lovlyDev/FormaCAD/actions/runs/40945699832)
and [Quality 40945699710](https://github.com/lovlyDev/FormaCAD/actions/runs/40945699710)
succeeded for `de735801142dbdda56d9b5a0fe4f12cb0ad34681`;
PR merge checkout was `0454ce6d0a99d5a786f44ef232962bc7732e3331`.
Saved [raw JSON](../storage-benchmark/de73580-linux.jsonl),
[environment](../storage-benchmark/de73580-environment.txt) and
[whole-process resources](../storage-benchmark/de73580-process-resources.txt).
Ubuntu 22.04 / Linux Azure, AMD EPYC 9V45, 4 vCPU; virtual NVMe with ROTA=0,
physical storage type unverified. Rust 1.97.1. Large single-observation metadata
open: full manifest 313.520 ms, metadata reader 9.627 ms, SQL 4.013 ms;
history status: 628.322 / 18.342 / 0.782 ms respectively. Metadata save:
manifest 1327.064 ms, SQL 8.852 ms plus separate writer startup 1.523 ms.
Work and validation scopes differ, with no cache flush; this does not select a winner.
New copy/CADPACK/child-process checks were added later and are not verified by this SHA.

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
