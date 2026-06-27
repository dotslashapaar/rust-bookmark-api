-- Add migration script here
DELETE FROM bookmarks;
ALTER TABLE bookmarks ADD COLUMN owner_id UUID NOT NULL REFERENCES users(id);
