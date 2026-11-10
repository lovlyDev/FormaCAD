# M4 — modeling tools II and data

[Documentation](index.md) · [Русский](../ru/modeling-ii.md) · [Plan](../architecture-2.0.md)

Work starts at main 8cb40fd (1.7.0-beta.1), branch task/m4-modeling-tools, Issue #21. This records implementation status and acceptance requirements; it does not declare M4 complete.

## Modules and acceptance

Separate modules cover split, thicken, mirror features, offset profile, variable fillet, IGES, PBR/glTF appearance, geometry table, history timeline, saved views and smart selection. Each feature needs the actual C++/Rust, staged Command API and UI route, bilingual documentation, measurements, BREP validation, exchange round-trip where applicable and transaction rollback.

History changes must be explicit; ambiguous references require manual choice. Storage and recovery Issues #27–#34 belong to wenlixxq. No placeholder controls are introduced.

## Empty-intersection diagnostic from M3

Disjoint solids and face/edge-only contact produce no common solid. The kernel returns EMPTY_INTERSECTION; the preview displays “The intersection is empty.” while preserving technical details. Other invalid-volume failures retain their original meaning.

Route: cad-core/src/booleans.cpp → native Shape/worker report → features/model-preview/previewError.ts → ru.json/en.json. Invalid preview still disables Build; a failed staged transaction must leave the saved revision unchanged. No persisted schema is added.

C++ regression checks cover disjoint solids, face contact and a valid self-intersection of a 40×20×10 box, volume 8000 mm³. Transaction regression checks the failure code and unchanged snapshot. Frontend tests check the translatable key and retained details. Built-app/real-IPC acceptance has not run yet and is not claimed as passed.

## Read-after-write conflict

Source audit at 8cb40fd identifies a window in list_projects/get: the SQL commit exposes a pending journal while the writer still owns ProjectLock. Reader reconciliation can attempt the same immediate writer lock. Current inspect_model uses a pure snapshot and returns PROJECT_SNAPSHOT_RECOVERY_REQUIRED for pending recovery; its reported writer-lock error has not been reproduced independently.

The report and explicit clarification were handed to the storage owner in [Issue #32](https://github.com/lovlyDev/FormaCAD/issues/32#issuecomment-6768465150). Storage source files were not changed; no retry delay masks the conflict.

## Delivery boundaries

Every operation needs acceptance through the built application and real IPC/host/OCCT with numeric results, Russian/English and both themes. PR remains draft until M4 is complete. No version, lockfile, installer or release changes.
