-- This file should undo anything in `up.sql`
DROP INDEX idx_unique_username_active;
DROP INDEX idx_unique_email_active;
DROP TABLE users;
