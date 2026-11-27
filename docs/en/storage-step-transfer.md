# Transfer a selected STEP into a new folder project

[Documentation](index.md) · [Русский](../ru/storage-step-transfer.md) · [Folder format](storage-formats.md)

In a Windows or Linux build with an available CAD worker, choose **Transfer STEP
to folder** on the dashboard. Select a saved STEP or STP file and a separate
empty folder. FormaCAD creates an independent FCP1 project from the selected file's
geometry. Older application data, including history and parametric operations,
stays in the original project. See the [folder API](storage-folder-host-api.md#open-a-transferred-step-in-the-original-window)
for details about the desktop flow.

## Host flow

1. Explicit file selection opens `api::SelectedStepFile` and retains `StepRead`.
   Internally these retain the same `SelectedStep`/`StepSnapshot` implementation.
   The 40 MiB input limit precedes allocation; reads check identity/hash/lifecycle.
2. An explicit creation decision calls `approve_destination`, producing a
   single-use `StepDestinationApproval` whose internal `StepTransferGrant` binds
   that source opening and an empty selected directory. It is private,
   nonserializable and distinct from the real OS writer lease.
3. `transfer_to_folder` accepts `StepProjectIntent` (new name/agent/prompt) and
   trusted `ModelRuntime`, delegating to existing `step_transfer::transfer`.
   It generates a fresh project UUID and uses mm units.
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
it through public `SelectedStepFile::transfer_to_folder`, checks a first commit
with one ImportStep, and inspects saved geometry/volume in a
fresh process and checks source file purity. Invalid STEP must fail before target
SQL/CAS creation. The workflow requires one executed passing test; zero matching
tests are not a pass.

The same workflow compiles a project generator from the FormaCAD 1.1.0 sources
and supplies it through `FORMA_LEGACY_1_1_GENERATOR`. It checks for
`FORMA_LEGACY_1_1_SOURCE_UNCHANGED` after comparing the original file tree.
This covers selected STEP transfer from 1.1.0. USB/NAS support and testing an
installed build require separate runs.
