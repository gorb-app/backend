-- Your SQL goes here
CREATE TABLE channels (
    uuid uuid PRIMARY KEY NOT NULL,
    guild_uuid uuid NOT NULL REFERENCES guilds(uuid) ON DELETE CASCADE,
    name varchar(32) NOT NULL,
    description varchar(500) NOT NULL
);
CREATE TABLE channel_permissions (
    channel_uuid uuid NOT NULL REFERENCES channels(uuid) ON DELETE CASCADE,
    role_uuid uuid NOT NULL REFERENCES roles(uuid) ON DELETE CASCADE,
    permissions int8 NOT NULL DEFAULT 0,
    PRIMARY KEY (channel_uuid, role_uuid)
);
