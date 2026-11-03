# Storage program: baseline audit

[Documentation](index.md) · [Русский](../ru/storage-program.md) · [Snapshots](project-snapshots.md)

Owner: wenlixxq; acceptance and merge: lovlyDev. Local branch `codex/storage-week`
started at `abe2e054cb4b4f807aa89195825edd0e4e82c62a`. API access confirmed
PR #5 merged; on 29 October the branch merged current main at
`4834e1020f755d4a158a105fc63965315d9c78c1` without rewriting history.
The locally authorized wenlixxq account has working API access; the separate
connector account's HTTP 403 does not apply to it. The program is tracked in
[Issue #27](https://github.com/lovlyDev/FormaCAD/issues/27), stages #28–34.
Issue #3 boundaries were read and remain excluded. Versions, releases, installation and native/Cargo/worker
files are outside these changes.

## Current position: 4 November 2026

`7e02a7c` passed strict Clippy without native client and 92 folder backend tests
(0 failed, 5 ignored) in
[run 41736623409](https://github.com/lovlyDev/FormaCAD/actions/runs/41736623409),
job `126267360681`. Coverage includes actual event-read denial by SQL authorizer,
refusal of history/copy/write with a damaged archive and original-view validation
after an external commit. The mandatory following native-client gate again
failed on known snap E0609; the complete run remains red.

`2423200` combines owner-accepted main `6a882fe` into the task branch. Only RU/EN
i18n conflicted; both sets of keys were retained. Version 1.6.1, Cargo/lock/Tauri
and release files are inherited from main without task-specific changes. Local
checks of the combined code passed frontend build, 294 unit tests in 104 files,
51 browser tests (41 native-fixture skips, 0 failed), i18n 1480, docs 172,
migrations and Rust formatting. Two EOF blank lines in settings-and-updates are
inherited unchanged; task-owned staged changes passed diff checking. Current-head
remote CI is not yet confirmed. These checks do not prove native geometry or
connected user-facing folder UI.

Stage 4 is active; stages 5–7 are incomplete. The task branch includes accepted
M2 main `a4aec3e6`; the approved integration preserves amendment/Undo and original
storage sessions. The [Rust host API](storage-folder-host-api.md) brings FCP1 into
ordinary supported builds and the common reader without app SQL. On `ca66156`,
strict Clippy without native client and four API tests passed in
[run 41607907708](https://github.com/lovlyDev/FormaCAD/actions/runs/41607907708).
The whole run failed: mandatory native-client compilation found E0609 in the new
snap consumer. Its exact one-line patch was subsequently approved by lovlyDev on
4 November and applied in `1b54cd4`; new native checks are still running. Scope is in
[Issue #31](https://github.com/lovlyDev/FormaCAD/issues/31#issuecomment-6599119794).

On `edd2f92`, the production V2 backup/migration API passed strict Clippy without
native client and all six API tests in run `41609704135`; the whole run again
failed on known snap E0609. Windows/Linux/macOS ARM Quality jobs passed. A separate
macOS correction scopes writer stamps to supported Windows/Linux without
disabling macOS jobs. FCP1 metadata/archive separation is now implemented but
needs current regressions and measurements. UI/index, a genuine 1.1 fixture and
hardware gates remain incomplete. Historical
results below prove only their explicitly named SHAs.

## Shared snapshot lifecycle correction

Users keep the existing attachment, copy and export actions. No new settings are
required. `CommittedSnapshot` retains its original internal `ReadSession`, captured
before SQL and checked after each SQL await and before return. Recheck validates
the original token; recapturing cannot authorize an old response. Close/reopen
rejects it with the existing localized `PROJECT_SNAPSHOT_CHANGED`: refresh the
project and retry. Readers acquire no writer lease and perform no recovery.

| Consumer | Route | Boundary |
| --- | --- | --- |
| History status | capture_controls → model context/cursor → recheck | Original token/raw seals, no chat strings/CAS/archive; full inline/noncanonical fallback; seven regressions on 12acee6 |
| Description and revision list | read_project_snapshot → ProjectRepository capture/recheck; list_managed catalog | Pure mixed-catalog/target test passed on e10c643; UI target reads/guards added in 7a6a91d, final CI separate |
| Structural revision diff | compare_ir_revisions → ProjectRepository → capture → recheck | Six stale/purity regressions passed on d089e6c |
| Attachments | capture_project → bounded read/hash → recheck | Inherits shared lifecycle checks |
| Copy | capture_with_history → artifacts → recheck → persist | Destination write is separate |
| CADPACK | capture_with_history → artifacts → recheck → publish | Publication follows optimistic recheck |
| STEP properties | capture_project → worker → recheck | Late close/reopen regression passed on f20a146/12acee6 |
| Preview | capture_project → worker → cleanup → original recheck | Late close/reopen and held cleanup mutex covered; delivery remains optimistic |
| Sections | capture_project → STEP IO/worker → original recheck and artifact seals | Late manual/face/pair regressions passed on f20a146/12acee6 |
| Measurements | snapshot::read → CommittedSnapshot → worker → recheck | Original lifecycle plus separate geometry seals; metadata-only saves preserve the request |

The measurement correction retains original lifecycle before/after geometry
recapture while preserving metadata-only save behavior. See the
[integration note](../integration/storage-measurement-session.md) for native
validation boundaries. New `Storage native consumers` runs three host suites;
workflow presence does not establish success for the corrected head.

Verification completed for `d089e6c016cb5e03efb9c6d391a2d475ec36b0fe`:
[Quality 41137407828](https://github.com/lovlyDev/FormaCAD/actions/runs/41137407828)
passed functional Windows/Linux/macOS ARM/Intel jobs, frontend and CAD. The whole
run is failure because its separate security job reports existing Wasmtime
50.0.0 (RUSTSEC-2026-0422/0423/0424). Linux library: 274 PASS, 0 FAIL,
5 ignored. All six structural diff tests and both new lifecycle regressions passed.
[Storage native consumers 41137407525](https://github.com/lovlyDev/FormaCAD/actions/runs/41137407525)
proved 330 library PASS / 5 ignored, 11 worker PASS, face host 7, pair host 7
and reference host 9 PASS. All three queued metadata-only regressions and the
real-worker close/reopen test passed. This is the verified basis for the next
stage 3 consumer; the whole program is unfinished and lovlyDev retains acceptance.

## Limits and remaining work

Checks are optimistic observations at specific points; they do not block changes
after the final check. Sessions now retain OS identity for the root and existing
source directories: Windows volume/file ID or Unix device/inode. Read handles
prevent reuse of the retained identity and grant no writer lease. Rename,
same-path replacement and appearance of a previously missing directory invalidate
the old snapshot. Root/path symlinks, reparse points and junctions are rejected.
Unsupported Windows file-ID queries fail closed without timestamp fallback.
Retained handles do not establish NAS guarantees. AccessRegistry
keys by project ID rather than backend/location. These require separate changes
before the next stage is ready. External folders are not independent backends.
Stage 1 baseline checks are complete on the SHA below; acceptance and merge
remain with lovlyDev. The complete program is unfinished.

## Reproduction

The registry holds its global map lock only while resolving an
`Arc<Mutex<ProjectState>>`. IO, OS lease acquisition and owner JSON use the
individual project gate. A slow source does not hold the global map. Sessions,
epochs and Weak writer references remain in project state; release retains a
lease while an in-flight WriteGuard survives and never unlinks the lock file.
New `project_access/registry_tests.rs` regressions cover another project operating
while the first gate is held, release/reacquire with a surviving guard, and
transient writes. An additional history regression closes during SQL recheck;
cursor capture checks the original token after both SQL awaits.

`project_repository/location/tests.rs` covers unchanged/missing paths,
leaf/parent/root replacement, traversal, regular files and platform links.
Two snapshot regressions cover directory replacement with unchanged SQL and
identical project IDs/payloads at different roots. They passed Quality on `1ad4937`;
later changes require their own run.

History write validation is extracted into pure `project_history::prepare_commit`.
It returns a privately constructed cursor/event payload; the SQL writer uses the
same function inside its transaction. Append, metadata, undo/redo, revision
immutability and original PreparedHistory fencing remain intact. This is adapter
groundwork, not a new write route: external folders are not wired yet.
Eight tests cover these rules and actual SQL state/event persistence with rollback
preserving the original source.

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
check:migrations and check:version passed on the original 1.3.0 baseline.
Merging main inherited the owner's 1.5.1 version; task changes do not modify
version files relative to main. Checks on the updated baseline are separate.
Rust tests did not execute: after dependencies downloaded, compilation stopped
at `linker link.exe not found`. The standard `scripts/windows-env.ps1` also
stopped because `C:\Program Files (x86)\Windows Kits\10\Lib` is absent.
Branch CI and strict Clippy remain unverified. No installer was built.

## Verified stage 1 baseline

[Quality 40869695219](https://github.com/lovlyDev/FormaCAD/actions/runs/40869695219)
succeeded for head `1ad4937569edb2aaf86ba3f7407b48c2f7b6cb34`: Windows, Linux,
macOS ARM/Intel, frontend, Python CAD and security. Windows/Linux Rust library:
249 passed, 0 failed, 3 ignored (benchmark and service helpers). Strict Clippy
and fmt passed. Frontend: 215 tests / 75 files; browser: 45 passed / 35 skipped
without native fixtures, which does not establish installed/native UI acceptance.
[Storage comparison 40869695213](https://github.com/lovlyDev/FormaCAD/actions/runs/40869695213)
on the same head passed strict `native-worker-client` Clippy and the benchmark.

Real Linux worker coverage ran separately in
[Quality 40868152101](https://github.com/lovlyDev/FormaCAD/actions/runs/40868152101),
job 123428058352, head `147a0dd05e1a15c94c7656b9569eca4f028b795c`:
292 library passed / 0 failed / 3 ignored and 11 worker passed. This includes
`close_and_reopen_after_real_worker_rejects_unchanged_measurement_source`.
The six agreed measurement files are identical between `147a0dd` and `1ad4937`;
this verifies that route, not the entire later head. Other jobs in that older
Quality run failed on the since-corrected benchmark lint; the whole old run is
not described as successful.

Stage 2 continues after this baseline. Local SQL-comparator corrections are not
covered by that evidence. USB/NAS, the full new folder-write route, late pair/face
responses and installed UI remain separate gates.

A navigation regression separately switches A/B with identical revision and body IDs in an opaque fixture model program: late A cannot replace B. All five navigation tests passed after strengthening the fixture. This checks frontend identity/generation, not CAD-document or kernel validity.

## Dependency cache for owned checks

The owned storage comparison/native workflows enable `cache-on-failure: true`
for `Swatinem/rust-cache@v2`. Standard toolchain/Cargo environment keys remain;
workspace crates are not added to the cache. Cached dependencies never replace
test results, source SHA or compile gates. This avoids rebuilding unchanged
dependencies after fixing task code; actual speed improvement must be measured
on subsequent runs. Owner Quality, Cargo and versions are unchanged.
The input is documented in [rust-cache configuration](https://github.com/Swatinem/rust-cache#configuration).
