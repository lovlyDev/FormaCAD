# Development
[Documentation](index.md) · [Русский](../ru/development.md)

## Prerequisites
Node.js 22+, Rust stable and [Tauri system dependencies](https://v2.tauri.app/start/prerequisites/). Windows needs Visual C++ Build Tools and WebView2; macOS needs Xcode command-line tools. macOS packages target 11+.

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
cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --check
cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
python scripts/test_cad.py
npm run package
~~~

CAD tests require the configured Python. Set FORMA_TEST_PYTHON for Rust geometry integration. Build installers on their target OS; [GitHub workflows](../../.github/workflows/release.yml) provide those machines. Signed releases follow the [release guide](releases.md).

## Interface changes
Add messages to both [en.json](../../apps/desktop/src/i18n/en.json) and [ru.json](../../apps/desktop/src/i18n/ru.json), including labels, errors and accessibility text. Use t(), local date/number formatting and theme colors. Preserve filenames, code, CAD IDs and user/AI messages. Check both languages and themes. Screenshots belong in ignored docs/verification/.
