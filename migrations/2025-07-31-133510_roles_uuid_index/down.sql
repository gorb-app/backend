-- This file should undo anything in `up.sql`
DROP INDEX roles_guuid_uuid;
ALTER TABLE roles DROP CONSTRAINT roles_pkey;
CREATE UNIQUE INDEX roles_pkey ON roles (uuid, guild_uuid);
ALTER TABLE roles ADD PRIMARY KEY USING INDEX roles_pkey;
