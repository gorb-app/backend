-- Your SQL goes here
ALTER TABLE channels ADD COLUMN is_above UUID UNIQUE REFERENCES channels(uuid) DEFAULT NULL;
