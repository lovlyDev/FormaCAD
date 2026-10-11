# Releases and updates
[Documentation](index.md) · [Русский](../ru/releases.md)

A single assigned release owner publishes after verifying the exact source commit and artifacts. **1.3.0 was published on 11 October 2026** after verification under the user's explicit instruction: [release](https://github.com/lovlyDev/FormaCAD/releases/tag/v1.3.0). The Windows EXE and signature are verified and its source commit passed GitHub Quality. Installation on the user's computer is not authorized. **1.2.6** remains the previously verified and published Windows release. Full 2.0 is unfinished; see [status](status.md).

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

## Packaged OCCT licenses

The native Windows and Unix configurations give each file a complete destination: `licenses/occt/LICENSE_LGPL_21.txt` and `licenses/occt/OCCT_LGPL_EXCEPTION.txt`. Mapping both individual files to the same path caused a filename collision in the NSIS archive; the 1.3.0 sources correct it. Licenses belong to the package and do not affect user databases. Extract the Windows EXE without running the installer; require both uniquely named licenses and compare their SHA-256 hashes with `resources/occt`. Compare the extracted app, worker and 25 DLLs with build outputs and check geometry using the extracted worker. Updating the Unix configuration does not establish macOS/Linux package validation.

Tauri changes exactly `__TAURI_BUNDLE_TYPE_VAR_UNK` to `__TAURI_BUNDLE_TYPE_VAR_NSS` in the packaged app, then restores the build output. Compare the extracted app's full SHA-256 with the expected result of that sole replacement; reject every other difference. [Primary Tauri source](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-bundler/src/bundle.rs).

## Two-PC handoff and source review

Follow the [collaboration guide](collaboration.md) and [AGENTS.md](../../AGENTS.md). By default, one active owner implements a task on one PC; the other person may read it or receive an explicit handoff; project head lovlyDev handles acceptance. Each person uses a separate clone. Do not switch an occupied branch, reset someone else's changes or transfer a personal database through Git.

Record the owner, task branch, base and final SHA, allowed files, actual checks, limits and next owner in the Issue and PR. A second-PC handoff transfers a reviewable commit, not assumptions from another chat. Uncommitted files remain on their original PC. The recipient confirms ownership before editing; the previous owner stops writing to the transferred area. The assigned integrator owns shared versions, lockfiles, migration numbers and release metadata.

Project head lovlyDev reviews the PR against the exact SHA and decides whether to merge; a second reviewer is optional. Agent summaries support review but do not replace it. Only the integrator merges changes and records the final release SHA. If source changes after verification, repeat affected checks; the artifact must correspond to the final commit. Direct pushes to the default branch are not a substitute for a PR and review.

## Prepare a new release

The owner selects a version not used by an existing release and synchronizes npm workspaces/lockfile, Cargo and Tauri with `npm run version:set -- X.Y.Z`, replacing `X.Y.Z` with the actual version. That command does not itself authorize publication. Prepare bilingual feature documentation and `docs/releases/X.Y.Z.md` with actual results and limitations.

Run the checks in the [development guide](development.md), including `check:version`, `check:i18n`, `check:migrations`, `check:docs`, frontend lint/types/unit/build, required browser scenarios in RU/EN and both themes, native tests, strict Clippy, the no-default-features check and fmt. Preserve the original result when retrying a failure; a successful narrow retry does not establish a successful full run. One assigned executor runs heavy native builds sequentially on each PC.

Build a new Windows EXE in its own versioned directory. Check its size and SHA-256, FileVersion/ProductVersion, updater signature against the existing key and metadata against the actual package. Verify the fresh worker extracted from **this** installer: its SHA, exactly one worker and the expected 25 OCCT DLLs, capabilities, positive analytic scenarios and safe rejections. A debug worker or old installer does not verify the new package. Save a sanitized check summary and EXE link; never overwrite old releases. Package verification does not establish installation or installed-WebView acceptance.

## Choose one publication route before pushing a tag

Human authorization for the current publication and the assigned owner must be explicit; earlier permission for 1.2.6 is not standing permission for later releases. Before pushing a new tag, check actual Actions state and whether a release or draft already exists for that version. Manually creating a release does not disable the tag-triggered workflow. Do not run manual Windows upload and an automatic publisher for the same version concurrently.

For the automatic route, the owner creates a new `vX.Y.Z` tag on the final verified SHA after the PR and review, then pushes only the agreed tag. Pushing the tag starts publication. Wait for actual success of Quality, all four platform builds and the final job; verify the published files, checksums and metadata. A tag or a started workflow does not establish release readiness.

[Release](../../.github/workflows/release.yml) runs [Quality](../../.github/workflows/ci.yml), then builds native CAD packages for Windows x64, Linux x64, macOS ARM64 and Intel. Windows uses [its OCCT build](../../scripts/build-occt-windows.ps1); macOS and Linux use [a pinned static OCCT build](../../scripts/build-native-unix.sh). Each worker must pass a geometry and checksum [smoke check](../../scripts/check-native-unix.py) before packaging. Packages are signed for the updater, collected without filename collisions, and combined into latest.json and SHA256SUMS. The release stays a draft until every asset is uploaded. Existing releases are never overwritten; resolve an incomplete draft before rerunning its tag.

Quality checks run automatically for pull requests targeting `main` and pushes to `main`; they do not publish packages. Quality also supports manual Actions runs and calls from the release workflow. Installer packaging and automatic publication remain tag-triggered: only a newly created `vX.Y.Z` tag starts the Release workflow, whose version must match the application. Manual Quality runs can include the native matrix; `nativeCadMatrix` selects `{ "os", "target" }` entries. Updating or deleting an existing tag does not publish a release. Separately authorized manual publication of a verified Windows package is described below.

## Separately authorized Windows route

For a Windows-only release, the owner uploads only verified local EXE, updater signature, SHA256SUMS and Windows metadata matching the exact release SHA. Before pushing a new tag, agree how a second publisher will be prevented: if Release has already started for that version, inspect its actual state and stop the competing publication within the granted authorization. Do not claim manual `create release` avoids the workflow. If a release already exists, do not replace its assets or reuse its tag.

Historical **1.2.6** was published through this separate route: its EXE and updater signature were verified, the package contained one worker and 25 OCCT DLLs, and the packaged worker passed 64 analytic cases and 19 rejections. See [artifact evidence](status.md). **1.3.0** requires its own completed checks and new artifact; it cannot borrow the 1.2.6 evidence. A Windows-only release does not establish macOS/Linux readiness or completion of the four-platform matrix.

The Release workflow above remains the route for a future automated matrix triggered by a new tag. It builds platforms, signs updater artifacts and merges metadata. Manually uploading the current Windows package does not prove that matrix passed and does not install anything on the user computer.

## Application behavior
Forma checks at startup and every six hours. A newer release opens an Install / Later dialog when no operation or editor is active. Later defers that prompt; the update button remains available. Installation verifies the artifact signature, saves preferences and creates a SQLite backup before invoking the platform installer. Windows restarts through NSIS; macOS and AppImage relaunch after replacement.

Linux DEB/RPM installations use new packages rather than in-place self-update. A missing release, offline connection or GitHub failure does not block startup. For a manual upgrade, use the installer of the specific verified published release. The application identity and data paths remain unchanged; back up the data directory before installation. Publishing a release does not authorize the agent to install it on the user's computer.

Version 1.0.0 can close at startup when an existing SQLite database contains a migration created with LF line endings. Install [1.1.0](../releases/1.1.0.md) manually if the app cannot stay open long enough to offer the update. The installer keeps the application data directory and existing projects.

## Platform signatures
Updater signatures are required and are not an Apple Developer or Windows Authenticode signature.

Without Apple credentials, the macOS workflow creates ad-hoc signed apps for both architectures. Gatekeeper may require explicit approval on installation. To distribute notarized builds, add APPLE_CERTIFICATE (base64 P12), APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY, APPLE_ID, APPLE_PASSWORD (app-specific) and APPLE_TEAM_ID as Actions secrets. See [macOS signing](https://v2.tauri.app/distribute/sign/macos/).

Windows installers can be built without a commercial signing certificate; SmartScreen reputation is separate from updater verification. See [Windows signing](https://v2.tauri.app/distribute/sign/windows/).

## References
I used [GitButler’s release matrix](https://github.com/gitbutlerapp/gitbutler/blob/master/.github/workflows/publish.yaml) as a reference for separate platform artifacts, and [OneCAD CI](https://github.com/andrejvysny/OneCAD/blob/master/.github/workflows/ci.yml) for native-platform checks. [Tauri Action](https://github.com/tauri-apps/tauri-action) documents the standard updater metadata. Forma merges manifests in one final job to avoid concurrent writes to latest.json.
