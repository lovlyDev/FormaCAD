# Forma CAD

[English](README.md) · [Русский](README.ru.md) · [Download](https://github.com/lovlyDev/FormaCAD/releases) · [Documentation](docs/en/index.md)

I’m building Forma as a local CAD workspace where manual editing and AI work on the same model. It combines a Rust desktop backend, a React/Three.js interface and a Python/CadQuery geometry worker.

**Version 1.2.5.** I redesigned the in-app update download indicator with a rounded progress bar and a clear percentage. See the [release notes](docs/releases/1.2.5.md) and [roadmap](docs/en/roadmap.md).

## Features

- Parametric circles, rectangles, extrusion, translation and booleans; editable feature parameters and a restricted CadQuery fallback.
- Local projects, revision history, restoration, comparison and STEP import.
- STL, OBJ, GLB, 3MF and generated STEP export; camera presets, body visibility, measurements and assembly motion.
- Codex / Claude CLI integration with permissions and local geometry validation.
- Russian and English, light and dark themes, saved workspace state and signed updates.

## Install

Download from [GitHub Releases](https://github.com/lovlyDev/FormaCAD/releases/latest).

| System | Package | Updates |
| --- | --- | --- |
| Windows x64 | EXE installer | In the app |
| macOS Apple Silicon / Intel | DMG | In the app |
| Linux x64 | AppImage | In the app |
| Linux x64 | DEB / RPM | Install the newer package |

Artifacts appear after the [release workflow](.github/workflows/release.yml) succeeds. macOS notarization and updater signing are separate mechanisms: see the [release guide](docs/en/releases.md). AI modeling needs a separately installed supported CLI and Python/CadQuery; see [setup](docs/en/development.md). Viewing saved models does not need an AI account.

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

See [development](docs/en/development.md), [architecture](docs/en/architecture.md), [data and recovery](docs/en/data.md), [release automation](docs/en/releases.md) and [contributing](CONTRIBUTING.md).

## Data and privacy

Projects, settings and backups stay in the application data directory. Updates preserve the application identifier and data paths. Before installation, Forma saves workspace preferences and backs up SQLite. Camera state, display options, panels and drafts are persisted per project. See [data storage](docs/en/data.md).

Update checks contact GitHub. AI requests go to the selected CLI provider when the relevant action is permitted. Project data is not sent to a Forma backend. See [security](SECURITY.md).

## Links

[Repository](https://github.com/lovlyDev/FormaCAD) · [Issues](https://github.com/lovlyDev/FormaCAD/issues) · [Releases](https://github.com/lovlyDev/FormaCAD/releases) · [Actions](https://github.com/lovlyDev/FormaCAD/actions) · [Release notes](docs/releases/1.2.5.md)
