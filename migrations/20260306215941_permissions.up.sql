-- Add up migration script here
ALTER TABLE users DROP COLUMN role;
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user';

CREATE TABLE group_members (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    group_id TEXT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    group_role TEXT NOT NULL DEFAULT 'viewer',
    PRIMARY KEY (user_id, group_id)
);

CREATE TABLE machine_key_permissions (
    key_id TEXT NOT NULL REFERENCES machine_keys(id) ON DELETE CASCADE,
    resource_type TEXT NOT NULL,
    resource_id TEXT, -- allows per-resource overrides
    action TEXT NOT NULL,
    PRIMARY KEY (key_id, resource_type, resource_id, action)
)
