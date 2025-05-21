-- Your SQL goes here
CREATE TABLE guilds (
    uuid uuid PRIMARY KEY NOT NULL,
    owner_uuid uuid NOT NULL REFERENCES users(uuid),
    name VARCHAR(100) NOT NULL,
    description VARCHAR(300)
);
CREATE TABLE guild_members (
    uuid uuid PRIMARY KEY NOT NULL,
    guild_uuid uuid NOT NULL REFERENCES guilds(uuid) ON DELETE CASCADE,
    user_uuid uuid NOT NULL REFERENCES users(uuid),
    nickname VARCHAR(100) DEFAULT NULL
);
