# Review an AI candidate before acceptance

[Documentation](index.md) · [Русский](../ru/ai-candidate-review.md) · [AI](ai.md) · [Commands](command-api.md)

In the native app, an AI response containing CAD IR v2 opens a dedicated **Review AI candidate** dialog. It displays proposed geometry and compares it with the revision captured when the request was sent. This review is required even when ordinary confirmation settings are disabled: receiving an assistant response does not save a new model.

## User actions

1. Send a request to the assistant. After the CAD kernel validates the response, wait for the dialog's preview. **Accept candidate** stays disabled until a fresh preview succeeds.
2. Rotate with the left mouse button, pan with the right button and zoom with the wheel. Fit-to-view animates smoothly; reduced motion disables the transition.
3. Switch between **AI candidate** and **Current model**. The latter loads the source revision's GLB as an independently owned object, leaving the main workspace scene intact. If no saved GLB is available, the dialog explains this without hiding the candidate.
4. Compare parameters, operations and bodies. Parameters show old/new values; operation edits, suppression, history order and renames are identified. Detailed JSON changes and candidate source expand and collapse with motion on each toggle.
5. **Accept candidate** invokes the existing build/apply route and saves one revision. **Reject candidate**, the close button, Escape and dialog dismissal end the review. The source model and history stay unchanged; assistant conversation follows the usual project persistence rules.

Changing the open project or its current revision blocks acceptance. Reject the candidate and request a new plan against the latest model. A failed preview can be retried and exposes technical details; failed geometry cannot be accepted.

## Technical route and storage

`App` captures the project snapshot at request time. `agents/repair` validates the response with at most two bounded correction attempts. For native CAD documents, `ActionReviewDialog` mounts the independent `features/agents/review` module. Structural comparison matches stable parameter/feature/body IDs. JSON object key order is ignored; array order remains meaningful.

`useModelPreview` invokes `preview_model` with the source project ID, expected revision and immutable candidate source. The worker builds BREP/GLB in a temporary workspace. The bounded binary packet is checked against the source SHA-256. Preview creates no revision, export STEP or project attachment and consumes no modification grant. Closing cancels the preview and disposes separately owned Three.js objects; late results cannot reopen the review.

Acceptance rechecks both project and revision IDs. When settings require `modify_project`, a one-use grant is requested/resolved at the decision, avoiding expiration during a long review. With modification confirmation disabled, the backend retains its automatic audit entry; the candidate dialog still requires an explicit user decision. Rejection uses the existing permission audit route. `apply_program` then validates again and saves one revision through its existing transactional path. No SQL tables are added. Comparison state is transient and is not stored as a model.

## Limits and validation

The diff is structural, not a geometric equality verdict. Exact metrics always describe the **candidate**, even while viewing the source model. Imported STEP/mesh or legacy CadQuery without CAD IR has no comparable source structure. The legacy CadQuery route retains its existing confirmation dialog. Views switch within one scene with smooth framing; this release has no overlaid geometry or linked pair of cameras.

Geometry is currently built during response validation, separately for interactive preview, and again on application. Preview output is not trusted as a saved artifact. Unaccepted candidates are not automatically recovered after app exit. Acceptance participates in transactional undo/redo; other 2.0 requirements are assessed separately.

## Structured review plan

`reviewPlan` contains assumptions, proposed dimensions with optional `parameterId`, affected body IDs and expected checks. Its card uses model-editor disclosures and library icons. Dimensions without parameter references remain AI statements. Limits are 16 assumptions of 400 characters each, 32 dimensions, 32 unique body IDs and 16 checks.

`validSolid`, `bodyCount`, `bounds` and `volume` checks use exact candidate geometry. `bodyCount` counts declared CAD IR bodies, so a STEP assembly imported as one compound counts as one body. Bounds and volume use the stated tolerance without a hidden addition. Checks remain pending until geometry is ready; a mismatch blocks acceptance. Before commit, the backend repeats checks using fresh worker output and validates body references and named parameter values. Failure saves no revision.

Route: `plan_model` → strict contract validation → `CandidatePlan` and verified preview → explicit decision → `apply_program` carrying the original plan → independent build and checks → one revision. Eligible geometry failures permit at most two AI corrections from the same committed base under the same requirements. Changing the plan is not a correction. Cancellation, access conflicts and lost references do not trigger these attempts.

Providers without a plan remain compatible; the dialog explicitly identifies the missing structured plan. Assumptions and check results are currently transient, without a durable revision report. Parameters and accepted geometry follow normal persistence. Validation includes contract/metric unit tests, a native wrong-volume test without commit, and RU/EN Chromium tests using real worker GLB, successful checks and blocked acceptance for an incorrect volume.

Validation covers comparison, acceptance/rejection and object disposal tests plus Chromium scenarios using GLB from the real local worker: RU/EN, both themes, narrow windows, repeated disclosures and project switching. These tests do not exercise an authenticated external CLI or installed WebView2. They do not use user projects.

## Shared model-editor interface

The frame, scrolling, footer and source surface use shared `features/model-editor/EditorDialog.css`. Code and detailed comparison headers use the same EditorDisclosure as the model editor; parameters reuse its card and SlidersHorizontal icon. Operations use the existing FeatureIcon. Dialog entry is 200 ms, exit is 140 ms; content disclosure and chevron rotation use the same 260 ms easing. Content stays mounted through the exit instead of flashing the generic permission dialog. Reduced motion disables animations. Transient candidate disclosures do not overwrite the editor’s SQLite preferences.
