# Selected-body and pair measurements, inspector: original opening

[Русский](storage-measurement-renderer-source.md) · [Host API](../en/storage-folder-host-api.md) · [Previous preview patch](storage-preview-renderer-source.en.md)

Status: **prepared, unapplied; separate agreement required**.
Issue #31 / PR #35, implementation wenlixxq, acceptance lovlyDev.
Base `ecf40d55669701ec482fa3fc60a77a1f805df327` includes applied preview and the
strict AI-review commit fix. The exact [patch](storage-measurement-renderer-source.patch)
does not copy those eight files wholesale: its overlapping App change only passes
the binding into SelectionModelProperties.

## Behavior and data

App passes the opening binding into the inspector and selected-body measurement.
Exact model inspection uses the original FolderSourceRef; its query cache includes
opening, source/task group and commit. Late replies or old cached properties cannot
cross an equal-ID reopen. An explicit missing/mismatched binding disables the exact
query, retaining the mesh estimate without managed fallback.

ReferenceMeasurement and PairMeasurement capture the original binding with their
geometry context. Result identity includes opening/source/task group, revision,
body/query, saved CAD program and primary STEP SHA/size; pairs also include scene token and
captured topology references. Opening changes clear pair slots and reject late
reports even with identical project/revision/body IDs.

Same-opening metadata-only refresh preserves compatible reports/slots without
rewriting a running request's capture. Sessions update only the next request's
context to the fresh commit reference. Native APIs use the capture's own
FolderSourceRef; an explicit foreign source/commit/group override fails with
MEASUREMENT_STALE before IPC. Legacy direct managed callers can omit binding;
production UI supplies it explicitly.

Folder reference/pair panel preferences stay in RAM through source preferences,
without managed preference reads/writes or persisted process-local tokens.
Retired pair capture/clear callbacks are ignored rather than erasing a new
opening's selection. Native algorithms and report provenance remain unchanged.

## All 16 paths

Relative to `apps/desktop/src/`:

1. `features/reference-measurements/measurementCapture.ts`
2. `features/reference-measurements/measurementSession.ts`
3. `features/reference-measurements/useExactReferenceMeasurement.ts`
4. `features/reference-measurements/measurementApi.ts`
5. new `features/project-sources/measurementSource.test.tsx`
6. `app/ModelInspector.tsx`
7. `features/reference-measurements/ExactReferenceMeasurement.tsx`
8. `features/reference-measurements/SelectionModelProperties.tsx`
9. `app/App.tsx`
10. new `features/project-sources/inspectionSource.test.tsx`
11. `features/pair-measurements/pairCapture.ts`
12. `features/pair-measurements/pairApi.ts`
13. `features/pair-measurements/pairSession.ts`
14. `features/pair-measurements/usePairSlots.ts`
15. `features/pair-measurements/PairMeasurement.tsx`
16. new `features/project-sources/pairSource.test.tsx`

## Checks and limits

The isolated copy passed **363 tests /118 files**, production build, lint and i18n (1483 EN/RU messages). Application still requires actual-head checks and CI; test presence alone is not PASS.
All other tracked frontend sources were compared with the base and match. No
Rust/native/Cargo/backend IPC/SQL/version changes or new user text. Fifteen new
renderer tests cover original routing, equal-ID reopen, cache isolation, metadata
continuation, next-request commit, foreign overrides and retired callbacks. These
are fixtures/mocks, not new CAD measurements or installed acceptance.

This does not connect folder face/manual sections, snap, history/import/export,
writer/permission UI or visible folder controls. Stage 4 remains incomplete.
Specification §5 requires agreement before applying the shared source patch.
