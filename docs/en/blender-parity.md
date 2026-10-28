# Preparing Blender workflow adaptations

[Documentation](index.md) · [Русский](../ru/blender-parity.md) · [Observations](../parity/blender/README.md) · [Readiness inventory](../parity/blender/readiness.md)

This guide describes research stage M0. It adds no tools to the installed FormaCAD application. Adaptations use independently written TypeScript/React, Rust and C++/OCCT while retaining exact B-Rep geometry and Command API. The [architecture plan](../architecture-2.0.md) remains authoritative for scope and release acceptance.

## Contributor actions

Read the [collaboration rules](collaboration.md), current [status](status.md) and assigned Issue. Confirm the human owner and owned paths. Use a separate branch/worktree from verified main; preserve existing local files. The current task is [M0 #17](https://github.com/lovlyDev/FormaCAD/issues/17).

Run independently authored probes against Blender factory startup and record the exact version and build hash. Set `--python-exit-code 1` for background probes: a normal Blender exit alone does not establish Python success. Check result files and assertions. For the live keymap, wait for GUI initialization in a separate process and terminate only that process. Never overwrite a user scene.

Write notes in your own words: source → observation → FormaCAD decision → module → status and verification. Do not transfer Blender source, decompiled code, assets, manual text, theme or keymap files. Any exceptional source consultation needs a specific reason and log; none was needed for this research.

## Technical route and storage

Blender CLI/Python and REA are development tools only. Probes, facts and logs stay in ignored `.local/m0-preflight/`; portable Node is under its `tools/` and REA under `rea/`. These do not enter the application, Git or user projects. Versioned notes under `docs/parity/blender/` contain no raw keymap tables or user geometry.

Future functionality must keep `React → typed IPC → Rust Command API → staged CAD IR → isolated worker → OCCT → validation → commit`. Observing Blender does not grant plugins or UI direct geometry access. View-only changes must not create geometry revisions. Research tools and optional SDK build dependencies do not make Python a mandatory CAD runtime.

## Environment and limitations

Research used Blender 5.2.3 LTS (`e13b728c6880`), local portable Node 24.22.1, REA 6.8.0 CLI, Rust 1.97.1, MSVC 14.51 and Windows SDK 10.0.26100.0. Build Tools include CMake 4.3.1-msvc1 and Ninja 1.13.2, but they are absent from the ordinary PATH; configure the build process explicitly.

REA was used without decompilation. Its MCP registration plan was inspected without applying it; CLI availability does not mean this chat has a newly connected MCP server. Ghidra was unnecessary for these observations. TypeScript/Python SDKs and their packagers belong to SDK implementation; they were not added to FormaCAD runtime.

Both `wasm32-unknown-unknown` and `wasm32-wasip2` targets were prepared for toolchain 1.97.1. Official Windows binaries wasm-tools 1.262.0 and wit-bindgen-cli 0.63.0 were extracted under `.local` and verified with `--version`. Archive SHA-256: Node `D2046D28C2C3DCD211AC5145C0BF30E2047EF9DFB306693C8C6E2ECCD8B95FA5`, wasm-tools `A39100CADAC426E174E449A80DB669F191B6A2E14F27C369553580CA6E9EA7E3`, wit-bindgen `213393E4A8B4875A3F19856E943E43067D0315141FCA5743E4CFB7795BC9AF14`. Node was additionally checked against official SHASUMS256; the two Wasm hashes record downloaded bytes rather than independent cryptographic attestation. None of these tools changes application dependencies.

A separate local npm package contains jco 1.39.0 and componentize-js 0.23.0, and a Python virtual environment contains componentize-py 0.25.1. CLI startup was checked; compiling SDK examples belongs to M5. Playwright Chromium was prepared in the project-local cache. These build tools were not added to the application workspace.

Synthetic GUI events, background operations and registered metadata do not replace manual UI checks, numerical OCCT parity, performance measurements, installed WebView or other operating systems. Individual probe conditions, including advanced numeric input `=10mm`, are recorded in the topic notes. Do not present them as completed FormaCAD features.

## Handoff checks

Run commands individually from the intended worktree root:

```powershell
npm run check:version
npm run check:i18n
npm run check:docs
npm run check:migrations
npm run lint
npm run build
npm run test
cargo +1.97.1 fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo +1.97.1 clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --locked -j 2 --all-targets -- -D warnings
cargo +1.97.1 test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked -j 2 --lib
```

Prepare MSVC/SDK using the existing `scripts/windows-env.ps1` before Rust checks. Kernel changes additionally require CTest and native worker tests; UI changes require RU/EN e2e in both themes. M0 changes no application code, so it creates no new application version or installer. Research notes never substitute for feature tests, documentation or acceptance.
