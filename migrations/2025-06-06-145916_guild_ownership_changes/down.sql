-- This file should undo anything in `up.sql`
ALTER TABLE guilds
ADD COLUMN owner_uuid UUID REFERENCES users(uuid);

UPDATE guilds g
SET owner_uuid = gm.user_uuid
FROM guild_members gm
WHERE gm.guild_uuid = g.uuid AND gm.is_owner = TRUE;

ALTER TABLE guilds
ALTER COLUMN owner_uuid SET NOT NULL;

ALTER TABLE guild_members
DROP COLUMN is_owner;
