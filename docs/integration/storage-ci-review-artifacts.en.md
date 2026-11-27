# Storage CI and review artifacts

[Русский](storage-ci-review-artifacts.md) · [Preview](storage-preview-source.en.md)

The branch-owned Storage comparison and Storage native consumers workflows exclude
only `docs/integration/**` from push triggers on `codex/storage-week`. Commits containing
only contracts/unapplied patch artifacts do not start another CAD build or cancel
the active comparison. In mixed commits, any path outside that directory triggers
the complete existing workflow.

`docs/fixtures`, frontend/Rust/C++ sources, Cargo, build scripts and CI configuration
remain covered. Manual workflow_dispatch remains. Jobs, commands, test filters,
PASS guards, permissions and concurrency are unchanged. The owner's shared Quality
workflow is unchanged; documentation/i18n checks remain required.

When introducing the filter, both YAML files were parsed and compared structurally
against `4b96158` after removing the sole new
paths-ignore field. Cases covered docs-only, docs+Rust, frontend, fixture, C++,
Cargo.lock and workflow changes. A search for direct runtime references to
docs/integration in apps/cad-core/scripts/workflows found none; the shared
documentation validator remains. This trigger filter does not validate
future source changes: every runtime commit requires its own checks.
