# Storage format comparison

[Documentation](index.md) · [Русский](../ru/storage-formats.md) · [Program](storage-program.md)

The independent folder backend format has not been selected. Candidates are the
existing v2 manifest/generations representation and experimental project-local
SQLite. The SQL candidate uses existing schemas and typed history; the global app
database does not supply its synthetic data. This is a benchmark, not a user-facing
open/save route.

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

## Shared integration

The [measurement contract and exact patch](../integration/storage-measurement-session.md)
target the current main baseline. lovlyDev separately authorized the six listed
files on 29 October 2026, and the patch is applied in the task branch. Compilation
and real-worker results remain required. Other Issue #3 files, worker protocol and
Cargo remain outside task edits.
