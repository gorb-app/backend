-- Your SQL goes here
ALTER TABLE roles DROP COLUMN position;
ALTER TABLE roles ADD COLUMN is_above UUID UNIQUE REFERENCES roles(uuid) DEFAULT NULL;
