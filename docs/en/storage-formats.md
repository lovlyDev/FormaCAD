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

Final scoped stage 2 verification:
[Storage comparison 41137407683](https://github.com/lovlyDev/FormaCAD/actions/runs/41137407683)
succeeded on `d089e6c016cb5e03efb9c6d391a2d475ec36b0fe`: strict Clippy,
6 CADPACK proofs, 15 fresh-process probes and 4 process-death boundaries.
[Raw JSON](../storage-benchmark/d089e6c-linux.jsonl),
[environment](../storage-benchmark/d089e6c-environment.txt),
[whole-process resources](../storage-benchmark/d089e6c-process-resources.txt).
The functional matrix and separate security failure are recorded in the
[program](storage-program.md); acceptance and production-backend integration remain separate.

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

The next real CAD executor seam has a separate
[exact contract and patch](../integration/storage-folder-execution.md) for six
files. It was applied after lovlyDev separately authorized all six paths,
including the two Issue #3 native asset files under specification §5. This does not establish a working
folder model commit.

### Independent SQLite backend foundations (stage 4, verification)

`project_repository/sqlite_folder` introduces a separate FCP1 schema
(`application_id=0x46435031`, `user_version=1`). Repository, project and commit
UUIDs are distinct. The reader checks the complete schema fingerprint and seals
the project, cursor and history archive with hashes. Publication distinguishes
confirmed commits, rejection before COMMIT and indeterminate COMMIT outcomes;
an indeterminate operation must not be retried automatically. Initialization
refuses to alter a nonempty foreign database.

The reader uses a non-default SQLite VFS denying writes, journal recovery, SHM
and ATTACH. It checks the actual opened file, retains its directory binding and
reads CAS through SQLite file primitives so closing an accidental Unix hardlink
does not release another connection's process locks. This layer supports Linux
and Windows; other platforms fail explicitly. Its ABI check pins SQLite 3.46.0
from the current lockfile.
The main database must also have exactly one filesystem link: another hardlink
name could hide the original rollback/WAL journal namespace. Refusal removes
neither the original database/journal nor the alternate path.
Writes compare the complete captured snapshot inside `BEGIN IMMEDIATE`, rather
than only the commit UUID. Same-ID metadata changes with recomputed hashes do
not authorize overwriting the source. A SQLite commit-hook regression models
COMMIT refusal: the indeterminate operation retains its known UUID, and a fresh
read determines actual state without replaying the command.
Known-UUID resolution distinguishes the current head, an earlier saved commit
and absence from the validated chain. It runs in one SQL transaction and checks
repository/project UUIDs. Absence does not authorize automatic replay. The
session permits resolution after the head advances, but rejects it after close
or through a different opening.

`project_access/folder_lease` retains a separate selected-folder writer lock.
Initialization requires an existing empty directory and does not claim atomic
reservation of an external namespace. Operations retain directory handles,
detect replacement and refuse links.

These modules are currently registered **only in test builds**. There is no FCP1
open/write user action yet. Tests cover the SQL contract, sidecar/WAL refusal,
source preservation and independent locks; they do not establish CAD commits,
crash recovery or stage 4 completion. Local compilation needs the Windows MSVC
linker. CI must verify this code before production routing is connected.

`sqlite_folder/cas` validates the whole candidate and pending batch before IO.
Existing objects are verified only through the VFS; new objects use `create_new`
and are written and flushed before SQL references appear. The published batch
retains its directory, writer lease and original new-file handles until SQL
outcome resolution. Failure leaves an unreferenced object for explicit recovery
instead of overwriting or deleting an existing file. Bootstrap requires all
candidate bytes and refuses any existing digest. Linux flushes the assets
directory; Windows uses write-through and new-file flushes. Process-crash tests
do not prove arbitrary-device power-loss durability. Flushing the root after
creating assets remains part of the future bootstrap protocol.

`sqlite_folder/write_connection` initially opens existing FCP1 without permission
to mutate data or recover journals. Under `BEGIN IMMEDIATE`, it verifies the
schema and full captured snapshot before permitting its own DELETE rollback
journal. The VFS retains the lease through actual base-file close, including
cancelled SQLx connect and asynchronous Drop. Foreign journal/WAL/SHM, a different
head or schema cause refusal. Linux journal unlink remains an optimistic check
under the cooperative lease, without claiming atomic unlink-if-inode-matches
against an unrelated namespace swap. This layer also remains test-only.

New database creation has a separate private VFS mode: exactly one
`MAIN_DB | READWRITE | CREATE | EXCLUSIVE`, followed by actual empty-file identity
attestation. Existing or raced files are never adopted. Only this mode receives
DDL authority for the immutable FCP1 schema; initialization is followed by saved
snapshot validation and the normal policy. Nonconfirmed outcomes poison further
connection use, and failures after COMMIT retain the UUID as Indeterminate.
Linux additionally flushes assets/.forma/root. Four new checks cover fresh reads,
existing-file refusal, interruption before the schema transaction and cancelled
connect; they are not the complete crash matrix. The host create/model route is
still not connected.

`sqlite_folder/creation` combines project/complete-asset validation, reservation
of the selected empty directory, CAS publication and SQL bootstrap. The blocking
publication job owns its lease: async cancellation cannot release it during IO
or start SQL after the caller is gone. The VFS source retains
`Arc<PublishedCasBatch>` through actual close; closing after SQL errors preserves
the known commit UUID. New tests cover a separate test-process read without app
index, nonempty-folder refusal and cancellation at an actual blocking-job barrier.
These storage fixtures use synthetic bytes, not a real CAD model commit.

Existing writes now require a `ReadBinding` minted only by the original
FolderSession under its mutex after snapshot recheck. It binds opening, location
and original actual DB identity. The VFS compares its actual writer base before
allowing journal/write authority. A copy's identical SQL UUIDs/seals do not
replace physical source authority. Once an operation owns a journal, it may
finish or roll back after lifecycle revocation; its result must not be accepted
into a closed UI session, and a possibly committed UUID requires resolution.
Close does not promise atomic cancellation of an already-started SQLite commit.

`FolderSession::commit` combines exact snapshot/history-intent checks, the real
writer lease, CAS job and SQL commit. It holds the original reader mutex through
completion so that session does not mistake its own journal for foreign recovery.
After COMMIT, it verifies the new snapshot and CAS; verification/close/lifecycle
failure returns Indeterminate with the known UUID, without replay.

For long CAD jobs, `latest_compatible` permits fresh metadata: equal current
revision/files/revisions, the original immutable commit node and only intervening
commits retaining original cursor/archive seals. One read transaction checks the
full validated chain without a schema change. Model edits and undo/redo do not
become metadata-only merely because geometry returns to an earlier state. The
result is a new exact commit snapshot within the original opening.

Shared [modeling/command request preparation](../integration/storage-model-request.md)
was applied after separate approval for five files. It extracts existing limits,
program, ReviewPlan, no-op and job checks without changing managed permission/task
ordering. The separately approved [common orchestration patch](../integration/storage-repository-apply.md)
is now applied: managed execution retains its original read session and compares
revision contents/history as well as head/files, while retaining compatible latest
metadata. Pending recovery is refused rather than reconciled by this route.
The folder adapter and real CAD vertical verification remain incomplete.

The trusted `FolderSession::apply_history` backend now uses the existing pure
history planner and session commit route. It requires a captured snapshot rather
than a revision ID. New synthetic storage regressions cover undo, a fresh child
process reading its CAS artifact, reopening for redo, and stale/closed refusal.
These tests do not establish CAD geometry correctness and await execution in CI.

At `1c5d59e`, Linux library CI passed 387 tests (9 ignored). Native run
41326510425 passed 454 library tests, 11 worker tests and host suites of 7/7/9;
storage comparison and Clippy also passed. Windows aborted when a test hook read
an empty locked writer-lock file. The fixture now checks its zero length without
reading locked bytes, and test hooks catch panics before returning across C ABI.
The new history tests and Windows correction require a fresh CI run.

The folder executor asset adapter prefetches only selected captured descriptors
through verified session reads. Metadata and unique-hash byte limits are checked
before CAS payload buffers are allocated; names sharing a hash share the cached
bytes. The synchronous existing CAD callback accepts only selected descriptors
and checks the original opening/location binding. Cached input does not authorize
publication: the host must recheck the original model after execution. The adapter
does not invoke CAD or connect the external-folder UI by itself.

The independent save-copy backend targets an existing empty directory and needs
no source writer lease or app-index row. It preserves revision IDs and portable
history while assigning new project/repository identity. It verifies source CAS,
rechecks the original snapshot after destination CAS staging and before SQL, then
checks it again after publication. There is no atomic transaction across source
and destination. If the final check fails after a destination commit, its known
commit UUID is retained as an indeterminate outcome; the copy is neither replayed
nor deleted automatically. Interrupted staging may leave unreferenced destination
assets. Source bytes and history remain untouched. New adapter/copy regressions
require CI; these are backend APIs, not available UI scenarios.

Runtime CI at 79431c2 found excessive VFS refusal: optional SYNC/COMMIT_PHASETWO
pager notifications returned READONLY. They now report NOTFOUND without delegation,
as SQLite permits. Bootstrap authorization allows REINDEX solely for the frozen
main index during creation. Initial writer attestation preserves original recovery
errors. These fixes still require new runtime CI.

Reader/writer now compile a single SQLite C ABI/platform binding after Clippy
identified duplicate modules and clashing extern declarations. Per-IO checks use
identity/lease; full CAS hashes remain at publication boundaries instead of
rehashing every attachment for every SQLite page.

CI showed that a SQLx connection must be discarded after COMMIT failure; outcome
resolution uses a fresh connection. The file-backed regression now follows that
route. Windows writer directory handles request FILE_LIST_DIRECTORY because
attribute-only opens do not enforce the intended delete-sharing check; the
assets-retention test detected this limitation.

`sqlite_folder/session` retains a distinct opening and serializes access to its
SQLite connection. Capture and recheck compare repository/project UUIDs, commit
identity and validated state hashes. Close immediately revokes the opening,
including operations waiting for the mutex. Another opening cannot accept a
snapshot even from a copied folder with identical UUIDs. Asset reads require the
exact captured descriptor, verify CAS and then recheck SQL state. New regressions
cover copying/reopening, metadata commits during a live session, close while
waiting and asset replacement. These also require execution in CI.

The [measurement contract and exact patch](../integration/storage-measurement-session.md)
target the current main baseline. lovlyDev separately authorized the six listed
files on 29 October 2026, and the patch is applied in the task branch. Compilation
and real-worker results remain required. Other Issue #3 files, worker protocol and
Cargo remain outside task edits.
