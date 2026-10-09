# Reading a committed project snapshot

[Documentation](index.md) · [Русский](../ru/project-snapshots.md) · [Project access](project-access.md) · [History and packages](project-history.md)

Managed-project copying, `.cadpack` export and exact measurements read committed state through a shared pure snapshot adapter. Reading does not open editing access, publish a recovered folder generation or clear its recovery journal. No new controls or settings are required.

## User workflow

Use the existing duplicate-project, save-editable-copy or package-export actions. For `PROJECT_SNAPSHOT_CHANGED`, refresh the project and retry: committed state changed during the read. For `PROJECT_SNAPSHOT_MISSING`, refresh the project list. For `PROJECT_SNAPSHOT_RECOVERY_REQUIRED`, reopen the project so the normal open route can finish recovery, then retry copying or exporting. The pure reader never repairs the source implicitly.

Exact measurements and sections using their capture retain compatible `MEASUREMENT_STALE` and `MEASUREMENT_RECOVERY_REQUIRED` messages. Technical codes and diagnostic details remain separate from the localized user message; user filenames and contents are not translated.

## Route and storage boundary

The shared adapter captures the project description and logical history from one consistent SQL snapshot, then checks the corresponding immutable folder generation. Copy and export consume this pair instead of separate reads between which the history cursor could move. A changed state or pending recovery causes an explicit failure without implicit reconciliation. Consumers retain their attachment and history validation; a consistent SQL snapshot alone does not verify files.

Persisted attachments are opened through a bounded reader: the file must be regular and match its declared size before allocation, then at most that size plus one byte is read and its checksum is checked. This prevents a corrupted or growing attachment mirror from causing an unbounded read. The existing40MiB per-file limit remains. Copy also checks decoded inline legacy bytes against their declared size and any saved checksum. Package export prepares verified bytes within its existing150MiB archive budget and rechecks the snapshot before invoking the existing temporary-file archive publisher; an invalid source leaves an existing destination intact. These checks do not claim protection against every hostile filesystem race.

The source remains the same project: its description, history and owned files are not rewritten by the reader. Creating a copy saves a NEW project UUID and validated history in a separate project; this inserts the new project into the same application-wide SQLite database. The guarantee therefore does not mean that the entire shared database file remains byte-identical. Package export does not create a project copy in the index. Application settings, CLI credentials and permissions are not transferred by copying.

The backend remains application-managed storage (`LegacyManaged` in the transition plan). This change does not make an external folder authoritative, implement arbitrary-folder in-place opening or move all consumers to a new format. NAS support and distributed lock recovery are not claimed.

## Compatibility

Copying a current managed project retains validated history and revision IDs under a new project UUID. This differs from the future conversion of a genuine application 1.1 project: that workflow must create a new 2.0 project from the last verified STEP without promising recovery of editable history. The `LegacyManaged` backend label alone does not identify application version 1.1.

## Verification

The frontend regression in `apps/desktop/src/i18n/errorText.test.ts` checks all three new codes in Russian and English, localized messages and separate preservation of raw diagnostics. Native shared-snapshot, copy, export and measurement-compatibility checks run separately; release evidence is recorded after they finish. A translation test does not prove OS locking or crash recovery. This step does not establish overall 2.0 storage readiness.

Implementation: [project_repository/legacy](../../apps/desktop/src-tauri/src/project_repository/legacy/mod.rs), `capture_project` / `capture_with_history`.
