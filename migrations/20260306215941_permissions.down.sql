-- Add down migration script here
DROP TABLE IF EXISTS machine_key_permissions;
DROP TABLE IF EXISTS group_members;
ALTER TABLE users DROP COLUMN role;
ALTER TABLE users ADD COLUMN role INT NOT NULL DEFAULT 0;
