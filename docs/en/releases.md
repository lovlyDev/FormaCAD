# Releases and updates
[Documentation](index.md) · [Русский](../ru/releases.md)

Preparation and publication of the consolidated **1.2.6** release are authorized. The new Windows package is locally verified and updater-signed; Verify source, versions and the local artifact first, then publish that exact commit. This does not claim 2.0 readiness; see [status](status.md).

## One-time GitHub setup
Repository: [lovlyDev/FormaCAD](https://github.com/lovlyDev/FormaCAD). Upload source files and enable Actions. The repository must be public for anonymous update downloads; no GitHub access token is embedded in the app.

In Settings → Secrets and variables → Actions add:
| Name | Type | Value |
| --- | --- | --- |
| TAURI_SIGNING_PRIVATE_KEY | Secret | Entire contents of the private updater key |
| TAURI_SIGNING_PRIVATE_KEY_PASSWORD | Secret | Key password, if set |
| TAURI_UPDATER_PUBLIC_KEY | Variable, optional | Public key; defaults to the committed config |

The initial local key was generated under .secrets/updater.key; its public counterpart is embedded in [tauri.conf.json](../../apps/desktop/src-tauri/tauri.conf.json). The private file and generation log are ignored. Keep an offline backup and never commit or regenerate it for each version: existing installations trust that key.

For a new distribution identity, generate keys with:
~~~sh
npm run tauri -w apps/desktop -- signer generate --ci -w ../../.secrets/updater.key
~~~
Do not overwrite a key already used by released installations. [Tauri updater reference](https://v2.tauri.app/plugin/updater/).

## Publish the verified consolidated release
~~~sh
npm run version:set -- 1.2.6
~~~
Run this procedure only against the verified release source and artifact. The command synchronizes npm, the lockfile, Cargo and Tauri. Prepare detailed bilingual feature documentation and `docs/releases/1.2.6.md`, run all checks, and build a local Windows installer. Publication is authorized; the commit and tag must identify the verified source:
~~~sh
npm run check:version
git tag v1.2.6
git push origin main
git push origin v1.2.6
~~~
Use the actual default branch if it differs. Do not tag or upload an unverified build.

[Release](../../.github/workflows/release.yml) runs [Quality](../../.github/workflows/ci.yml), then builds native CAD packages for Windows x64, Linux x64, macOS ARM64 and Intel. Windows uses [its OCCT build](../../scripts/build-occt-windows.ps1); macOS and Linux use [a pinned static OCCT build](../../scripts/build-native-unix.sh). Each worker must pass a geometry and checksum [smoke check](../../scripts/check-native-unix.py) before packaging. Packages are signed for the updater, collected without filename collisions, and combined into latest.json and SHA256SUMS. The release stays a draft until every asset is uploaded. Existing releases are never overwritten; resolve an incomplete draft before rerunning its tag.

Quality checks run automatically for pull requests targeting `main` and pushes to `main`; they do not publish packages. Quality also supports manual Actions runs and calls from the release workflow. Installer packaging and automatic publication remain tag-triggered: only a newly created `vX.Y.Z` tag starts the Release workflow, whose version must match the application. Manual Quality runs can include the native matrix; `nativeCadMatrix` selects `{ "os", "target" }` entries. Updating or deleting an existing tag does not publish a release. Separately authorized manual publication of a verified Windows package is described below.

## Current Windows 1.2.6 route and the automated matrix

The current consolidated release uses a separate Windows route: build the local EXE from the verified commit, check its packaged worker and DLLs, calculate SHA-256, sign it with the existing updater key, then manually upload the verified EXE, signature and metadata to the authorized release. The local EXE and updater signature are verified; the release owner handles manual upload/publication. See [artifact evidence](status.md). This Windows-only route does not establish verified macOS/Linux packages or a completed four-platform matrix.

The Release workflow above remains the route for a future automated matrix triggered by a new tag. It builds platforms, signs updater artifacts and merges metadata. Manually uploading the current Windows package does not prove that matrix passed and does not install anything on the user computer.

## Application behavior
Forma checks at startup and every six hours. A newer release opens an Install / Later dialog when no operation or editor is active. Later defers that prompt; the update button remains available. Installation verifies the artifact signature, saves preferences and creates a SQLite backup before invoking the platform installer. Windows restarts through NSIS; macOS and AppImage relaunch after replacement.

Linux DEB/RPM installations use new packages rather than in-place self-update. A missing release, offline connection or GitHub failure does not block startup. To move to the consolidated release manually, use the verified 1.2.6 installer after publication. The application identity and data paths remain unchanged; back up the data directory before installation.

Version 1.0.0 can close at startup when an existing SQLite database contains a migration created with LF line endings. Install [1.1.0](../releases/1.1.0.md) manually if the app cannot stay open long enough to offer the update. The installer keeps the application data directory and existing projects.

## Platform signatures
Updater signatures are required and are not an Apple Developer or Windows Authenticode signature.

Without Apple credentials, the macOS workflow creates ad-hoc signed apps for both architectures. Gatekeeper may require explicit approval on installation. To distribute notarized builds, add APPLE_CERTIFICATE (base64 P12), APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY, APPLE_ID, APPLE_PASSWORD (app-specific) and APPLE_TEAM_ID as Actions secrets. See [macOS signing](https://v2.tauri.app/distribute/sign/macos/).

Windows installers can be built without a commercial signing certificate; SmartScreen reputation is separate from updater verification. See [Windows signing](https://v2.tauri.app/distribute/sign/windows/).

## References
I used [GitButler’s release matrix](https://github.com/gitbutlerapp/gitbutler/blob/master/.github/workflows/publish.yaml) as a reference for separate platform artifacts, and [OneCAD CI](https://github.com/andrejvysny/OneCAD/blob/master/.github/workflows/ci.yml) for native-platform checks. [Tauri Action](https://github.com/tauri-apps/tauri-action) documents the standard updater metadata. Forma merges manifests in one final job to avoid concurrent writes to latest.json.
