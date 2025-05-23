-- Your SQL goes here
CREATE TABLE refresh_tokens (
    token varchar(64) PRIMARY KEY UNIQUE NOT NULL,
    uuid uuid NOT NULL REFERENCES users(uuid),
    created_at int8 NOT NULL,
    device_name varchar(16) NOT NULL
);
CREATE TABLE access_tokens (
    token varchar(32) PRIMARY KEY UNIQUE NOT NULL,
    refresh_token varchar(64) UNIQUE NOT NULL REFERENCES refresh_tokens(token) ON UPDATE CASCADE ON DELETE CASCADE,
    uuid uuid NOT NULL REFERENCES users(uuid),
    created_at int8 NOT NULL
);
