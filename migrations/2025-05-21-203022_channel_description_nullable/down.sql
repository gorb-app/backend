-- This file should undo anything in `up.sql`
UPDATE channels SET description = '' WHERE description IS NULL;
ALTER TABLE ONLY channels ALTER COLUMN description SET NOT NULL;
ALTER TABLE ONLY channels ALTER COLUMN description DROP DEFAULT;
