-- Add up migration script here
CREATE TABLE plugins (
    id TEXT PRIMARY KEY, -- uuid V7
    artifact_id TEXT NOT NULL,
    group_id TEXT NOT NULL,
    version TEXT NOT NULL
);
