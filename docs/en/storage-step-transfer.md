# Transfer a selected STEP into a new folder project

[Documentation](index.md) · [Русский](../ru/storage-step-transfer.md) · [Folder format](storage-formats.md)

This stage-4 backend API is currently test-only with the native worker client.
No UI button or IPC is connected. It takes an explicitly selected last STEP and
creates an independent FCP1 project with real validated geometry. It neither
discovers nor parses/migrates an old database. Old credentials, settings, history
and parametric features are not imported or represented as recovered. No genuine
1.1 project fixture has been provided; a synthetic directory containing real STEP
does not prove full 1.1 compatibility.

## Host flow

1. Explicit file selection opens `SelectedStep` and retains its original snapshot.
   The 40 MiB input limit precedes allocation; reads check identity/hash/lifecycle.
2. An explicit creation decision mints a single-use `StepTransferGrant` bound to
   that source opening and an empty selected directory. It is private,
   nonserializable and distinct from the real OS writer lease.
3. `step_transfer::transfer` accepts the new name/agent/prompt, trusted CAD runtime
   paths and task services. It generates a fresh project UUID and uses mm units.
   No app database or recent index is required.
4. Existing `conversion::import_document::source` produces validated ImportStep IR
   referencing the selected asset hash. Common `repository_apply::apply` handles
   intent preparation, authorization, queueing/cancellation, real execution and
   the single pure `PreparedModelChange` preparation.
5. Worker success precedes the first SQL publication. Publication adds the input
   STEP to verified pending assets and calls existing `creation::create_checked`.
   Its first commit contains the model and new import history, rather than an
   intermediate empty project row.

## Checks and outcomes

The original source opening/hash survives queueing, worker execution and commit.
Snapshot clones retain that Arc capability; they do not reopen the path or sample
a fresh hash. Target location identity is retained from the initial selection.
The pre-SQL boundary checks source and target; after commit the shared verifier
checks the exact receipt, SQL/history and every referenced CAS object.

Pre-commit errors never report a saved model. Refusal/cancellation before staging
leaves the target empty; later staging failure can leave only owned unreferenced
assets for explicit recovery. Post-commit failure retains the known UUID as
`Indeterminate`, without deletion or command replay. Existing FCP1 limits remain,
including 100 MiB of file references. The old directory receives selected-file
reads only.

## Reproduction and limits

Policy unit tests cover closed/cross-opening grants, full-future Send, queued
cancel/close, nonempty destination and invalid intent. A separate regression
populates the destination after approval: transfer must refuse without replacing
the foreign file, creating SQL/CAS or a worker workspace, or registering a CAD
task. These policy fixtures use synthetic bytes.
The native workflow explicitly invokes ignored `real_worker_step_transfer_roundtrip`
with `FORMA_FOLDER_TEST_WORKER`. It generates a real 40×20×10 box STEP, transfers
it, checks a first commit with one ImportStep, inspects saved geometry/volume in a
fresh process and checks source file purity. Invalid STEP must fail before target
SQL/CAS creation. The workflow requires one executed passing test; zero matching
tests are not a pass.

Current-head CI is still required. USB/NAS, a genuine 1.1 fixture and production
UI are not established by this API. Storage workflows run on pushes to the owned
`codex/storage-week` branch so conflicting PRs during parallel main development
cannot suppress backend checks. They verify that exact task SHA; clean current-main
integration and Quality remain separate mandatory handoff gates.
