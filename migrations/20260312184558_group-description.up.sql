-- Add up migration script here
ALTER TABLE groups ADD COLUMN description NOT NULL DEFAULT 'unset';
