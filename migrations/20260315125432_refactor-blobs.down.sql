-- Add down migration script here
DROP TABLE IF EXISTS blob_refs;
ALTER TABLE blobs ADD COLUMN ref_count INTEGER NOT NULL DEFAULT 0;
-- causes some data loss so hopefully no one runs this migration
