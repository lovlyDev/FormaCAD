# Exact draft model preview

[Documentation](index.md) · [Русский](../ru/model-preview.md) · [Editor](model-editor.md)

Open **Edit model parameters** in the native desktop app. CAD IR v2 shows **3D draft preview** on the right. Select **Preview model**; subsequent edits debounce for 500 ms while **Auto-update preview** remains enabled. The editor permits one request at a time. When the source changes during a build, an old result is hidden and the latest draft runs afterward. Volume, surface area and bounds come from exact OpenCascade geometry.

Orbit and zoom the preview independently of the main scene; **Fit preview to view** reframes it. Theme changes update its background without resetting the saved scene. Preview faces are not saved-revision selection references, and the panel does not export geometry. **Build** uses the normal permission, revision, kernel and commit route. Building is disabled while preview is running or waiting to refresh.

Closing the editor or disabling auto-update requests cancellation of its own preview task; AI/ordinary build tasks are unaffected. The worker retains its 120-second bound. Failure, cancellation and revision changes do not save preview results. Browser mode reports native unavailability. CadQuery/Python and CAD IR v1 do not support this preview yet.

Route: [hook](../../apps/desktop/src/features/model-preview/useModelPreview.ts) → [API](../../apps/desktop/src/features/model-preview/previewApi.ts) → `preview_model` → [service](../../apps/desktop/src-tauri/src/model_preview/mod.rs) → isolated worker. Source is bounded to 60,000 bytes. The service checks project and expected revision before/after building, reserves the existing project task slot and consumes no modifying grant. The [workspace](../../apps/desktop/src-tauri/src/model_preview/workspace.rs) creates unique `project/cache/preview-UUID` and removes only that directory after response/error. A process crash may leave derived cache, never altered source or history.

The worker validates B-Rep and STEP/GLB checksums. A [binary packet](../../apps/desktop/src-tauri/src/model_preview/packet.rs) contains a small JSON-header size, metrics/source SHA-256 and native GLB bytes; vertices are not JSON arrays. The [decoder](../../apps/desktop/src/features/model-preview/previewPacket.ts) validates bounds, version, hash and GLB header. Blob URLs are revoked and stale Three.js objects disposed. No preview STEP/GLB is attached to the project. SQLite, files, revisions, messages and exports remain unchanged; source drafts retain existing UI-preference storage. Canvas loads lazily after a valid response.

Preview uses the exact kernel but does not replace final build validation. This covers manual draft review; AI plan acceptance and structural diff remain a separate next stage.

## Smooth fitting

The button beside “Preview model” fits actual draft bounds while preserving viewing direction. [cameraFit](../../apps/desktop/src/features/model-preview/cameraFit.ts) and [PreviewFraming](../../apps/desktop/src/features/model-preview/PreviewFraming.tsx) interpolate camera position and target together for 380 ms without springs or overshoot. Initial display and reduced motion snap immediately. Repeated clicks retarget from the current pose; a manual gesture interrupts. The previous valid geometry stays visible while a new draft builds, with metrics hidden and a building indicator; it is not presented as the new draft’s result. Errors do not save revisions.

Verify repeated fitting after panning/orbiting, draft thickness changes, and reduced motion. The browser test uses a test IPC bridge and real local CAD-worker outputs, rather than an installed WebView2 session.

[AI candidate review ](ai-candidate-review.md): interactive geometry and structural comparison before explicit acceptance.
