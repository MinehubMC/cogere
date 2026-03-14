-- Add up migration script here
CREATE TABLE assemblies (
    id TEXT NOT NULL PRIMARY KEY, -- uuid v7
    group_id TEXT NOT NULL REFERENCES groups(id),
    status TEXT NOT NULL DEFAULT 'pending',
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT,
    expires_at TEXT,
    error TEXT,
    blob_id TEXT REFERENCES blobs(id)
);

CREATE TABLE assembly_artifacts (
    assembly_id TEXT NOT NULL REFERENCES assemblies(id) ON DELETE CASCADE,
    group_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    version TEXT NOT NULL,
    PRIMARY KEY (assembly_id, group_id, artifact_id, version)
);
