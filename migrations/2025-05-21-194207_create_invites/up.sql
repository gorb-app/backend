-- Your SQL goes here
CREATE TABLE invites (
    id varchar(32) PRIMARY KEY NOT NULL,
    guild_uuid uuid NOT NULL REFERENCES guilds(uuid) ON DELETE CASCADE,
    user_uuid uuid NOT NULL REFERENCES users(uuid)
);
