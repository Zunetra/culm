// The culm telemetry receiver.
//
// Accepts exactly the payload src/telemetry/payload.rs sends (schema 2, as
// docs/TELEMETRY.md lists it), validates its shape, and inserts one row per
// event. Any key the spec does not list is a rejection, not something to
// ignore: decision 9's "never sent" list is enforced here too, so a client
// bug that starts sending more cannot quietly land in the table.

const SCHEMA = 2;
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const LAYER_PATTERN = /^[a-z0-9-]+\/[a-z0-9-]+$/;

const TOP_KEYS = [
  "schema", "id", "version", "os", "arch", "command", "exit",
  "error_kind", "duration_ms", "scaffold", "run",
];
const SCAFFOLD_KEYS = [
  "shape", "layers", "selected_via", "preset", "install",
  "install_failed_manager", "saved_preset", "cancelled_at",
];
const RUN_KEYS = [
  "projects", "tasks", "cacheable", "hits", "misses",
  "parallel", "affected", "remote_cache",
];

const COMMANDS = ["new", "run", "cache", "graph"];
const EXITS = ["ok", "error", "cancelled"];
const ERROR_KINDS = [
  "other", "select", "preset_fetch", "resolve", "merge", "write", "install",
  "version_resolve", "config", "graph", "task_failed", "cache",
];
const SHAPES = ["single-app", "monorepo", "tooling-only"];
const SELECTED_VIA = ["flags", "preset", "wizard"];
const INSTALLS = ["ok", "failed", "skipped", "none"];
const MANAGERS = ["pnpm", "cargo", "go"];
const CANCEL_STEPS = [
  "confirm", "shape", "frontend", "backend", "data", "toolchain",
  "quality", "infra", "editor", "runner",
];

export default {
  async fetch(request, env) {
    if (request.method !== "POST") {
      return new Response("Method Not Allowed", { status: 405 });
    }

    let body;
    try {
      body = await request.json();
    } catch {
      return new Response("Bad Request: invalid JSON", { status: 400 });
    }

    const error = validate(body);
    if (error) {
      return new Response(`Bad Request: ${error}`, { status: 400 });
    }

    const s = body.scaffold;
    const r = body.run;
    await env.DB.prepare(
      `INSERT INTO events (
         schema, machine_id, version, os, arch, command, exit, error_kind, duration_ms,
         shape, layers, selected_via, preset, install, install_failed_manager,
         saved_preset, cancelled_at,
         run_projects, run_tasks, run_cacheable, run_hits, run_misses,
         run_parallel, run_affected, run_remote_cache
       ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    )
      .bind(
        body.schema,
        body.id,
        body.version,
        body.os,
        body.arch,
        body.command,
        body.exit,
        body.error_kind,
        body.duration_ms,
        s?.shape ?? null,
        s ? JSON.stringify(s.layers) : null,
        s?.selected_via ?? null,
        s?.preset ?? null,
        s?.install ?? null,
        s?.install_failed_manager ?? null,
        s ? Number(s.saved_preset) : null,
        s?.cancelled_at ?? null,
        r?.projects ?? null,
        r?.tasks ?? null,
        r?.cacheable ?? null,
        r?.hits ?? null,
        r?.misses ?? null,
        r ? Number(r.parallel) : null,
        r ? Number(r.affected) : null,
        r ? Number(r.remote_cache) : null,
      )
      .run();

    // No body, no cache: this is a write-only endpoint, and the client is
    // fire-and-forget on a short timeout regardless of what comes back.
    return new Response(null, { status: 204 });
  },
};

/* Returns a message naming what is wrong, or `null` when the payload is
 * exactly the shape docs/TELEMETRY.md describes. */
export function validate(body) {
  if (!isObject(body)) return "payload must be a JSON object";

  const keys = exactKeys(body, TOP_KEYS, "");
  if (keys) return keys;

  if (body.schema !== SCHEMA) return `"schema" must be ${SCHEMA}`;
  if (typeof body.id !== "string" || !UUID_PATTERN.test(body.id)) return '"id" must be a UUID';
  for (const field of ["version", "os", "arch"]) {
    if (typeof body[field] !== "string" || body[field].length === 0 || body[field].length > 32) {
      return `"${field}" must be a short non-empty string`;
    }
  }
  if (!COMMANDS.includes(body.command)) return '"command" is not a culm command';
  if (!EXITS.includes(body.exit)) return '"exit" is not a known exit';
  if (!nullOr(body.error_kind, ERROR_KINDS)) return '"error_kind" is not a known stage';
  if (!isCount(body.duration_ms)) return '"duration_ms" must be a non-negative integer';

  if (body.scaffold !== null) {
    const error = validateScaffold(body.scaffold);
    if (error) return error;
  }
  if (body.run !== null) {
    const error = validateRun(body.run);
    if (error) return error;
  }
  return null;
}

function validateScaffold(s) {
  if (!isObject(s)) return '"scaffold" must be an object or null';
  const keys = exactKeys(s, SCAFFOLD_KEYS, "scaffold.");
  if (keys) return keys;

  if (!nullOr(s.shape, SHAPES)) return '"scaffold.shape" is not a shape';
  if (!Array.isArray(s.layers) || !s.layers.every((l) => typeof l === "string" && LAYER_PATTERN.test(l))) {
    return '"scaffold.layers" must be an array of <category>/<id> strings';
  }
  if (!nullOr(s.selected_via, SELECTED_VIA)) return '"scaffold.selected_via" is not known';
  if (s.preset !== null && (typeof s.preset !== "string" || !/^[a-z0-9-]{1,40}$/.test(s.preset))) {
    return '"scaffold.preset" must be a bare name or null';
  }
  if (!nullOr(s.install, INSTALLS)) return '"scaffold.install" is not known';
  if (!nullOr(s.install_failed_manager, MANAGERS)) return '"scaffold.install_failed_manager" is not known';
  if (typeof s.saved_preset !== "boolean") return '"scaffold.saved_preset" must be a boolean';
  if (!nullOr(s.cancelled_at, CANCEL_STEPS)) return '"scaffold.cancelled_at" is not a known step';
  return null;
}

function validateRun(r) {
  if (!isObject(r)) return '"run" must be an object or null';
  const keys = exactKeys(r, RUN_KEYS, "run.");
  if (keys) return keys;

  for (const field of ["projects", "tasks", "cacheable", "hits", "misses"]) {
    if (!isCount(r[field])) return `"run.${field}" must be a non-negative integer`;
  }
  for (const field of ["parallel", "affected", "remote_cache"]) {
    if (typeof r[field] !== "boolean") return `"run.${field}" must be a boolean`;
  }
  return null;
}

function exactKeys(object, allowed, prefix) {
  for (const key of Object.keys(object)) {
    if (!allowed.includes(key)) return `"${prefix}${key}" is not part of the payload`;
  }
  for (const key of allowed) {
    if (!(key in object)) return `"${prefix}${key}" is missing`;
  }
  return null;
}

function isObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isCount(value) {
  return Number.isInteger(value) && value >= 0;
}

function nullOr(value, allowed) {
  return value === null || allowed.includes(value);
}
