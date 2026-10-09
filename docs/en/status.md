# Forma status

[Documentation](index.md) · [Русский](../ru/status.md) · [Roadmap](roadmap.md)

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
