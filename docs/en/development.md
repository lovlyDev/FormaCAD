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

## Interface changes
Windows native packaging uses the Cargo-built worker binary. The native Windows Tauri configuration does not additionally declare the staged copy as externalBin: both routes previously installed the same worker name, making file order significant. Verify the generated NSIS script contains exactly one worker File instruction, then smoke-test that exact binary with the bundled DLLs and record its SHA-256. Staged fixtures are not proof of which worker the installer contains. Preserve older versioned installers; do not install the new one as part of local verification.

Add messages to both [en.json](../../apps/desktop/src/i18n/en.json) and [ru.json](../../apps/desktop/src/i18n/ru.json), including labels, errors and accessibility text. Use t(), local date/number formatting and theme colors. Preserve filenames, code, CAD IDs and user/AI messages. Check both languages and themes. Screenshots belong in ignored docs/verification/.
