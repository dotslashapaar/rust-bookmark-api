-- Add migration script here

-- CREATE TABLE users (
--     id UUID PRIMARY KEY,
--     password_hash TEXT NOT NULL,
--     email VARCHAR(255) NOT NULL,
--     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
-- );

CREATE TABLE bookmarks (
    id UUID PRIMARY KEY,
    -- owner_id UUID NOT NULL REFERENCES users(id),
    title VARCHAR(255) NOT NULL,
    url TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
