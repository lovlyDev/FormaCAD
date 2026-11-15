# IGES exchange

[Documentation](index.md) · [Русский](../ru/iges.md) · [M4](modeling-ii.md)

## Import

In the desktop app select one `.igs` or `.iges` through normal file import or drop it into the workspace. Limits are 40 MB and a 120-character filename. Mixed batches are rejected rather than silently selecting the first file. If no project is open, create one first; IGES review opens next. Geometry is not stored until confirmation.

The dialog shows a real 3D preview and before/after measurements: volume, area, closed solids, faces, edges and bounds. Header units are normalized to millimetres. If a model exists, explicitly confirm replacement in a **new** revision. Select “Import IGES” and approve conversion. Earlier revisions/programs are retained; standard undo/redo remain available. Cancel adds neither attachments nor revisions. Creating an empty project on Home is a separate explicit action.

Closed BREP solids are supported. Every solid of a compound file remains inside one named CAD body `imported_body`; no first-component fallback is used. Use explicit component separation for subsequent splitting. Open surfaces, standalone curves, face sets without a solid and partially transferred roots are rejected. Shell sewing and guessed solid reconstruction are not performed. Technical details remain available on errors; the original project stays intact.

The main error message is translated without native codes or debugging text. Expand “Technical details” to inspect the original diagnostic. An unknown failure uses the translated exchange error rather than exposing raw backend output as the explanation.

## Export

Open Export and choose “IGES — exact BREP solids”. With multiple named bodies explicitly choose the whole model or one body. Committed STEP geometry is supported; a selected body requires authored CAD IR and verified assets. STL/OBJ/GLB meshes are not guessed into exact solids.

After permission, OCCT writes BREP-mode IGES in millimetres and reads it back. Volume, area, bounds and solid count are compared. Only a successful round-trip opens the standard system save dialog. Canceling it creates no file, revision or export entry. After saving, the app displays verified measurements. Writable projects receive a separate export-history entry; failure of this optional metadata step is not presented as loss of the saved IGES. Geometry remains unchanged.

IGES does not carry Forma parametric history, authoring links, saved views or PBR appearance. Face/edge counts may change with surface representation and are shown in the report. Closed-solid count must match. Volume, area and bounds tolerance is the greater of `1e-4` in the corresponding units and relative `1e-7`. Unverified geometry is not accepted as a successful exchange.

## Route, storage and transaction

Separate modules are `cad-core/src/iges.cpp`, `native/iges` (CXX/worker/client), `iges` (contract/host) and `features/iges` (UI/capture). `preview_iges_import`, `import_iges` and `export_iges` use the original host and a separate OCCT process. IPC accepts file content and captured revision identity, never a worker executable, arbitrary output paths or renderer-supplied measurements.

The worker receives a fixed file with SHA-256 and size. Response/report are bound to request ID and the hashes/sizes of input, output and preview. The host verifies them and rechecks the project snapshot and cancellation before commit. Preview is generated from the re-read canonical STEP; its body name matches the CAD IR. Late responses cannot retarget another project.

Home passes the chosen file to the explicitly created project through an in-memory IGES store that survives the route remount. It is not written to preferences or disk. Canceling project creation discards the file; a changed project/revision closes the review instead of silently retargeting it.

Import uses the shared modeling staged commit: original IGES, `model-<revision>.step`, `preview-<revision>.glb` and the revision publish together. CAD IR feature `imported_iges` has type `importStep` and a `step_<sha256>` canonical asset reference instead of an external path. The original IGES remains an attachment. A filename collision with different contents requires renaming the incoming file. Parse errors, wrong confirmed hashes, stale revisions and failed round-trips cannot publish staged state.

There are no SQL migrations or locking-algorithm changes; only attachment formats are extended. Issue #32 read-after-write remains with the storage owner. Ambiguous IGES face references are not invented: imported topology does not become authored box/cylinder provenance automatically.

## Validation

C++ covers a 40×20×10 mm box, box/cylinder compound, torus and variable fillet: IGES → BREP → STEP, volume/area and every closed solid. An independent writer checks an inch header. Surface-only, malformed and missing files are rejected. The Rust transaction checks read-only preview/export, complete rollback of malformed content/wrong hashes, exactly one committed revision, original attachment, canonical asset and body rebuilding through its resolver.

Built-app acceptance uses `scripts/acceptance/iges-native.mjs`; `iges-native-save.ps1` controls the actual Windows dialog of its own isolated process. It verifies PID/executable, saves only a new `.iges` under that test runtime and refuses overwrite. IPC is not replaced. Isolated profile/CDP setup is also described in [thicken acceptance](thicken.md). The runtime needs copied `forma-cad.exe`, `forma-cad-worker.exe` and `pid.txt`, embedded assets (`http://tauri.localhost`) and matching OCCT DLLs, including TKDEIGES, on PATH.

```powershell
$env:FORMA_NATIVE_ACCEPTANCE='isolated'
node scripts/acceptance/iges-native.mjs http://127.0.0.1:9336 <runtime>/iges-report.json
```

Native acceptance and CI results are recorded in PR #50. This module does not declare all M4 complete; no installer or release is produced by this task.
