-- Add up migration script here
CREATE TABLE instance_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
