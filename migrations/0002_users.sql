CREATE TABLE IF NOT EXISTS users (
    id            BIGSERIAL   PRIMARY KEY,
    username      TEXT        NOT NULL UNIQUE,
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Notes now belong to a user. Notes created before this migration have no owner,
-- so no one can see them through the API.
ALTER TABLE notes ADD COLUMN IF NOT EXISTS user_id BIGINT REFERENCES users (id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS notes_user_id ON notes (user_id);
