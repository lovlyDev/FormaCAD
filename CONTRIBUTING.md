# Contributing
[README](README.md) · [Русский](CONTRIBUTING.ru.md)

I welcome focused issues and pull requests. Describe the behavior, reproduction steps and expected result. Include OS, application version and sanitized diagnostics.

Follow the [development guide](docs/en/development.md), [architecture](docs/en/architecture.md) and [project rules](AGENTS.md). Keep UI text bilingual and support both themes. Preserve application identity, migrations and data paths. Test stored-state compatibility whenever changing persistence.

Run checks before a PR. Include a short explanation and relevant test results. Do not commit secrets, local projects, screenshots used only as evidence, generated installers or caches. GitHub releases are made through the [release workflow](.github/workflows/release.yml).
