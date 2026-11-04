# License and code signing

[Documentation](index.md) · [Русский](../ru/code-signing.md) · [Releases](releases.md)

## License

FormaCAD is distributed under the **GNU GPL version 3 or later** (`GPL-3.0-or-later`); the full text is in [LICENSE](../../LICENSE). You may study, change and redistribute the code; modified versions must also be released under the GPL with source code.

Third-party components keep their own licenses: OpenCascade (LGPL-2.1 with the OCCT exception; licenses ship next to the package), `vendor/tauri-plugin-updater` (MIT/Apache-2.0), npm and Cargo dependencies (MIT, Apache-2.0, BSD, ISC and compatible). The project owner and license decision: lovlyDev.

## Code signing policy

Windows installers are planned to be signed with a free SignPath Foundation certificate through SignPath.io. Until signing is set up, Windows may show "Unknown publisher".

Once set up:

- Only FormaCAD's own program is signed, built from this repository's sources in GitHub Actions (Release workflow). Third-party libraries (OpenCascade and others) are not signed with the project certificate.
- Every release is signed only after manual approval.
- Credit line: *Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org).*

### Roles

| Role | Who |
|---|---|
| Authors (trusted committers) | [lovlyDev](https://github.com/lovlyDev) |
| Reviewers (review changes from other contributors) | [lovlyDev](https://github.com/lovlyDev) |
| Approver of every release signature | [lovlyDev](https://github.com/lovlyDev) |

All team members use multi-factor authentication on GitHub and SignPath.

### Privacy

The program runs locally; projects, models and settings are not sent to FormaCAD servers. Network requests:

- update checks on GitHub (stable channel) or formacad.ru (beta channel, only after signing in);
- optional sign-in to a FormaCAD account and account rights refresh on formacad.ru;
- the plugins.formacad.ru catalog when it is opened;
- AI requests to the CLI provider the user selected, only when the user allowed the action.

More: [data](data.md), [security](../../SECURITY.md) and the privacy policy on [formacad.ru](https://formacad.ru). The program does not change system settings without warning and uninstalls normally: Windows Settings → Apps, or the uninstaller in the install folder.
