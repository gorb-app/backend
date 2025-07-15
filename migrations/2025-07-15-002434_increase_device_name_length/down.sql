-- This file should undo anything in `up.sql`
ALTER TABLE refresh_tokens ALTER COLUMN device_name TYPE varchar(16);