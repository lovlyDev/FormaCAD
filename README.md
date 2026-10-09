# Forma CAD

[English](README.md) · [Русский](README.ru.md) · [Download](https://github.com/lovlyDev/FormaCAD/releases) · [Documentation](docs/en/index.md)

I’m building Forma as a local CAD workspace where manual editing and AI work on the same model. It combines a Rust desktop backend and a React/Three.js interface. Tagged releases build C++/OpenCascade for CAD IR v2 on Windows, macOS, and Linux; Python/CadQuery remains available for older models.

**Consolidated release 1.2.6: the Windows installer is built, verified and updater-signed.** [Windows EXE](https://github.com/lovlyDev/FormaCAD/releases/download/v1.2.6/Forma_1.2.6_x64-setup.exe): 19,017,594 bytes; [release page](https://github.com/lovlyDev/FormaCAD/releases/tag/v1.2.6). Full 2.0 is unfinished. See [evidence and limits](docs/en/status.md), [release notes](docs/releases/1.2.6.md) and the [release procedure](docs/en/releases.md).

## Features

- CAD IR v2 with named parameters and bodies, feature history, suppression, rollback, constrained polygon sketches, exact OpenCascade operations and a restricted CadQuery compatibility path.
- Local projects, revision history, restoration, comparison and STEP import.
- STL, OBJ, GLB, 3MF and generated STEP export; camera presets, body visibility, measurements and assembly motion.
- Codex / Claude / Custom CLI integration with permissions and local geometry validation.
- Russian and English, light and dark themes, saved workspace state and signed updates.

## Install

Download a verified published package from [GitHub Releases](https://github.com/lovlyDev/FormaCAD/releases/latest). [Windows 1.2.6 EXE](https://github.com/lovlyDev/FormaCAD/releases/download/v1.2.6/Forma_1.2.6_x64-setup.exe) is the verified updater-signed Windows package; other platform packages are not established by this release.

| System | Package | Updates |
| --- | --- | --- |
| Windows x64 | EXE installer | In the app |
| macOS Apple Silicon / Intel | DMG | In the app |
| Linux x64 | AppImage | In the app |
| Linux x64 | DEB / RPM | Install the newer package |

Artifacts appear after the [release workflow](.github/workflows/release.yml) succeeds. macOS notarization and updater signing are separate mechanisms: see the [release guide](docs/en/releases.md). AI modeling needs a separately installed supported CLI; tagged builds include the native CAD kernel. Python/CadQuery remains optional for older models; see [setup](docs/en/development.md). Viewing saved models does not need an AI account.

## Develop

Install Node.js 22+, Rust stable and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
npm run desktop
```

Browser preview: `npm run dev`. The browser cannot execute local CLI tools or install updates.

```sh
npm run check:version
npm run check:migrations
npm run check:i18n
npm run check:docs
npm run lint
npm test
npm run test:e2e
npm run build
```

Start with the [documentation index](docs/en/index.md). Detailed guides cover [using Forma](docs/en/using-forma.md), [projects](docs/en/projects.md), [modeling](docs/en/modeling.md), [sketches](docs/en/sketches.md), [AI and permissions](docs/en/ai.md), [viewer and files](docs/en/viewer-and-files.md), [settings and updates](docs/en/settings-and-updates.md), [data and recovery](docs/en/data.md), [architecture](docs/en/architecture.md), [code map](docs/en/internals.md), [development](docs/en/development.md), [release automation](docs/en/releases.md), and [contributing](CONTRIBUTING.md). New work must update both language guides as described in [documentation maintenance](docs/en/documentation.md).

## Data and privacy

Projects, settings and backups stay in the application data directory. Updates preserve the application identifier and data paths. Before installation, Forma saves workspace preferences and backs up SQLite. Camera state, display options, panels and drafts are persisted per project. See [data storage](docs/en/data.md).

Update checks contact GitHub. AI requests go to the selected CLI provider when the relevant action is permitted. Project data is not sent to a Forma backend. See [security](SECURITY.md).

## Links

[Repository](https://github.com/lovlyDev/FormaCAD) · [Issues](https://github.com/lovlyDev/FormaCAD/issues) · [Published releases](https://github.com/lovlyDev/FormaCAD/releases) · [Actions](https://github.com/lovlyDev/FormaCAD/actions) · [All change notes](docs/en/changelog.md)

[Sketch construction lines](docs/en/sketch-construction.md) are available in local development.

Local Sketcher: [holes and contours](docs/en/sketch-profiles.md), [editing and constraints](docs/en/sketch-constraints.md), [parameters](docs/en/cad-parameters.md), [diagnostics](docs/en/sketch-diagnostics.md).


[Local changes 1.2.6](docs/releases/1.2.6.md): sketch editor, shared parameters and adaptive grid; [guide](docs/en/sketches.md).

[Typed AI edits](docs/en/command-api.md): shared parameter/link edits in one validated revision.

[AI candidate review ](docs/en/ai-candidate-review.md).

[Verified CAD selection in AI requests](docs/en/ai-selection-context.md).

[Checked model application transactions](docs/en/model-apply-transactions.md): shared editor/AI/command host route, permissions, cancellation, and stale-result checks.

[Persistence after commit](docs/en/postcommit-persistence.md): a failed mirror does not turn a durable revision into a failed model change.

[Operation-owned topology references](docs/en/topology-references.md): exact references for supported box edges through rigid transforms.

[Model history, storage and native operations](docs/en/project-history.md) · [Storage](docs/en/project-storage-v2.md) · [Parity](docs/en/modeling-parity.md)

[Project access](docs/en/project-access.md) · [Operations on STEP](docs/en/imported-step-features.md)

[Exact model sections](docs/en/model-sections.md) · [Heavy CAD queue](docs/en/cad-task-queue.md)

[Sections from a selected CAD face](docs/en/face-sections.md): signed offsets, verified planes and whole-STEP intersections.

[Exact pair measurements](docs/en/pair-measurements.md): minimum distances between captured CAD elements and outward-normal/acute-line angles within one saved body.

[Exact selection measurements](docs/en/reference-measurements.md): included in the consolidated 1.2.6 source; the Windows package is verified.

[Circular constructor measurements](docs/en/circular-measurements.md): exact authored radius and diameter; included in the consolidated 1.2.6 source.

[Pure project snapshots](docs/en/project-snapshots.md): consistent managed-project/history reads without implicit recovery.


[Two-PC collaboration](docs/en/collaboration.md): source through Git, user project data kept separate.
