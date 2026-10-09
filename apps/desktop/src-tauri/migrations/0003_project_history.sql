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
