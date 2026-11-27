# Checking the result of a folder write

[Русский](storage-folder-commit-query.md) · [Host API](../en/storage-folder-host-api.md)

Proposed against `d830ec6def86a0eac5ef11d1234c8da706749a98`.
The [exact patch](storage-folder-commit-query.patch) has SHA256
`a7a7e9f25065bbf46b5f6823b146f88711a77d47d70a4c98e7ca8d1ed24203d6`.
The source changes are in the isolated review copy.

After an uncertain write, the application can check its attempted commit ID
through the original opening. `observe_folder_commit` takes
`projectId`, `sourceId`, and `commitId`. The host takes the window from
Tauri and reads the saved commit chain. Its reply echoes the three IDs
with a status of `current`, `historical`, or `absent`. The client checks
the IDs against the original request. An absent result is an observation
at the time of the read. The API has no replay operation.

The patch registers the read command in the shared `src-tauri/src/lib.rs`,
adds `window_sources/ipc/commit_query.rs` and its declaration in
`window_sources/ipc.rs`, and adds the client and tests in
`project-sources/{commitObservation.ts,commitObservation.test.ts}`.
The user action and checking the active opening before updating the scene
remain part of the next integration.

The patch applies cleanly. Rust formatting, TypeScript, ESLint on the new
modules, and nine transport tests passed. The Rust handler still needs
compilation after agreement.
