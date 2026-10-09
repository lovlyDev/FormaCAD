# Maintaining documentation

[Documentation](index.md) · [Русский](../ru/documentation.md) · [Development](development.md)

**Write documentation immediately after every implemented feature, behavior fix, or user-flow change.** This is a permanent rule in [AGENTS.md](../../AGENTS.md) and the local handoff `.local/CODEX_HANDOFF_2_0.md`. A feature is incomplete until its behavior and limits are described in Russian and English. There is no file-count limit: split large topics by responsibility and link them from the [index](index.md) and [README](../../README.md).

## What to cover

For each material change, update its topical guide under `docs/en/` and the matching `docs/ru/` guide. Describe purpose and user steps, inputs and output, important failure modes, the UI/Rust/CAD-worker boundary, persisted data and recovery, limitations, and platform differences. For security or data-transfer changes, update [security](../../SECURITY.md) and [data](data.md). For build or publication changes, update [development](development.md) and [releases](releases.md). Add relative links to source files so readers can verify the explanation.

After UI changes, add new strings to both [i18n catalogs](../../apps/desktop/src/i18n/) and check Russian, English, light, and dark modes. A translated button does not document behavior, and a guide does not replace UI localization. For a new version, add a bilingual note in `docs/releases/VERSION.md` and link it from the [change index](changelog.md); never present a local note as a published release.

## Before handoff

Walk through the user flow and a failure case, then compare documentation claims with code and checks. Run `npm run check:docs` to validate local Markdown links and the Russian counterpart of every `docs/en/` file. For app changes, also run checks required by [AGENTS.md](../../AGENTS.md), including i18n, migration checks when changing schema, a build, and a new Windows EXE. Run `git diff --check` after documentation edits. If a platform or package was not verified, state that explicitly in [status](status.md) rather than treating an assumption as a result.

The local `.local/CODEX_HANDOFF_2_0.md` is ignored by Git and helps transfer context between chats. The tracked [AGENTS.md](../../AGENTS.md) is the durable rule for all contributors; topical guides and their indexes belong in the repository.
