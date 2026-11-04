# Installing with winget

[Documentation](index.md) · [Русский](../ru/package-managers.md) · [Releases](releases.md)

On Windows, FormaCAD can be installed from the Microsoft winget catalog. winget downloads the same installer from GitHub Releases and checks its SHA-256.

## User steps

1. Open Terminal or PowerShell.
2. Install: `winget install lovlyDev.FormaCAD`
3. Update: the app updates itself; manually, `winget upgrade lovlyDev.FormaCAD`.
4. Remove: `winget uninstall lovlyDev.FormaCAD` or Settings → Apps.

winget installs FormaCAD for the current user, no administrator rights needed. The first version, 1.6.1, is [submitted to the catalog](https://github.com/microsoft/winget-pkgs/pull/470914) and passed the automated check; the command works once a Microsoft moderator accepts the package.

## Technical route

- The package lives in `packaging/winget/manifests/l/lovlyDev/FormaCAD/<version>/`: version, installer (per-user NSIS, `ProductCode` `Forma`) and English and Russian descriptions. Check with `winget validate --manifest <folder>`.
- New versions are submitted by the `package-managers` job of the Release workflow after a stable release is published: `wingetcreate update lovlyDev.FormaCAD --version <X.Y.Z> --urls <EXE> --submit`. It opens a PR in `microsoft/winget-pkgs` as the token owner.
- It needs the `WINGET_TOKEN` repository secret: a classic GitHub token with `public_repo`. Without the secret the step is skipped with a warning and the release does not fail.

### Creating and adding `WINGET_TOKEN`

1. GitHub → avatar at the top right → **Settings** → **Developer settings** at the bottom left → **Personal access tokens** → **Tokens (classic)** → **Generate new token** → **Generate new token (classic)**.
2. **Note**: `FormaCAD winget`. **Expiration**: `No expiration` or a year (then renew it).
3. Tick only **public_repo** (inside the `repo` section). Leave everything else unticked.
4. Click **Generate token** and copy the `ghp_…` token right away: GitHub shows it once.
5. Repository `lovlyDev/FormaCAD` → **Settings** → **Secrets and variables** → **Actions** → **New repository secret**.
6. **Name**: `WINGET_TOKEN`, **Secret**: paste the token → **Add secret**.

Never paste the token into a chat or commit it to the repository.

## Limitations

- Betas (`-beta.N`) never go to winget.
- Microsoft moderators review every new version too, so it reaches winget after a few hours to a couple of days.
- Other package managers (Homebrew, Flathub) are not supported: on macOS and Linux use the DMG, AppImage, DEB or RPM from the release.
