-- FCP1 schema v1: independent immutable migration lineage.
-- History table contract derives from app 0003_project_history.sql; never run app migrations here.
CREATE TABLE folder_format (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
    marker TEXT NOT NULL,
    schema_sha256 TEXT NOT NULL,
    schema_fingerprint TEXT NOT NULL
);
CREATE TABLE repository_state (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
    repository_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    head_commit_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK(sequence > 0)
);
CREATE TABLE folder_commits (
    commit_id TEXT PRIMARY KEY NOT NULL,
    parent_commit_id TEXT REFERENCES folder_commits(commit_id),
    sequence INTEGER NOT NULL UNIQUE CHECK(sequence > 0),
    revision_id TEXT,
    project_sha256 TEXT NOT NULL,
    cursor_sha256 TEXT NOT NULL,
    history_sha256 TEXT NOT NULL,
    seal_sha256 TEXT NOT NULL
);
CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    payload TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE project_history (
    project_id TEXT PRIMARY KEY NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    state_json TEXT NOT NULL
);
CREATE TABLE project_history_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('edit', 'undo', 'redo')),
    from_revision TEXT,
    to_revision TEXT,
    target_revision TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX project_history_events_project ON project_history_events(project_id, id);
