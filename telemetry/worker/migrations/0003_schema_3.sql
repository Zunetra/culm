-- Schema 2 to schema 3, for a database created before schema 3.
-- `schema.sql` already has these columns for a fresh database; CREATE TABLE IF
-- NOT EXISTS never alters an existing table, which is why this file exists.
-- Run it once: SQLite has no ADD COLUMN IF NOT EXISTS, so a second run fails
-- with "duplicate column name" and changes nothing.

ALTER TABLE events ADD COLUMN error_code TEXT;
ALTER TABLE events ADD COLUMN held_back INTEGER;
ALTER TABLE events ADD COLUMN install_ms INTEGER;
ALTER TABLE events ADD COLUMN wizard_changed TEXT;
ALTER TABLE events ADD COLUMN run_ms_saved INTEGER;
ALTER TABLE events ADD COLUMN run_verify_mismatches INTEGER;
ALTER TABLE events ADD COLUMN graph_format TEXT;
ALTER TABLE events ADD COLUMN cache_action TEXT;
ALTER TABLE events ADD COLUMN tool_rustc TEXT;
ALTER TABLE events ADD COLUMN tool_cargo TEXT;
ALTER TABLE events ADD COLUMN tool_node TEXT;
ALTER TABLE events ADD COLUMN tool_pnpm TEXT;
ALTER TABLE events ADD COLUMN tool_go TEXT;

-- Answers "which errors, how often" without a table scan.
CREATE INDEX IF NOT EXISTS events_error_code ON events (error_code);
