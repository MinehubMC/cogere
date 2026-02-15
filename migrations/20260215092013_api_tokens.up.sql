-- Add up migration script here
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    hashed_key TEXT NOT NULL,
    description TEXT NOT NULL,
    role INT NOT NULL DEFAULT 0
);
