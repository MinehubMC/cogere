-- Add down migration script here
DROP INDEX IF EXISTS idx_blobs_sha256;
DROP INDEX IF EXISTS idx_group_plugins_plugin;
DROP INDEX IF EXISTS idx_plugin_versions_blob;
DROP INDEX IF EXISTS idx_plugin_versions_plugin;
DROP INDEX IF EXISTS idx_plugins_external;
DROP INDEX IF EXISTS idx_group_plugins_single_owner;
DROP TABLE IF EXISTS group_plugins;
DROP TABLE IF EXISTS plugin_versions;
DROP TABLE IF EXISTS plugins;
DROP TABLE IF EXISTS blobs;
ALTER TABLE groups DROP COLUMN used_bytes;
ALTER TABLE groups DROP COLUMN quota_bytes;
CREATE TABLE plugins (
    id TEXT PRIMARY KEY, -- uuid V7
    artifact_id TEXT NOT NULL,
    group_id TEXT NOT NULL,
    version TEXT NOT NULL
);
