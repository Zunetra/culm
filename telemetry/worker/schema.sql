-- The telemetry ingestion table.
--
-- One row per event, not one row per machine: `machine_id` repeats across a
-- machine's whole history, since decision 9's id is anonymous and per-machine,
-- not per-run. `received_at` is what turns "which layers were selected" into
-- an adoption curve over time.
--
-- Never add a column here for anything decision 9 forbids: no repository
-- name, no file path, no task name, no environment variable value, no command
-- string, no git remote. A schema change is a decision 9 change first.

CREATE TABLE IF NOT EXISTS events (
  event_id INTEGER PRIMARY KEY AUTOINCREMENT,
  machine_id TEXT NOT NULL,
  version TEXT NOT NULL,
  os TEXT NOT NULL,
  arch TEXT NOT NULL,
  command TEXT NOT NULL,
  -- A JSON array, not a joined string: a layer id can never contain a comma,
  -- but storing it as real JSON means a query never has to assume that.
  layers TEXT NOT NULL,
  preset TEXT,
  received_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Answers "how many distinct machines" without a table scan.
CREATE INDEX IF NOT EXISTS events_machine_id ON events (machine_id);

-- Answers "what changed this week" without a table scan.
CREATE INDEX IF NOT EXISTS events_received_at ON events (received_at);
