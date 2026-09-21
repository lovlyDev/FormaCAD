CREATE TABLE projects (id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL, payload TEXT NOT NULL, updated_at TEXT NOT NULL);
CREATE TABLE permissions (id TEXT PRIMARY KEY NOT NULL, project_id TEXT NOT NULL, action TEXT NOT NULL, detail TEXT NOT NULL, decision TEXT NOT NULL DEFAULT 'pending', created_at TEXT NOT NULL);
CREATE TABLE agent_sessions (id TEXT PRIMARY KEY NOT NULL, project_id TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL, completed_at TEXT);
CREATE TABLE activities (id INTEGER PRIMARY KEY AUTOINCREMENT, project_id TEXT, kind TEXT NOT NULL, detail TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE TABLE settings (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL);
CREATE INDEX permission_project ON permissions(project_id, created_at);
