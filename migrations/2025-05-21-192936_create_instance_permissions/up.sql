-- Your SQL goes here
CREATE TABLE instance_permissions (
    uuid uuid PRIMARY KEY NOT NULL REFERENCES users(uuid),
    administrator boolean NOT NULL DEFAULT FALSE
);
