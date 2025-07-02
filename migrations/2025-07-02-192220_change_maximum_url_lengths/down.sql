-- This file should undo anything in `up.sql`
ALTER TABLE users ALTER COLUMN avatar TYPE varchar(100);
ALTER TABLE guilds ALTER COLUMN icon TYPE varchar(100);
