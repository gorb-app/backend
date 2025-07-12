-- This file should undo anything in `up.sql`
ALTER TABLE channels DROP COLUMN in_category;

DROP TABLE categories;
