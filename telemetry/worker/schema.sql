-- The telemetry ingestion table, schema 2 (docs/TELEMETRY.md).
--
-- One row per event, not one row per machine: `machine_id` repeats across a
-- machine's whole history, since decision 9's id is anonymous and per-machine,
-- not per-run. `received_at` is what turns "which layers were selected" into
-- an adoption curve over time.
--
-- Never add a column here for anything decision 9 forbids: no repository
-- name, no file path, no task name, no environment variable value, no command
-- string, no git remote, no error message. A schema change is a decision 9
-- change first.
--
-- Nothing was ever deployed against schema 1, so this replaces it rather
-- than migrating it.

CREATE TABLE IF NOT EXISTS events (
  event_id INTEGER PRIMARY KEY AUTOINCREMENT,
  schema INTEGER NOT NULL,
  machine_id TEXT NOT NULL,
  version TEXT NOT NULL,
  os TEXT NOT NULL,
  arch TEXT NOT NULL,
  command TEXT NOT NULL,
  exit TEXT NOT NULL,
  error_kind TEXT,
  duration_ms INTEGER NOT NULL,

  -- `culm new` only; NULL for every other command.
  shape TEXT,
  -- A JSON array, not a joined string, so a query never has to assume a
  -- separator cannot appear in an id.
  layers TEXT,
  selected_via TEXT,
  preset TEXT,
  install TEXT,
  install_failed_manager TEXT,
  saved_preset INTEGER,
  cancelled_at TEXT,

  -- `culm run` only; NULL for every other command.
  run_projects INTEGER,
  run_tasks INTEGER,
  run_cacheable INTEGER,
  run_hits INTEGER,
  run_misses INTEGER,
  run_parallel INTEGER,
  run_affected INTEGER,
  run_remote_cache INTEGER,

  received_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Answers "how many distinct machines" without a table scan.
CREATE INDEX IF NOT EXISTS events_machine_id ON events (machine_id);

-- Answers "what changed this week" without a table scan.
CREATE INDEX IF NOT EXISTS events_received_at ON events (received_at);

-- Answers "what breaks, and where" without a table scan.
CREATE INDEX IF NOT EXISTS events_exit ON events (command, exit, error_kind);
