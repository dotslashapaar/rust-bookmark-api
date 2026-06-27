-- Add migration script here

CREATE TABLE users (
    id UUID PRIMARY KEY,
    password_hash TEXT NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

