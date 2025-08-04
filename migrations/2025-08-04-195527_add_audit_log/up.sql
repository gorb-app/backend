-- Your SQL goes here
CREATE TABLE audit_logs (
	uuid UUID PRIMARY KEY NOT NULL,
	guild_uuid UUID NOT NULL,
	action_id INT2 NOT NULL,
	by_uuid UUID NOT NULL REFERENCES guild_members(uuid),
	channel_uuid UUID REFERENCES channels(uuid) DEFAULT NULL,
	user_uuid UUID REFERENCES users(uuid) DEFAULT NULL,
	message_uuid UUID REFERENCES messages(uuid) DEFAULT NULL,
	role_uuid UUID REFERENCES roles(uuid) DEFAULT NULL,
	audit_message VARCHAR(200) DEFAULT NULL,
	changed_from VARCHAR(200) DEFAULT NULL,
	changed_to VARCHAR(200) DEFAULT NULL
);
