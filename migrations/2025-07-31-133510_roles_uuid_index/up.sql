-- Your SQL goes here
ALTER TABLE roles DROP CONSTRAINT roles_pkey;
CREATE UNIQUE INDEX roles_pkey ON roles (uuid);
ALTER TABLE roles ADD PRIMARY KEY USING INDEX roles_pkey;
CREATE UNIQUE INDEX roles_guuid_uuid ON roles (uuid, guild_uuid);