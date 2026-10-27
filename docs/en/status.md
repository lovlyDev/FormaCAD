# Forma status

[Documentation](index.md) · [Русский](../ru/status.md) · [Roadmap](roadmap.md)

## Changes in 1.4.0

Plugins: installing `.cadplugin` from a file and from the [plugins.formacad.ru](https://plugins.formacad.ru) catalog with signature checks, a Wasmtime sandbox with permissions, a consent window, palette actions, plugin log and settings. Details and verification scope: [1.4.0 release](../releases/1.4.0.md), the [plugins article](plugins.md) and the [author guide](plugin-authoring.md). The Windows x64 installer is published: [v1.4.0 release](https://github.com/lovlyDev/FormaCAD/releases/tag/v1.4.0).

## Changes in 1.3.3

Optional FormaCAD account sign-in from the app through the system browser (VK ID, Yandex ID, GitHub, passkey), signed rights with a bounded offline period, fixed project deletion on Windows and merged wenlixxq work on history sessions. Details and verification scope: [1.3.3 release](../releases/1.3.3.md) and the [account guide](account.md). The Windows x64 installer is published: [v1.3.3 release](https://github.com/lovlyDev/FormaCAD/releases/tag/v1.3.3).

## Changes in 1.3.2

The desktop is separated from OCCT: the UI builds with `native-worker-client`, and the separate CAD process with `native-occt`. Building, STEP, measurements and sections retain their native routes. Readiness runs a real unit box inside the worker with a time limit and translates unavailable-kernel errors. Windows packaging includes verified Microsoft runtime files; the build rejects unknown host imports and unsuitable DLLs. This addresses confirmed packaging deficiencies in 1.3.0 without claiming the cause of the error on another PC.

`FORMA_DATA_DIR` redirects the entire profile through Tauri 2.12: database, projects, logs, window state and the default WebView directory. Invalid paths fail before profile creation. Appearance, privacy and permissions settings no longer launch external health commands. Usage, storage and verification are described in the [development guide](development.md).

These changes do not establish complete architectural 2.0 readiness, installed WebView acceptance on clean Windows, Smart App Control compatibility, NAS guarantees or other operating systems. Previous installers remain preserved; publishing source code does not install the application. Verification of the specific new artifact is reported separately in the [1.3.2 release](../releases/1.3.2.md).

## Verified 1.3.0 package

The Windows 1.3.0 installer is built and verified: 19,027,793 bytes, SHA-256 `FA7923F235E64D19B3DE6FDB44EC035EC344646AA06016BEDA247AF363010D00`. Its updater signature was independently verified against the application public key. The app, one worker, 25 OCCT DLLs and both separately named licenses were extracted without executing the installer and their contents verified. App comparison permits only the exact standard Tauri `UNK` to `NSS` token patch; all other bytes match the build output.

Five read routes use consistent snapshots and revalidate sources: history, attachments, STEP properties, previews and manual sections. Checks: native351/30targets (two existing ignored), frontend197/71files, full browser72PASS; fresh packaged preview/section browser3PASS. Strict Clippy, no-default, fmt, lint, i18n1149, docs106 and migrations passed. Extracted worker:64 analytical cases and19 expected rejections. The Chromium harness mocks host IPC and does not establish installed WebView or macOS/Linux acceptance. See [snapshots](project-snapshots.md), [1.3.0 notes](../releases/1.3.0.md) and [two-PC workflow](collaboration.md). Full architectural2.0 is unfinished; project head lovlyDev reviews the PR and decides whether to merge. Installation was not performed.

[Windows EXE1.3.0](https://github.com/lovlyDev/FormaCAD/releases/download/v1.3.0/Forma_1.3.0_x64-setup.exe) · [GitHub release](https://github.com/lovlyDev/FormaCAD/releases/tag/v1.3.0).


Publication of1.3.0 is confirmed: downloaded EXE,signature and updater metadata match the local files; both Ed25519 signatures verify. [GitHub Quality](https://github.com/lovlyDev/FormaCAD/actions/runs/38170203431) succeeded for source commit `0eecb136338f772b1fe773cf5e9e6f67b2d1c0d1`. CI:37 browserPASS/35skipped without native fixtures; local full suite72PASS. The release comes from the task branch; [PR#1](https://github.com/lovlyDev/FormaCAD/pull/1) remains unmerged and main stays at1.2.6 until review and a decision by project head lovlyDev.

## Consolidated release 1.2.6

All current improvements are consolidated into **1.2.6**. The new Windows installer is built, verified and updater-signed. macOS/Linux packages for this release are not verified; see [release notes](../releases/1.2.6.md).

## Included behavior

- CAD IR v2, named bodies and parameters, dependencies, suppression, rollback, validated native operations and STEP exchange.
- Sketch holes, constraints, construction lines and shared coordinate parameters; diagnostics, smooth editing and 3D draft preview.
- Shared editor UI, repeated motion, workspace persistence, adaptive grid and CAD selection/cursor fixes.
- Revision-bound AI command batches, candidate review, explicit permissions, checked host application and postcommit persistence.
- Supported box/circular constructor references; exact selection and pair measurements; whole-STEP sections and offsets from planar CAD faces.
- Model history and undo/redo, read-only access and copies, portable packages and coherent pure project/history snapshots.
- Russian/English, light/dark themes and updater support.

Detailed workflows and limits are in the [index](index.md). [Two-PC collaboration](collaboration.md) separates Git source work from user project data.

## Verification and remaining work

Windows EXE: **19,017,594 bytes**, SHA-256 `B202C15CFF26F3F6B80735B5F7274EFAEA4A51CC6DE519DB08C93D7907184708`; FileVersion and ProductVersion are 1.2.6. The package contains exactly one worker (SHA-256 `3040C0BD7CF9402E2F5014A85E32D298C897EACFA2C0E9900E17608E6FEC63FF`) and 25 OCCT DLLs. The packaged worker passed feature checks, 64 analytic cases and 19 safe rejections. Updater signing was verified by the Tauri CLI and independently for both Ed25519 payload/comment signatures against the application public key; `latest.json` is Windows-only. This does not establish Windows Authenticode, installation on the user computer or macOS/Linux packages.

Full source checks before version-metadata-only consolidation: native — 328 tests across 30 targets, two service helpers ignored; frontend — 197 tests across 71 files; strict Clippy, no-default-features and fmt passed. The final Chromium run of the new 1.2.6 UI passed 8/8 scenarios in 2.2 minutes, in Russian and English with both themes and genuine packaged-worker reports. The browser harness mocks the host boundary; this is not installed-WebView verification.

[Windows EXE](https://github.com/lovlyDev/FormaCAD/releases/download/v1.2.6/Forma_1.2.6_x64-setup.exe) · [GitHub release](https://github.com/lovlyDev/FormaCAD/releases/tag/v1.2.6). The release owner publishes after artifact verification.

**2.0 remains unfinished.** Arbitrary-folder in-place authority, all-consumer repository sessions, media-loss/NAS guarantees, general topology remapping, scalable arc sketching and installed acceptance on every platform are not established. Copying a managed project with validated history differs from converting a genuine 1.1 project from its last verified STEP.
