-- Your SQL goes here
CREATE TABLE messages (
    uuid uuid PRIMARY KEY NOT NULL,
    channel_uuid uuid NOT NULL REFERENCES channels(uuid) ON DELETE CASCADE,
    user_uuid uuid NOT NULL REFERENCES users(uuid),
    message varchar(4000) NOT NULL
);
