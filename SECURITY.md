# Security
[README](README.md) · [Русский](SECURITY.ru.md)

Do not post credentials or private project files in public issues. Use [GitHub private vulnerability reporting](https://github.com/lovlyDev/FormaCAD/security/advisories/new) if enabled; otherwise open an issue requesting a private contact without disclosing exploit details.

Forma restricts filesystem paths and geometry execution. CLI agents are external programs and are not a general-purpose OS sandbox. Permissions are checked in Rust. Updates require a trusted signature; updater keys are separate from platform certificates.

Update checks contact GitHub; AI requests use the chosen provider. There is no Forma telemetry backend. Logs and raw diagnostics may contain paths or prompts: inspect them before sharing.

Private keys, environment files and runtime data are excluded by [.gitignore](.gitignore). See [release signing](docs/en/releases.md) and [backup procedures](docs/en/data.md).
