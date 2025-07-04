-- Your SQL goes here
ALTER TABLE messages ADD COLUMN reply_to UUID REFERENCES messages(uuid) DEFAULT NULL;
