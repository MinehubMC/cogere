-- Add up migration script here
CREATE TABLE users (
    id TEXT PRIMARY KEY, -- uuid V7
    username TEXT NOT NULL,
    email TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role INT NOT NULL DEFAULT 0
);

CREATE TABLE groups (
    id TEXT PRIMARY KEY, -- uuid V7
    name TEXT NOT NULL
);

CREATE TABLE machine_keys (
    id TEXT PRIMARY KEY, -- uuid V7
    description TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    group_id TEXT NOT NULL REFERENCES groups(id)
);
