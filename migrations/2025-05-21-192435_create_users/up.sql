-- Your SQL goes here
CREATE TABLE users (
    uuid uuid PRIMARY KEY NOT NULL,
    username varchar(32) NOT NULL,
    display_name varchar(64) DEFAULT NULL,
    password varchar(512) NOT NULL,
    email varchar(100) NOT NULL,
    email_verified boolean NOT NULL DEFAULT FALSE,
    is_deleted boolean NOT NULL DEFAULT FALSE,
    deleted_at int8 DEFAULT NULL,
    CONSTRAINT unique_username_active UNIQUE NULLS NOT DISTINCT (username, is_deleted),
    CONSTRAINT unique_email_active UNIQUE NULLS NOT DISTINCT (email, is_deleted)
);

CREATE UNIQUE INDEX idx_unique_username_active 
ON users(username) 
WHERE is_deleted = FALSE;
CREATE UNIQUE INDEX idx_unique_email_active 
ON users(email) 
WHERE is_deleted = FALSE;
