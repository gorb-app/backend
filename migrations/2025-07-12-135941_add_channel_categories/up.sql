-- Your SQL goes here
CREATE TABLE categories (
    uuid UUID PRIMARY KEY NOT NULL,
    guild_uuid UUID NOT NULL REFERENCES guilds(uuid),
    name VARCHAR(32) NOT NULL,
    description VARCHAR(500) DEFAULT NULL,
    is_above UUID UNIQUE REFERENCES categories(uuid) DEFAULT NULL
);

ALTER TABLE channels ADD COLUMN in_category UUID REFERENCES categories(uuid) DEFAULT NULL;
