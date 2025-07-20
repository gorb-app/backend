-- Your SQL goes here
ALTER TABLE guild_members ADD UNIQUE (user_uuid, guild_uuid)