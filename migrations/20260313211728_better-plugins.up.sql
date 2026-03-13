-- Add up migration script here
DROP TABLE IF EXISTS plugins;
ALTER TABLE groups ADD COLUMN quota_bytes INTEGER NOT NULL DEFAULT 1073741824; -- 1GB
ALTER TABLE groups ADD COLUMN used_bytes INTEGER NOT NULL DEFAULT 0;

CREATE TABLE blobs (
    id TEXT PRIMARY KEY,  -- uuid V7
    sha256 TEXT NOT NULL UNIQUE,
    size_bytes INTEGER NOT NULL,
    ref_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE plugins (
    id TEXT PRIMARY KEY,  -- uuid V7
    plugin_group_id TEXT NOT NULL,
    plugin_artifact_id TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'local',
    external_provider TEXT,
    external_id TEXT,

    UNIQUE (external_provider, external_id)
);

CREATE TABLE plugin_versions (
    id TEXT PRIMARY KEY,  -- uuid V7
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    blob_id TEXT REFERENCES blobs(id) ON DELETE RESTRICT,

    UNIQUE (plugin_id, version)
);

CREATE TABLE group_plugins (
    group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE RESTRICT,
    is_owner INTEGER NOT NULL DEFAULT 0 CHECK (is_owner IN (0, 1)),
    visibility TEXT NOT NULL DEFAULT 'private',
    attached_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),

    PRIMARY KEY (group_id, plugin_id)
);

CREATE UNIQUE INDEX idx_group_plugins_single_owner ON group_plugins (plugin_id) WHERE is_owner = 1;

CREATE INDEX idx_plugins_external ON plugins (external_provider, external_id) WHERE external_provider IS NOT NULL;
CREATE INDEX idx_plugin_versions_plugin ON plugin_versions (plugin_id);
CREATE INDEX idx_plugin_versions_blob ON plugin_versions (blob_id) WHERE blob_id IS NOT NULL;
CREATE INDEX idx_group_plugins_plugin ON group_plugins (plugin_id);
CREATE INDEX idx_blobs_sha256 ON blobs (sha256);
