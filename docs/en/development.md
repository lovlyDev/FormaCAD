# Development
[Documentation](index.md) · [Русский](../ru/development.md)

## Prerequisites
Node.js 22+, Rust 1.97.1 and [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/). Windows needs Visual C++ Build Tools and WebView2; macOS needs Xcode command-line tools. macOS packages target 11+.

Ubuntu 22.04+:
~~~sh
sudo apt-get update
sudo apt-get install -y build-essential curl wget file libssl-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libfuse2 rpm
npm ci
npm run desktop
~~~

On Windows with multiple Visual Studio installations:
~~~powershell
. ./scripts/windows-env.ps1
npm run desktop
~~~
The [toolchain helper](../../scripts/windows-env.ps1) selects complete C++ headers and libraries.

## Reproducible Rust
Local Rust checks and the verified Windows package use Rust 1.97.1 and Clippy 0.1.97. [Quality](../../.github/workflows/ci.yml) and [Release](../../.github/workflows/release.yml) pin that version instead of floating `stable`; strict `-D warnings` remains enabled. Matching the checked environment does not replace a successful CI run on each platform.

After installing rustup, run these commands from your own clone:
~~~sh
rustup toolchain install 1.97.1 --profile minimal --component rustfmt --component clippy
rustup override set 1.97.1
rustc --version
clippy-driver --version
~~~
The override applies to this clone; other projects' global toolchains need not change. Expected versions are `rustc 1.97.1` and `clippy 0.1.97`. Tauri and npm build commands use the Cargo selected here.

Upgrade Rust in a separately owned task: agree on the new fixed version in both workflows and this guide, run rustfmt, strict Clippy for all targets, locked Rust tests, and native CAD checks in a prepared environment. Newer Clippy releases can introduce additional warnings; fix their causes while preserving strict checks, and verify CI results before merging.

CI Rust tests use `runner.temp` as `TMPDIR`: macOS's system temporary path can traverse the `/var` symlink. This selects a real root for test projects while retaining the application's symlink rejection and its negative tests. For local macOS runs, `TMPDIR` can point to an existing checked directory without symlinks; do not disable safe-path validation to accommodate a test environment.

## CAD environment
Use Python 3.12 and [requirements-cad.txt](../../requirements-cad.txt):
~~~sh
python -m venv .venv
# Windows
.venv/Scripts/python -m pip install -r requirements-cad.txt
# macOS / Linux
.venv/bin/python -m pip install -r requirements-cad.txt
~~~
Choose the interpreter in Settings → CAD environment. Its path survives updates. FORMA_PYTHON is an optional fallback. Install and authenticate the CLI separately. Desktop launches search common Homebrew and local binary directories without executing shell profiles.

## Checks
~~~sh
npm run check:version
npm run check:i18n
npm run check:docs
npm run lint
npm test
npx playwright install chromium
npm run test:e2e
cargo +1.97.1 fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo +1.97.1 clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo +1.97.1 test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
python scripts/test_cad.py
npm run package
~~~

CAD tests require the configured Python. Set FORMA_TEST_PYTHON for Rust geometry integration. Build installers on their target OS; [GitHub workflows](../../.github/workflows/release.yml) provide those machines. Signed releases follow the [release guide](releases.md).

Browser checks use real user interactions. The shared `e2e/customSelect.ts` waits for animations on the control and its containers, selects an option from the open custom menu, then verifies the closed trigger's resulting text. Do not replace this route with `selectOption` on a combobox button, forced clicks, or direct state changes. Editor checks retain the 32 px height, alignment, and repeated animation assertions; row screenshots follow full viewport visibility checks without a second locator scroll. Chromium with SwiftShader does not replace installed WebView acceptance.

Browser automation uses Chromium's explicit SwiftShader **WebGL fallback** (`--use-gl=angle --use-angle=swiftshader-webgl --enable-unsafe-swiftshader`), following the [Chromium SwiftShader guide](https://chromium.googlesource.com/chromium/src/+/main/docs/gpu/swiftshader.md). This keeps actual software WebGL2 while using software compositing for ordinary page content. The previous GPU-replacement mode (`--use-angle=swiftshader`) delayed the first `Page.captureScreenshot` by roughly 56 seconds in the reproduced Windows case; the same disclosure test with fallback took 28 ms for that capture. This is a browser-test configuration change, with the original deadlines, real animation assertions and screenshots preserved. `e2e/webgl-renderer.spec.ts` verifies a genuine WebGL2 context, the SwiftShader renderer and a shader-drawn triangle's pixel values. The unsafe fallback opt-in is confined to tests of this trusted local application; it does not change the shipped app renderer or prove installed WebView or every CI platform acceptance.

## Interface changes
Windows native packaging builds the worker separately with `native-occt` and includes it through `externalBin`. The desktop builds only with `native-worker-client`: Tauri excludes the Cargo worker binary, which requires `native-occt`, from this step. Therefore `externalBin` is required here and remains the only packaging route for the worker; duplicating the two routes is invalid. Verify the generated NSIS script contains exactly one worker File instruction, then smoke-test that exact binary with the bundled DLLs and record its SHA-256. Staged fixtures are not proof of which worker the installer contains. Preserve older versioned installers; do not install the new one as part of local verification.

Add messages to both [en.json](../../apps/desktop/src/i18n/en.json) and [ru.json](../../apps/desktop/src/i18n/ru.json), including labels, errors and accessibility text. Use t(), local date/number formatting and theme colors. Preserve filenames, code, CAD IDs and user/AI messages. Check both languages and themes. Screenshots belong in ignored docs/verification/.

## Windows CAD loader isolation

The desktop builds with `native-worker-client`: pure Rust clients and checked contracts without CXX/OCCT. The separate `forma-cad-worker` builds with `native-occt`, which includes the client contract and kernel. Build, STEP import, properties, measurements and sections still call the worker; disabling those capabilities is not a way to obtain an independent host. The bridge lives in `src/native/kernel.rs`; pure measurement input validation lives in `native/reference_measurements/inputs.rs`.

Windows packaging builds the worker separately, then the desktop with the client feature. [Runtime staging](../../scripts/stage-windows-runtime.ps1) accepts DLLs only from the licensed Visual Studio `x64/Microsoft.VC143.CRT` / `x64/Microsoft.VC145.CRT` directory, checks Microsoft signatures and x64 PE headers before copying, verifies SHA-256 and refuses to replace differing staged DLLs. Pass `RuntimeDirectory` to `build-native-windows.ps1` to select an explicit source. Package resources place the DLLs next to the worker; users need neither Visual Studio nor a runtime network download at launch. Local runtime distribution requires compliance with the Visual Studio license and shipping runtime security fixes with app updates; see [Microsoft](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files).

Validation: client-only build without OCCT_ROOT, kernel tests and actual worker scenarios, direct/transitive imports of the fresh extracted desktop (no OCCT), signatures and packaged runtime dependency closure. Actual UI startup with missing CAD DLLs and installed-app acceptance on clean Windows remain separate gates. A successful source build does not establish those gates. App/OCCT Authenticode and Smart App Control policy are distinct from updater signatures. This fix does not change the existing 1.3.0 package.

Readiness uses `--health-check`: the worker validates an actual unit box in memory and returns bounded JSON without writing STEP, GLB or project data. The host checks successful process exit, protocol, engine, application version and response schema. Executable existence alone is not readiness. `NATIVE_KERNEL_UNAVAILABLE` is localized at display boundaries; technical diagnostics are retained separately. The probe has an eight-second timeout and uses existing process-tree cleanup.

The probe runs in an owned temporary directory and removes it after the process finishes. This isolates writes from an old worker that does not recognize `--health-check` from the installation and user project directories. An incompatible response remains a readiness failure. Failure to remove the temporary directory is also reported as a technical readiness error.

Licensed VC143 and VC145 runtime families are supported. Before copying, every DLL version must be at least that of the selected compiler; this also applies to an explicitly supplied `RuntimeDirectory`. Microsoft documents [v14 compatibility across MSVC 2015–2026](https://learn.microsoft.com/en-us/cpp/porting/binary-compat-2015-2017). A stale runtime beside a newer worker is not accepted merely because its DLL names match.

## Separate application profile

`FORMA_DATA_DIR` sets an absolute directory for the entire Forma profile: the `forma.sqlite` database, projects, logs, window settings and local WebView profile. It is intended for separate local profiles and verification; without it, the standard app directories remain in use. Set a separate folder before launching the EXE to start a new profile. Reuse that path to retain settings; a different path opens an independent profile. Existing default-profile data is not automatically copied.

The `app_profile` module validates the path before creating the Tauri Builder, windows or plugins. Empty or relative paths fail startup without falling back to the default profile. Tauri 2.12 uses the official [AppDirectoriesOverride through Context](https://docs.rs/tauri/2.12.0/tauri/struct.Context.html#method.config_mut): config/data/local-data resolve to the selected folder, cache to `caches`, and logs to `logs`. The database also derives its path from the Tauri resolver. The standard window-state plugin remains enabled; WebView files and `.window-state.json` belong to that profile. The Tauri runtime/macros/utils/build family is refreshed together; Windows Known Folders are not redirected by environment substitution.

Validation covers rejection before writes for invalid paths, the actual resolver and window-state saving through MockRuntime, and SQLite migrations and read/write operations inside the selected profile. These checks do not replace an actual WebView launch and restored window from the extracted EXE. Use a disposable Windows profile/VM, before/after directory snapshots and two launches with state restoration for real-process acceptance. App-owned storage is redirected; this does not guarantee isolation of Windows logs or all internal browser/system writes. A profile directory may contain credentials and user data: do not commit it or send it with source code.

Settings environment checks run only on the AI and CAD pages. Appearance, privacy and permission pages do not launch external CLIs for unused results; permission auditing keeps its own query route. Browser verification uses actual buttons and confirms no environment query before entering CAD in both languages and themes.

Windows integration tests using the actual Tauri MockRuntime also receive an embedded Common Controls v6 manifest. Tauri adds its standard resource only to application binaries, while `muda` imports `TaskDialogIndirect`, which the older system Common Controls library does not export. Without a manifest, the test exited with `0xc0000139` before its harness started. Test-only linker arguments correct loading without replacing the production application manifest.

On Windows, an external `WEBVIEW2_USER_DATA_FOLDER` or WebView2 policy may override the browser directory. Check these overrides separately for controlled acceptance, following [Microsoft documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl). App profile routing is not a sandbox for external AI tools: their health commands run separately when the corresponding pages are opened.
