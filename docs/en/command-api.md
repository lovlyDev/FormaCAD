# Typed model edits

[Documentation](index.md) · [Русский](../ru/command-api.md) · [AI](ai.md) · [Sketches](sketches.md)

Local 1.2.6 lets AI propose a small command list instead of rewriting the complete CAD document. For example: “Set width to 80 mm and unlink point a's X coordinate without moving it.” Commands stage against the captured CAD IR v2 revision, pass validation/BREP checks, and save one revision after approval. Intermediate commands are never persisted. The existing confirmation UI shows the resulting CAD document rather than an interactive command list.

## Proposal format

```json
{"message":"Change width and unlink one axis","expectedRevision":"draft","commands":[{"command":"set_parameter","parameterId":"width","valueMm":80},{"command":"remove_sketch_binding","featureId":"base_sketch","bindingId":"a_x"}]}
```

`expectedRevision` must match `currentProgram.revisionId`. Only 1–256 known commands and recognized fields are accepted. `commands` excludes `cad`/`program`; unknown commands and extra fields fail. New projects, imported STEP/mesh and missing CAD IR v2 are not command bases: return a supported complete `cad` or clarification with `program:null`. Codex, Claude and Custom CLI share the same parser.

## Coordinate-link commands

| Command | Fields | Result |
|---|---|---|
| `add_sketch_binding` | `featureId`, `binding` | Adds an explicit link; duplicate IDs/axes fail |
| `set_sketch_binding` | `featureId`, `binding` | Replaces the existing link with the same ID; missing IDs fail |
| `remove_sketch_binding` | `featureId`, `bindingId` | Materializes the selected axis's effective coordinate and removes its link |

A `binding` is `{"id":"a_x","target":"point","pointId":"a","axis":"x","value":{"kind":"parameter","parameterId":"width","scale":-0.5,"offsetMm":0}}`. Sketch origins use `target:"origin"`, no pointId, and x/y/z; points support x/y. Coordinate = parameter × scale + offsetMm. Bindings and constraints share a limit of 64 per sketch; ranges/references are documented in [sketches](sketches.md). No automatic profile links or general expression engine are implied.

Commands run in order: changing width from 40 to 80 before removing a_x materializes -40 mm. Other axes retain their links; later width changes no longer move the unlinked axis. Removing a link does not delete its point. Direct set_sketch_point on linked points fails PARAMETER_BOUND; edit the parameter/link or explicitly unlink first. Use set_literal for literal dimensions and set_parameter for linked dimensions.

## Data route and storage

The [AI parser](../../apps/desktop/src-tauri/src/agents/protocol/command_response.rs) reads a strict proposal. [Command API](../../apps/desktop/src-tauri/src/cad_ir/commands.rs) clones the source, verifies revision, stages the batch and validates the result. [Link commands](../../apps/desktop/src-tauri/src/cad_ir/commands/bindings.rs) use the same coordinate resolver as diagnostics/BREP; unlink materializes only the selected axis. Any failure returns a code/targetId and leaves the original unchanged.

[Session](../../apps/desktop/src-tauri/src/agents/session.rs) captures source/revision before launching CLI; [repair](../../apps/desktop/src-tauri/src/agents/repair.rs) builds the candidate in the isolated worker. The result enters existing App approval. Applying checks the project's revision again; a model changed during the request rejects acceptance. The existing modeling/projects route stores one source revision and STEP/GLB artifacts in SQLite/project files. Bindings remain in source/CADPACK; commands do not export files or run code. No SQL migration is required.

This establishes a transaction API foundation; interactive AI diff, general undo/redo and portable project storage remain separate 2.0 tasks. Complete cad responses remain compatible. The CadQuery compatibility route rejects native command proposals. Contract tests require no external provider accounts/CLI and do not verify them.

## Verification

Check parameter-before-unlink ordering, retained independent axes, origins, stable-ID replacement, stale revision, failing late commands and duplicate targets. A failed batch must preserve the entire original document. Provider fixtures cover Codex/Claude/Custom, mixed responses and the 256-command bound. Worker tests verify changed linked-profile metrics and unchanged geometry after unlinking all axes.

Unlinking a coordinate leaves independent dimension references intact: if width also controls an extrusion length, changing width still changes that length. Fully detaching a part requires explicit edits to those dimensions/links; remove_sketch_binding does not freeze the entire body.

Before recording the reply and accepting a candidate, [candidateSource](../../apps/desktop/src/features/agents/candidateSource.ts) checks both project ID and revision. Copied projects can retain revision IDs, so revision equality alone is insufficient: an original project reply must not be recorded/applied to another open project.

The sample draft revision matches the [reproducible linked-profile document](../fixtures/linked-profiles.cad.json). Saved projects require their actual current revision ID rather than the sample draft value.

[AI candidate review ](ai-candidate-review.md): interactive geometry and structural comparison before explicit acceptance.
