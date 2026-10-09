CREATE TABLE project_storage_commits (
    project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    base_manifest_sha256 TEXT,
    target_manifest TEXT NOT NULL,
    project_sha256 TEXT NOT NULL
);
