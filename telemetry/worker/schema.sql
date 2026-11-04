-- The telemetry ingestion table, schema 3 (docs/TELEMETRY.md). Rows from
-- schema 2 clients leave the schema 3 columns NULL.
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
  -- Schema 3: which error, from the closed list in src/telemetry/codes.rs.
  error_code TEXT,
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
  -- Schema 3.
  held_back INTEGER,
  install_ms INTEGER,
  -- A JSON array of wizard screen names.
  wizard_changed TEXT,

  -- `culm run` only; NULL for every other command.
  run_projects INTEGER,
  run_tasks INTEGER,
  run_cacheable INTEGER,
  run_hits INTEGER,
  run_misses INTEGER,
  run_parallel INTEGER,
  run_affected INTEGER,
  run_remote_cache INTEGER,
  -- Schema 3.
  run_ms_saved INTEGER,
  run_verify_mismatches INTEGER,

  -- Schema 3: `culm graph` and `culm cache` only.
  graph_format TEXT,
  cache_action TEXT,

  -- Schema 3: `culm new` only, major.minor or NULL.
  tool_rustc TEXT,
  tool_cargo TEXT,
  tool_node TEXT,
  tool_pnpm TEXT,
  tool_go TEXT,

  received_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Answers "how many distinct machines" without a table scan.
CREATE INDEX IF NOT EXISTS events_machine_id ON events (machine_id);

-- Answers "what changed this week" without a table scan.
CREATE INDEX IF NOT EXISTS events_received_at ON events (received_at);

-- Answers "what breaks, and where" without a table scan.
CREATE INDEX IF NOT EXISTS events_exit ON events (command, exit, error_kind);

-- Answers "which errors, how often" without a table scan.
CREATE INDEX IF NOT EXISTS events_error_code ON events (error_code);
