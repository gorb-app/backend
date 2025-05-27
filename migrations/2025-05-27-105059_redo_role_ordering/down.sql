-- This file should undo anything in `up.sql`
ALTER TABLE roles ADD COLUMN position int NOT NULL DEFAULT 0;
ALTER TABLE roles DROP COLUMN is_above;
