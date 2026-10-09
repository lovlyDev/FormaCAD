# AI assistant and permissions

[Documentation](index.md) · [Русский](../ru/ai.md) · [Modeling](modeling.md) · [Security](../../SECURITY.md)

The assistant receives the request and a bounded context from the selected project. It needs a separately installed and configured provider CLI. Viewing an existing model needs no AI account. The main paths are [session](../../apps/desktop/src-tauri/src/agents/session.rs), [CLI setup](../../apps/desktop/src-tauri/src/agents/cli_spec.rs), and [context](../../apps/desktop/src-tauri/src/agents/context.rs).

## Provider setup

Choose OpenAI Codex, Claude, or Custom CLI in project settings. Custom CLI uses a local executable path and one argument per line; Forma launches it without a shell. This configuration is stored locally in SQLite. Check that the selected CLI is installed and authenticated in its own environment. An older CadQuery document may also need Python/CadQuery, while native CAD IR v2 builds without Python. The interpreter path is set in CAD environment settings and survives app updates. See [development](development.md) and [data](data.md).

## Request to revision

1. Open a project, describe a model or edit, and optionally select a model element. Selection is tied to the current revision; a stale selection is rejected.
2. Optionally attach images already saved in the project. Up to four PNG/JPEG/WebP images of at most 20 MB each are accepted with Codex only. Convert PDF/DXF to an image first. A text prompt must contain 1–16,000 characters.
3. Approve `run_agent` when confirmation settings require it. Only one task can run per project at a time. Progress and cancellation are shown in the UI.
4. The native route requests a structured CAD IR plan, validates it, and builds geometry in the isolated worker. Up to two bounded correction attempts are allowed after a known structured geometry error, against the same captured committed model. An invalid candidate is not saved as a successful revision. Older documents use a separate compatibility route.

Isolating the geometry worker and validating its output does not make a third-party CLI a universal operating-system sandbox. The selected provider may process the context it receives under its own rules. See [repair logic](../../apps/desktop/src-tauri/src/agents/repair.rs), [output protocol](../../apps/desktop/src-tauri/src/agents/protocol.rs), and [selection context](../../apps/desktop/src-tauri/src/agents/selection.rs).

## Confirmations and audit

Forma distinguishes `modify_project`, `run_agent`, `export_file`, `convert_file`, and `install_dependency`. Mode `all` asks for each action, `cli` asks for AI execution and dependency installation, and `none` asks for neither; per-action overrides are available. The default is `all`. A grant is bound to one project and action, used once, and expires after five minutes. The latest 100 decisions are available in the audit log. A third-party CLI cannot change a project merely by returning text: applied CAD changes still pass model validation. See [permissions.rs](../../apps/desktop/src-tauri/src/permissions.rs).

If a build fails, check the selected CLI, authentication, current revision, and the technical error detail. For a revision conflict, retry on the latest model. For a denied permission, repeat the action and approve it explicitly. Check the Python path only when troubleshooting an older CadQuery document. Avoid sending sensitive attachments to a provider without understanding its handling rules; see [security](../../SECURITY.md).

[Typed AI edits and link commands ](command-api.md): revision-bound batches and explicit coordinate unlinking without movement.

[AI candidate review ](ai-candidate-review.md): interactive geometry and structural comparison before explicit acceptance.

## Bounded geometry corrections

The native planner makes one initial proposal and at most two corrections: three provider calls and candidate builds in total. Each correction repeats the original committed context and expected revision, plus the latest bounded kernel feedback. A failed candidate never becomes the next base and is never saved as a revision. CAD command batches on every attempt are staged against the same captured document. Required imported STEP assets are verified and staged from that captured project into each isolated candidate directory; candidate directories are removed after validation.

Cancellation and the current committed revision are checked before provider invocation, after its response, and after the geometry build. A stale revision or cancellation stops the request even when the build simultaneously reports a retryable error. Only known geometry errors qualify for correction. Permission denials, CLI/process failures, invalid worker protocols, missing/corrupt input assets and database/file errors return directly. `run_agent` approval authorizes the complete bounded planning request once; applying the reviewed candidate still requires the separate `modify_project` route. No correction bypasses approval or saves intermediate geometry.

The bounded state machine has injected planner, builder and guard tests covering zero/one/two corrections, exhaustion at three builds, unchanged original context, permission denial, cancellation during a failed build, stale state before and after building, non-model responses and input-integrity errors. These tests do not authenticate or run external provider accounts; provider compatibility and installed-runtime acceptance remain separate checks.
## Typed review plan and checked expectations

A native full-CAD or command response may include `reviewPlan`. Absence keeps older CLI responses compatible and must be shown as an absent plan, without inventing assumptions or checks. New responses are instructed to include bounded assumptions, dimensions (`name`, `valueMm`, optional `parameterId`), affected body IDs and expected checks. A plan permits at most 16 assumptions of 400 characters, 32 dimensions with names of 80 characters and values from 0 to 10,000 mm, 32 unique affected body IDs and 16 checks. Unknown fields and invalid bounds are rejected. A named parameter must exist in the candidate and have exactly the declared value; affected body IDs must occur in the captured or candidate document. A dimension without a parameter ID is a provider statement, not a verified measurement.

Expected checks are typed: `validSolid`, `bodyCount {count}`, `bounds {sizeMm:[x,y,z],toleranceMm}`, and `volume {valueMm3,toleranceMm3}`. Body count means CAD IR bodies, not the number of BREP solids inside an imported compound. Bounds and volume refer to the complete candidate, not a selected subbody. Tolerances are explicit and finite; bounding-box tolerance is 0–10 mm. The original review plan is retained across every correction; a later provider response cannot drop or relax its expectations. Failure is `EXPECTED_CHECK_FAILED` and may use the remaining correction budget.

Repair validation reads the fresh verified worker response before returning a candidate. `apply_program` separately accepts optional `reviewPlan`, validates its identifiers and parameter values against the actual proposed source and captured current document, builds the source anew, then checks the fresh worker metrics before saving any revision. UI preview metrics and provider claims cannot replace this commit check. Omitting the option preserves the previous manual/legacy workflow. The plan is transient review data; the current revision schema does not store a durable structured plan or measurement report. Tests cover strict schema/limits, parameter and body references, missing/non-matching worker metrics, full-CAD/command envelopes and unchanged expectations during repair.
