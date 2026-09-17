# Releases and updates
[Documentation](index.md) · [Русский](../ru/releases.md)

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

## Publish a version
~~~sh
npm run version:set -- 1.0.0
~~~
This synchronizes npm, the lockfile, Cargo and Tauri. Add release notes under docs/releases/VERSION.md. For subsequent releases choose a higher version. Commit the source, push it, then tag the matching commit:
~~~sh
git tag v1.0.0
git push origin main
git push origin v1.0.0
~~~
Use the actual default branch if it differs.

[Release](../../.github/workflows/release.yml) runs [Quality](../../.github/workflows/ci.yml), then builds Windows x64, Linux x64, macOS ARM64 and Intel. Packages are signed for the updater, collected without filename collisions, and combined into latest.json and SHA256SUMS. The release stays a draft until every asset is uploaded. Existing releases are never overwritten; resolve an incomplete draft before rerunning its tag.

Ordinary commits, branch pushes and pull requests do not start checks, installer builds or publication. The only automatic trigger is pushing a new vX.Y.Z tag. Its version must match the application, checked before the matrix starts. Quality is called only by Release. Updating or deleting an existing tag does not publish a release. No manual GitHub Release creation is needed. Previously started workflows can be cancelled separately in Actions.

## Application behavior
Forma checks at startup and every six hours. A newer release opens an Install / Later dialog when no operation or editor is active. Later defers that prompt; the update button remains available. Installation verifies the artifact signature, saves preferences and creates a SQLite backup before invoking the platform installer. Windows restarts through NSIS; macOS and AppImage relaunch after replacement.

Linux DEB/RPM installations use new packages rather than in-place self-update. A first release, offline connection or GitHub failure does not block starting the app. The local preview version line predates this GitHub numbering; install 1.0.0 manually once if moving from a higher-numbered local preview. Data paths stay unchanged.

## Platform signatures
Updater signatures are required and are not an Apple Developer or Windows Authenticode signature.

For notarized macOS releases add APPLE_CERTIFICATE (base64 P12), APPLE_CERTIFICATE_PASSWORD, APPLE_SIGNING_IDENTITY, APPLE_ID, APPLE_PASSWORD (app-specific) and APPLE_TEAM_ID as Actions secrets. See [macOS signing](https://v2.tauri.app/distribute/sign/macos/). Without these credentials macOS builds are not notarized and Gatekeeper may require explicit approval.

Windows installers can be built without a commercial signing certificate; SmartScreen reputation is separate from updater verification. See [Windows signing](https://v2.tauri.app/distribute/sign/windows/).

## References
I used [GitButler’s release matrix](https://github.com/gitbutlerapp/gitbutler/blob/master/.github/workflows/publish.yaml) as a reference for separate platform artifacts, and [OneCAD CI](https://github.com/andrejvysny/OneCAD/blob/master/.github/workflows/ci.yml) for native-platform checks. [Tauri Action](https://github.com/tauri-apps/tauri-action) documents the standard updater metadata. Forma merges manifests in one final job to avoid concurrent writes to latest.json.
