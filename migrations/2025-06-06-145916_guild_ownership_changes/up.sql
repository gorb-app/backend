-- Your SQL goes here
ALTER TABLE guild_members
ADD COLUMN is_owner BOOLEAN NOT NULL DEFAULT false;

UPDATE guild_members gm
SET is_owner = true
FROM guilds g
WHERE gm.guild_uuid = g.uuid AND gm.user_uuid = g.owner_uuid;

CREATE UNIQUE INDEX one_owner_per_guild ON guild_members (guild_uuid)
WHERE is_owner;

ALTER TABLE guilds
DROP COLUMN owner_uuid;
