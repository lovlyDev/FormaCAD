# Forma documentation

[README](../../README.md) · [Русский](../ru/index.md) · [Project status](status.md)

These guides describe the consolidated 1.2.6 candidate and its limits. Final packaging and publication are tracked in [status](status.md); [release notes](../releases/1.2.6.md) supplement topical documentation.

## For users

- [Exact pair measurements](pair-measurements.md): two captured elements of one saved body, minimum distance and explicit angle conventions.

- [Sections from a selected CAD face](face-sections.md): authoritative planar-face references, signed offsets and whole-STEP results.

- [Circular constructor measurements](circular-measurements.md): exact radius, diameter, cap planes and tracked cylinder references.

- [Exact selection measurements](reference-measurements.md): authored references, checked inputs, units and read-only evaluation.

- [Using Forma](using-forma.md): dashboard, workspace, settings, themes, and languages.
- [Projects, history, and transfer](projects.md): cards, pinning, deletion, revisions, and `.cadpack`.
- [Modeling and CAD IR v2](modeling.md): parameters, features, bodies, validation, and compatibility.
- [Checked host model transactions](model-apply-transactions.md): shared application route, permissions, temporary output, cancellation, and commit checks.
- [Persistence after commit](postcommit-persistence.md): guarded mirrors, recoverable publication and revision notifications.
- [2D sketches and constraints](sketches.md): drawing, planes, constraints, and limits.
- [AI assistant and permissions](ai.md): providers, context, confirmations, and failures.
- [Verified CAD selection in AI requests](ai-selection-context.md): host checks, sealed artifacts, and revision-scoped topology.
- [Operation-owned topology references](topology-references.md): supported transformed box edges, reference validation, and limits.
- [3D viewer, measurements, and files](viewer-and-files.md): camera, selection, import, and export.
- [Data and recovery](data.md): directories, backups, and persisted preferences.
- [Settings, localization, and updates](settings-and-updates.md): themes, language, versions, and persistence.
- [Releases and updates](releases.md): publication state and update installation.

## For developers

- [Architecture](architecture.md): React, Rust, worker, and CAD kernel boundaries.
- [Code map and data flows](internals.md): modules and the model-edit path.
- [Development and local builds](development.md): environment, tests, and packaging.
- [Maintaining documentation](documentation.md): the required process after every feature.
- [Roadmap and references](roadmap.md): work still open before 2.0.
- [Contributing](../../CONTRIBUTING.md) and [security](../../SECURITY.md).
- [Change index](changelog.md): every version note in one place.

- [Sketch construction lines](sketch-construction.md): diagonal dimensions, constraints, storage and limits.

- [Sketch profiles and holes](sketch-profiles.md)
- [Sketch diagnostics](sketch-diagnostics.md)

- [Point constraints and profile editing](sketch-constraints.md)
- [Named parameters and dimensions](cad-parameters.md)

- [Model editor](model-editor.md)
- [Exact draft model preview](model-preview.md)




[Typed AI edits and link commands ](command-api.md): revision-bound batches and explicit coordinate unlinking without movement.

[AI candidate review ](ai-candidate-review.md): interactive geometry and structural comparison before explicit acceptance.

- [Model undo and redo](project-history.md)

- [Self-contained project storage](project-storage-v2.md)

- [Native operations and parity](modeling-parity.md)

- [Operations on imported STEP](imported-step-features.md)

- [Project access and read-only mode](project-access.md)

- [Exact model sections](model-sections.md): viewport controls, saved view preferences, clipping and native curve lengths.
- [Heavy CAD task queue](cad-task-queue.md): global worker slot, waiting and cancellation.

- [Pure project snapshots](project-snapshots.md): consistent reads for copies, packages and measurements without implicit recovery.


- [Two-PC collaboration](collaboration.md): Git source and separate user data.
