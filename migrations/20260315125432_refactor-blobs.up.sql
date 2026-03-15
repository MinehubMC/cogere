-- Add up migration script here
CREATE TABLE blob_refs (
    blob_id TEXT NOT NULL REFERENCES blobs(id) ON DELETE RESTRICT,
    group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE RESTRICT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    PRIMARY KEY (blob_id, entity_id)
);

ALTER TABLE blobs DROP COLUMN ref_count;
