# Independent folder format decision

[Documentation](index.md) · [Русский](../ru/storage-format-decision.md) · [Measurements](storage-formats.md)

wenlixxq's implementation target is **project-local SQLite using DELETE rollback
journal and FULL synchronous, plus immutable attachment CAS**. lovlyDev accepts
the program and changes to the shared application route. This selects a target
backend; it does not announce an available user workflow. LegacyManaged and
supported manifest v2 reads remain. Existing folders are not rewritten
automatically, and benchmark schema does not become production schema.

## Rationale

The isolated SQL prototype uses real Project and typed project/history rules
without the global application database. Executed comparisons on
[de73580, 4f8b394 and d08a99e](storage-formats.md) cover metadata, history status,
bounded attachments, growth, independent copies, actual CADPACK, fresh processes
and four process-death boundaries. Raw observations and environments are saved
alongside the report. Repeated references actually reuse one CAS object.

The SQL prototype costs less on these fixtures, but operations perform different
validation work. The decision also rests on one transaction for project payload,
cursor and events. It avoids coordinating separate manifest and cursor pointers
and permits undo/redo status without all assets or the complete archive. External
CAS still needs its own publication protocol.

Manifest/generations with a separately sealed cursor remains possible. Current
v2 does not support that entry: it needs a schema change, cursor/archive
consistency checks and another comparison. It is unmeasured and does not inherit
the current metadata reader's timings. Format selection does not automatically
fix the identified CAS and legacy recovery defects.

## Required implementation contract

1. `project.sqlite` is authority for project/model/history. The global app index
   contains derived location/recent/pinned/settings only. Losing the index does
   not require model recovery from the old global SQLite database.
2. Production format has its own marker, application_id, schema version and
   project-local migration lineage. Benchmark values are not silently reused.
   Unknown versions or incompatible schema refuse before writes.
3. Readers open verified supported sources without migrations, recovery,
   checkpoints, grants or index writes. Pending journal/WAL/SHM produces a
   distinct recovery-required/unsupported state; never use `immutable`.
   Bound fields/events before materialization; the synthetic coefficient-six
   archive admission is not the final production contract.
4. Snapshots retain backend/location/opening/epoch and original saved state.
   Equal project IDs at different sources are not interchangeable. Writer lease
   is distinct from read-session; UI flags do not grant write authority.
5. Writers publish verified CAS before SQL commit: owned temporary file,
   bounded read-back, hash, flush and installation without truncating/replacing
   existing CAS. Only then does a SQL transaction publish metadata, cursor,
   events and a new commit ID after expected-base/revision checks. Orphan CAS
   is not a commit.
6. After SQL commit, derived-index failure does not mean the model rolled back.
   Results distinguish uncommitted from committed/index-update-required.
   Cancellation does not delete another opening's staging, CAS or lock files.
7. Recovery is explicit under a valid writer/storage lease and repeats location,
   expected-commit and supported-format checks. Pure readers never open
   write-capable connections to trigger automatic SQLite rollback.
8. A real model commit through the agreed CAD pipeline must survive a new
   process and support undo/redo without the app DB. Benchmark project renaming
   does not count as this vertical scenario.

## Compatibility and integration gates

The first safe LegacyManaged/v2 migration is a verified new copy with a new
identity and unchanged source. In-place migration needs a separate crash/recovery
protocol and a backup before writes. Real 1.1 data may transfer through verified
STEP into a new project without invented parametric history.

Shared open/save/copy/history, IPC, frontend and error display require an agreed
exact patch. This decision does not extend approval of the six measurement and
three CADPACK test files to other shared points. Application versions,
Cargo/lockfile, releases and installation remain with the designated integrator.

The first production gate is verified local Windows/Linux/macOS filesystems.
Actual USB/NAS, FAT/exFAT, network locks and power loss are unverified; tempdir CI
does not establish support. Agree the concrete storage matrix before promising
support to users.

Stage 2 still requires final-head checks and recorded Issue limitations.
Security CI separately reports existing Wasmtime vulnerabilities in Issue #3's
area; this task branch neither changes that dependency nor suppresses the audit.
