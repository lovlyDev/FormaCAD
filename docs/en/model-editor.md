# Model editor

[Documentation](index.md) · [Русский](../ru/model-editor.md) · [Preview](model-preview.md)

**Edit model parameters** opens the draft editor with source status and operation/body counts. Named parameters sit above the feature tree: names and IDs, aligned unit-tagged value fields, referencing operations and a dedicated delete control. UI icons come from installed Lucide; user-authored names are not translated. Existing JSON and stable IDs remain intact.

Feature cards show type, activation status, ID, dependencies and supported modifier suppression. Sketch tools expand inside their feature. Body outputs select the last operation below the tree. Referenced parameters cannot be deleted. Long names/references wrap; narrow windows stack panels vertically. Both themes use project surface/text/border variables.

JSON is in the expandable **CAD source / Advanced** section. **Format JSON** changes valid JSON presentation without changing IDs/values; failure leaves the text intact. Manual source edits remain available and pass normal build validation. Empty projects can create a starter sketch or accept a pasted document.

The right panel provides [exact 3D preview](model-preview.md). Close/build controls remain visible while scrolling. Closing retains the draft through existing preferences but creates no revision. Building saves a new revision only after successful validation. Code: [ProgramDialog](../../apps/desktop/src/app/dialogs/ProgramDialog.tsx), [CadParameterEditor](../../apps/desktop/src/components/CadParameterEditor.tsx), [TypedFeatureTree](../../apps/desktop/src/components/TypedFeatureTree.tsx). Shared Modal is now its [own module](../../apps/desktop/src/components/ui/Modal.tsx).

Imports without CAD IR are protected against replacement by a starter sketch. See [file exchange](viewer-and-files.md).

## Selected edge fillet

Enable “Select CAD edge” in the workspace, click an edge, then open “Edit model parameters”. The persistent “Edge fillet” section at the top shows the edge ordinal and body ID. Enter a radius, choose “Add fillet to draft”, inspect the 3D preview, then build. The button has moved out of Model properties. A filletEdge feature is appended to CAD IR and becomes the body’s source. Only a successful build through the usual permission checks saves a new revision.

Mapped box edges support the legacy box-edge key. Positive rectangle extrusions also supply [operation-owned references](topology-references.md) through supported translation, rotation, and mirroring. The editor prefers the selected edge's complete reference and creates filletReferencedEdge; it rejects a wrong owner or transform branch without falling back to a guessed box key. STEP and ordinary meshes do not restore this operation history; not every exact CAD edge supports filleting. The section stays visible and explains missing selection, missing stable reference, or the need to build a changed draft and reselect. A selection from an older revision is rejected. Zero, negative, and nonfinite radii are rejected; the CAD kernel validates geometric radius limits. The radius field and draft action share a 32 px height and aligned bottom edges. Styles live with the fillet module, follow both themes, and use reduced-motion-aware hover transitions; the panel retains the shared 260 ms disclosure motion on every toggle.

## Editor state

Each project retains expanded parameter, sketch, fillet, and source sections; automatic preview; solved-coordinate preview; new parameter and fillet radius fields; tool point selections; new contour dimensions; sketch framing; and scroll position. editorPreferences uses the existing usePersistentState: localStorage is a fast cache, and set_ui_preferences writes SQLite settings/ui_preferences after 800 ms. Closing the editor flushes queued writes. These preferences are separate from the CAD document and revision history. No migration is needed. Section opening respects reduced motion and both themes.

Verify reopening and application restart, separate project preferences, stale edge selections, unsupported references, preview, and a successful new revision. Code: [preferences](../../apps/desktop/src/features/model-editor/editorPreferences.tsx), [fillet panel](../../apps/desktop/src/features/model-editor/EdgeFilletEditor.tsx), [draft operation](../../apps/desktop/src/features/model-editor/edgeFilletDraft.ts).


In 1.2.6 parameters can drive coordinates and origins across multiple sketches. Open Edit 2D sketch → Coordinate parameter links; [the sketch guide](sketches.md#explicit-coordinate-links) explains storage, limits and the linked-profile example. Pre-build review requires an explicit choice when geometric constraints change the drawn shape.

## Section motion and tool sizing

Click or Enter toggles sections through [AnimatedDetails](../../apps/desktop/src/features/model-editor/AnimatedDetails.tsx). Height and opacity animate for 260 ms; reversal starts from the current frame. Contents remain mounted to retain drafts and become inert/hidden from the accessibility tree on close. Reduced motion switches immediately; pointer interaction finishes animation. PersistentDetails retains existing SQLite preference keys; source uses the saved source.open key. The module changes no CAD document or revision. Sketch dropdowns/actions share a 32 px height and matching top/bottom edges. Verify repeated open/close, rapid reversal, Enter, both themes/locales and narrow layouts.

The frame and disclosure headers now live in shared EditorDialog.css and EditorDisclosure. [AI candidate review](ai-candidate-review.md) uses their exact motion, icons and surfaces. Editor settings retain their persistence independently of transient candidate review state.
