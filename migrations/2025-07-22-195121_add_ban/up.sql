-- Your SQL goes here
CREATE TABLE guild_bans (
	guild_uuid uuid NOT NULL REFERENCES guilds(uuid) ON DELETE CASCADE,
	user_uuid uuid NOT NULL REFERENCES users(uuid),
	reason VARCHAR(200) DEFAULT NULL,
	ban_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
	PRIMARY KEY (user_uuid, guild_uuid)
);
