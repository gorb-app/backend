-- Your SQL goes here
CREATE TABLE password_reset_tokens (
    token VARCHAR(64) NOT NULL,
    user_uuid uuid UNIQUE NOT NULL REFERENCES users(uuid),
    created_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (token, user_uuid)
);
