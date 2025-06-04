-- Your SQL goes here
CREATE TABLE instances (
    instance_url VARCHAR(8000) PRIMARY KEY NOT NULL,
    public_key VARCHAR(500) UNIQUE NOT NULL
);

CREATE TABLE federated_users (
    uuid UUID PRIMARY KEY NOT NULL,
    instance_url VARCHAR(8000) NOT NULL REFERENCES instances(instance_url)
);
