# Check, release and maintenance scripts

[Documentation](index.md) · [Русский](../ru/maintainer-scripts.md) · [Development](development.md) · [Releases](releases.md)

This page covers helper scripts that used to live outside the repository: personal `.cmd`/`.ps1` files and temporary scripts from individual sessions. They now live in `scripts/` in generalised form, with no absolute paths and no secrets. Keys, `.pem` files, `updater.key`, `.env` and tokens are still kept outside Git only (`.secrets/` and `.local/` are ignored).

## Summary

| Script | Purpose | Who runs it |
| --- | --- | --- |
| [`scripts/windows/verify.ps1`](../../scripts/windows/verify.ps1) | All pre-PR checks in one command; optionally builds the Windows installer | any contributor, AI agents |
| [`scripts/verify-updater-signatures.mjs`](../../scripts/verify-updater-signatures.mjs) | Verifies updater signatures (`*.sig`) with the public key from `tauri.conf.json` and checks `SHA256SUMS` | release owner |
| [`scripts/maintainers/github-app-admin.mjs`](../../scripts/maintainers/github-app-admin.mjs) | Maintains the "FormaCAD Plugins" GitHub App: installations, webhook deliveries, redelivery, secret rotation | catalog admin |
| [`scripts/maintainers/site-docs.py`](../../scripts/maintainers/site-docs.py) | Converts the plugin guides into formacad.ru site articles | site admin |

## `scripts/windows/verify.ps1`

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/windows/verify.ps1
```

Order: version check → `check:i18n` → `check:docs` → `check:migrations` → ESLint → frontend typecheck and build → Vitest → `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test --lib`. The script stops at the first failure and prints the step name and exit code. Before the Rust steps it loads `scripts/windows-env.ps1`, which selects a complete x64 MSVC toolchain and Windows SDK.

Parameters:

- `-Install` — run `npm ci` first.
- `-CargoTargetDir <dir>` — a shared Cargo target directory, for example one for several worktrees (rebuilding OCCT-dependent crates is slow).
- `-TempDir <dir>` — `TMP`/`TEMP` for this run, if the system drive is short of space.
- `-Installer -OcctRoot <dir>` — build the installer with `scripts/build-native-windows.ps1` after the checks. OCCT is built by `scripts/build-occt-windows.ps1` into `.local/occt-build`.
- `-UpdaterKeyFile <file>` — the updater signing key, for example `.secrets/updater.key`. The key is read only into process memory and never printed or written. Without this parameter the `TAURI_SIGNING_PRIVATE_KEY` variable is used; without that, the installer is built without an update signature and the script warns.

Run the native worker and C++ tests (`cargo test --features native-occt --test native_worker`, `ctest` in `cad-core`) and Playwright e2e separately when the kernel or UI changes; see [development](development.md).

Limits: Windows x64 only. The script neither installs the app nor publishes anything.

## `scripts/verify-updater-signatures.mjs`

```bash
node scripts/verify-updater-signatures.mjs .release-artifacts
```

For every `<file>.sig` in the directory (recursively) the script verifies the minisign/Ed25519 signature with `plugins.updater.pubkey` from `apps/desktop/src-tauri/tauri.conf.json`. It supports both raw-byte and prehashed BLAKE2b-512 signatures. If `SHA256SUMS` is present, it then checks the hash of every listed file. It exits with 1 on any failure. A second argument selects another `tauri.conf.json`, for example from a release branch.

Release order: the Release workflow builds installers → `scripts/release-artifacts.mjs collect` and `merge` produce `latest.json` and `SHA256SUMS` → this script checks the downloaded artifacts before publication. Publication only on the user's explicit instruction (see [AGENTS.md](../../AGENTS.md) and [releases](releases.md)).

## `scripts/maintainers/github-app-admin.mjs`

```bash
GITHUB_APP_ID=<id> GITHUB_APP_PRIVATE_KEY_PATH=<path to .pem outside Git> node scripts/maintainers/github-app-admin.mjs installations
```

Commands: `installations`, `deliveries [n]`, `redeliver <delivery id>`, `set-secret <file>`. The script signs a short-lived App JWT (5 minutes) with the private key from the given file and calls the GitHub REST API. Use it when the catalog missed a plugin release event: list deliveries, redeliver the one you need, and update the webhook secret in the App settings when it is rotated. The same secret must also be set on the catalog server.

The catalog and account server is a separate repository, [FormaCAD-Server](https://github.com/lovlyDev/FormaCAD-Server). Server deployment and diagnostic scripts (containers, `.env`, certificates) live there and are not moved here.

## `scripts/maintainers/site-docs.py`

```bash
python scripts/maintainers/site-docs.py <site content dir> <commit sha> [owner/repo]
```

Takes `docs/{ru,en}/{plugins,plugin-authoring,plugin-platform}.md` from the current checkout, removes the repository navigation line and rewrites links. Links between these guides and to `command-api.md` become site paths `/docs/...` with site-style anchors; an unknown anchor is an error. Every other relative link points to GitHub at the given commit. Output: `<dir>/<locale>/<article>.md`. The site is built in FormaCAD-Server.

## Release documentation rules

- Release notes, [status](status.md) and [releases](releases.md) describe the platform and API but **never list or announce specific catalog plugins**, including examples by project authors. Plugins are community work and the catalog checks them itself.
- Scripts must not contain absolute paths of personal machines, keys or tokens. Paths are passed as parameters or environment variables.

## Verification

`npm run check:docs` checks this guide's links. `verify-updater-signatures.mjs` was tested on the published 1.2.6 installer, and `site-docs.py` on the current guides.
