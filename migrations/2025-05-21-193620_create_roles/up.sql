-- Your SQL goes here
CREATE TABLE roles (
    uuid uuid UNIQUE NOT NULL,
    guild_uuid uuid NOT NULL REFERENCES guilds(uuid) ON DELETE CASCADE,
    name VARCHAR(50) NOT NULL,
    color int NOT NULL DEFAULT 16777215,
    position int NOT NULL,
    permissions int8 NOT NULL DEFAULT 0,
    PRIMARY KEY (uuid, guild_uuid)
);
CREATE TABLE role_members (
    role_uuid uuid NOT NULL REFERENCES roles(uuid) ON DELETE CASCADE,
    member_uuid uuid NOT NULL REFERENCES guild_members(uuid) ON DELETE CASCADE,
    PRIMARY KEY (role_uuid, member_uuid)
);
