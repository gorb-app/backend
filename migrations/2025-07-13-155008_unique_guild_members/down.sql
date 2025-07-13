-- This file should undo anything in `up.sql`
ALTER TABLE guild_members DROP CONSTRAINT guild_members_user_uuid_guild_uuid_key;
