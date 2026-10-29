# Storage program: baseline audit

[Documentation](index.md) · [Русский](../ru/storage-program.md) · [Snapshots](project-snapshots.md)

Owner: wenlixxq; acceptance and merge: lovlyDev. Local branch `codex/storage-week`
starts at `abe2e054cb4b4f807aa89195825edd0e4e82c62a`, the PR #5 head recorded
in the supplied specification. Current PR metadata, reviews, CI and Issue #3
remain unverified: the connected GitHub account returns HTTP 403
`Your account was suspended`. Versions, releases, installation and native/Cargo/worker
files are outside these changes.

## Shared snapshot lifecycle correction

Users keep the existing attachment, copy and export actions. No new settings are
required. `CommittedSnapshot` retains its original internal `ReadSession`, captured
before SQL and checked after each SQL await and before return. Recheck validates
the original token; recapturing cannot authorize an old response. Close/reopen
rejects it with the existing localized `PROJECT_SNAPSHOT_CHANGED`: refresh the
project and retry. Readers acquire no writer lease and perform no recovery.

| Consumer | Route | Boundary |
| --- | --- | --- |
| History status | capture_history_state → recheck_history_state | Original token and typed SQL cursor; archive separate |
| Attachments | capture_project → bounded read/hash → recheck | Inherits shared lifecycle checks |
| Copy | capture_with_history → artifacts → recheck → persist | Destination write is separate |
| CADPACK | capture_with_history → artifacts → recheck → publish | Publication follows optimistic recheck |
| STEP properties | capture_project → worker → recheck | Native consumer code unchanged |
| Preview | capture_project → worker → recheck | Later awaited cleanup is not atomic delivery |
| Sections | capture_project → recheck and artifact checks | Consumer code unchanged |
| Measurements | snapshot::read → project | Drops token; requires an agreed integration patch |

## Limits and remaining work

Checks are optimistic observations at specific points; they do not block changes
after the final check. Paths do not yet provide OS directory identity: replacing
a directory at the same path with identical bytes is undetected. AccessRegistry
keys by project ID rather than backend/location, and acquire/status/ensure_write
perform filesystem work under its global mutex. These require separate changes
before the next stage is ready. External folders are not independent backends.
Neither stage 1 nor the complete program is declared finished.

## Reproduction

New tests in `project_repository/legacy/session_tests.rs` cover close/reopen for
project/archive snapshots and closure during SQL capture and recheck. They exhaust
the pool and poll to pending without sleeps, asserting rejection and unchanged
logical source; SQL tests also compare files. The existing seven history tests
are preserved.

Run `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --no-default-features --lib project_repository::legacy`.
Fixtures are synthetic: they do not establish real geometry, USB/NAS or installed
UI acceptance. Execution results must be recorded separately; test presence is not PASS.

Local checks on 29 October 2026: rustfmt and `git diff --check` passed;
check:docs covered 108 documents, check:i18n covered 1149 messages,
check:migrations and check:version passed (version remains 1.3.0).
Rust tests did not execute: after dependencies downloaded, compilation stopped
at `linker link.exe not found`. The standard `scripts/windows-env.ps1` also
stopped because `C:\Program Files (x86)\Windows Kits\10\Lib` is absent.
Branch CI and strict Clippy remain unverified. No installer was built.
