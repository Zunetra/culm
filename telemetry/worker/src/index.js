// The culm telemetry receiver.
//
// Accepts exactly the payload src/telemetry/payload.rs sends, schema 2 (culm
// 0.1.1) or schema 3 (later), as docs/TELEMETRY.md lists it, validates its
// shape, and inserts one row per event. Any key the spec does not list is a
// rejection, not something to ignore: decision 9's "never sent" list is
// enforced here too, so a client bug that starts sending more cannot quietly
// land in the table.

const SCHEMAS = [2, 3];
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const LAYER_PATTERN = /^[a-z0-9-]+\/[a-z0-9-]+$/;
const TOOL_VERSION_PATTERN = /^\d{1,4}\.\d{1,4}$/;

const TOP_KEYS = {
  2: [
    "schema", "id", "version", "os", "arch", "command", "exit",
    "error_kind", "duration_ms", "scaffold", "run",
  ],
  3: [
    "schema", "id", "version", "os", "arch", "command", "exit",
    "error_kind", "error_code", "duration_ms", "scaffold", "run",
    "graph", "cache", "tools",
  ],
};
const SCAFFOLD_KEYS = {
  2: [
    "shape", "layers", "selected_via", "preset", "install",
    "install_failed_manager", "saved_preset", "cancelled_at",
  ],
  3: [
    "shape", "layers", "selected_via", "preset", "install",
    "install_failed_manager", "saved_preset", "cancelled_at",
    "held_back", "install_ms", "wizard_changed",
  ],
};
const RUN_KEYS = {
  2: [
    "projects", "tasks", "cacheable", "hits", "misses",
    "parallel", "affected", "remote_cache",
  ],
  3: [
    "projects", "tasks", "cacheable", "hits", "misses",
    "parallel", "affected", "remote_cache", "ms_saved", "verify_mismatches",
  ],
};
const TOOL_KEYS = ["rustc", "cargo", "node", "pnpm", "go"];

const COMMANDS = ["new", "run", "cache", "graph"];
const EXITS = ["ok", "error", "cancelled"];
const ERROR_KINDS = [
  "other", "select", "preset_fetch", "resolve", "merge", "write", "install",
  "version_resolve", "config", "graph", "task_failed", "cache",
];
// Must equal `ALL` in src/telemetry/codes.rs; tests/telemetry_codes.rs checks.
export const ERROR_CODES = [
  "cas_corrupted", "cas_io", "cas_malformed", "cas_missing",
  "cas_no_cache_dir", "cli_cancelled", "cli_destination_not_a_directory",
  "cli_destination_not_empty", "cli_no_layers_chosen",
  "cli_no_task_or_affected", "cli_no_tasks_defined",
  "cli_project_name_empty", "cli_project_name_is_path",
  "cli_project_name_starts_with_dash", "cli_project_name_unusable",
  "cli_release_age", "install_dev_unsupported", "install_failed",
  "install_features_unsupported", "install_malformed_package_json",
  "install_no_root_package_json", "install_not_started",
  "install_unknown_manager", "io", "layer_category_empty",
  "layer_category_forbidden", "layer_category_overfull", "layer_conflicting",
  "layer_duplicate_id", "layer_malformed", "layer_missing_requirement",
  "layer_non_table_merge_value", "layer_shape_missing",
  "layer_single_app_has_no_app", "layer_unknown_layer",
  "layer_unknown_shape", "layer_unqualified_reference", "layer_unreadable",
  "merge_conflicting", "merge_missing_requirement", "merge_non_object_root",
  "merge_scalar_conflict", "merge_strategy_mismatch", "merge_type_conflict",
  "other", "preset_malformed", "preset_misnamed",
  "preset_remote_clone_failed", "preset_remote_git_not_started",
  "preset_remote_malformed", "preset_remote_missing_file",
  "preset_remote_no_cache_dir", "preset_unknown", "remote_cache_rejected",
  "remote_cache_unreachable", "runner_bad_pattern", "runner_cache",
  "runner_cache_mismatch", "runner_cycle", "runner_git_unavailable",
  "runner_malformed", "runner_nested_workspace", "runner_not_started",
  "runner_output_outside_project", "runner_task_failed",
  "runner_unknown_dependency", "runner_unknown_project",
  "runner_unknown_ref", "runner_unknown_task", "runner_unreadable",
  "scaffold_escapes_root", "scaffold_file_collision", "scaffold_io",
  "scaffold_template_missing",
];
const SHAPES = ["single-app", "monorepo", "tooling-only"];
const SELECTED_VIA = ["flags", "preset", "wizard"];
const INSTALLS = ["ok", "failed", "skipped", "none"];
const MANAGERS = ["pnpm", "cargo", "go"];
const CANCEL_STEPS = [
  "confirm", "shape", "frontend", "backend", "data", "toolchain",
  "quality", "infra", "editor", "runner",
];
const WIZARD_SCREENS = [
  "shape", "frontend", "backend", "data", "toolchain",
  "quality", "infra", "editor", "runner",
];
const GRAPH_FORMATS = ["list", "dot"];
const CACHE_ACTIONS = ["stats", "clear"];

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

    const row = toRow(body);
    const columns = Object.keys(row);
    await env.DB.prepare(
      `INSERT INTO events (${columns.join(", ")}) VALUES (${columns.map(() => "?").join(", ")})`,
    )
      .bind(...Object.values(row))
      .run();

    // No body, no cache: this is a write-only endpoint, and the client is
    // fire-and-forget on a short timeout regardless of what comes back.
    return new Response(null, { status: 204 });
  },
};

/* One validated payload as column values. Column names come only from this
 * function, never from the payload, so the INSERT above cannot be steered by
 * a client. Schema 2 payloads leave the schema 3 columns null. */
export function toRow(body) {
  const s = body.scaffold;
  const r = body.run;
  const t = body.tools ?? null;
  return {
    schema: body.schema,
    machine_id: body.id,
    version: body.version,
    os: body.os,
    arch: body.arch,
    command: body.command,
    exit: body.exit,
    error_kind: body.error_kind,
    error_code: body.error_code ?? null,
    duration_ms: body.duration_ms,
    shape: s?.shape ?? null,
    layers: s ? JSON.stringify(s.layers) : null,
    selected_via: s?.selected_via ?? null,
    preset: s?.preset ?? null,
    install: s?.install ?? null,
    install_failed_manager: s?.install_failed_manager ?? null,
    saved_preset: s ? Number(s.saved_preset) : null,
    cancelled_at: s?.cancelled_at ?? null,
    held_back: s?.held_back ?? null,
    install_ms: s?.install_ms ?? null,
    wizard_changed: s?.wizard_changed ? JSON.stringify(s.wizard_changed) : null,
    run_projects: r?.projects ?? null,
    run_tasks: r?.tasks ?? null,
    run_cacheable: r?.cacheable ?? null,
    run_hits: r?.hits ?? null,
    run_misses: r?.misses ?? null,
    run_parallel: r ? Number(r.parallel) : null,
    run_affected: r ? Number(r.affected) : null,
    run_remote_cache: r ? Number(r.remote_cache) : null,
    run_ms_saved: r?.ms_saved ?? null,
    run_verify_mismatches: r?.verify_mismatches ?? null,
    graph_format: body.graph?.format ?? null,
    cache_action: body.cache?.action ?? null,
    tool_rustc: t?.rustc ?? null,
    tool_cargo: t?.cargo ?? null,
    tool_node: t?.node ?? null,
    tool_pnpm: t?.pnpm ?? null,
    tool_go: t?.go ?? null,
  };
}

/* Returns a message naming what is wrong, or `null` when the payload is
 * exactly the shape docs/TELEMETRY.md describes for its schema. */
export function validate(body) {
  if (!isObject(body)) return "payload must be a JSON object";
  if (!SCHEMAS.includes(body.schema)) return `"schema" must be one of ${SCHEMAS.join(", ")}`;
  const schema = body.schema;

  const keys = exactKeys(body, TOP_KEYS[schema], "");
  if (keys) return keys;

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

  if (schema >= 3) {
    if (!nullOr(body.error_code, ERROR_CODES)) return '"error_code" is not a known code';
    if ((body.error_code === null) !== (body.exit !== "error")) {
      return '"error_code" must be set exactly when "exit" is "error"';
    }
    const error = validateGraph(body.graph) ?? validateCache(body.cache) ?? validateTools(body.tools);
    if (error) return error;
  }

  if (body.scaffold !== null) {
    const error = validateScaffold(body.scaffold, schema);
    if (error) return error;
  }
  if (body.run !== null) {
    const error = validateRun(body.run, schema);
    if (error) return error;
  }
  return null;
}

function validateScaffold(s, schema) {
  if (!isObject(s)) return '"scaffold" must be an object or null';
  const keys = exactKeys(s, SCAFFOLD_KEYS[schema], "scaffold.");
  if (keys) return keys;

  if (!nullOr(s.shape, SHAPES)) return '"scaffold.shape" is not a shape';
  if (!Array.isArray(s.layers) || s.layers.length > 100
    || !s.layers.every((l) => typeof l === "string" && l.length <= 80 && LAYER_PATTERN.test(l))) {
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

  if (schema >= 3) {
    if (s.held_back !== null && !isCount(s.held_back)) return '"scaffold.held_back" must be a count or null';
    if (s.install_ms !== null && !isCount(s.install_ms)) return '"scaffold.install_ms" must be a count or null';
    if (s.wizard_changed !== null) {
      if (!Array.isArray(s.wizard_changed) || s.wizard_changed.length > WIZARD_SCREENS.length
        || !s.wizard_changed.every((screen) => WIZARD_SCREENS.includes(screen))) {
        return '"scaffold.wizard_changed" must be an array of wizard screen names or null';
      }
    }
  }
  return null;
}

function validateRun(r, schema) {
  if (!isObject(r)) return '"run" must be an object or null';
  const keys = exactKeys(r, RUN_KEYS[schema], "run.");
  if (keys) return keys;

  const counts = ["projects", "tasks", "cacheable", "hits", "misses"];
  if (schema >= 3) counts.push("ms_saved", "verify_mismatches");
  for (const field of counts) {
    if (!isCount(r[field])) return `"run.${field}" must be a non-negative integer`;
  }
  for (const field of ["parallel", "affected", "remote_cache"]) {
    if (typeof r[field] !== "boolean") return `"run.${field}" must be a boolean`;
  }
  return null;
}

function validateGraph(g) {
  if (g === null) return null;
  if (!isObject(g)) return '"graph" must be an object or null';
  const keys = exactKeys(g, ["format"], "graph.");
  if (keys) return keys;
  if (!GRAPH_FORMATS.includes(g.format)) return '"graph.format" is not known';
  return null;
}

function validateCache(c) {
  if (c === null) return null;
  if (!isObject(c)) return '"cache" must be an object or null';
  const keys = exactKeys(c, ["action"], "cache.");
  if (keys) return keys;
  if (!CACHE_ACTIONS.includes(c.action)) return '"cache.action" is not known';
  return null;
}

function validateTools(t) {
  if (t === null) return null;
  if (!isObject(t)) return '"tools" must be an object or null';
  const keys = exactKeys(t, TOOL_KEYS, "tools.");
  if (keys) return keys;
  for (const tool of TOOL_KEYS) {
    if (t[tool] !== null && (typeof t[tool] !== "string" || !TOOL_VERSION_PATTERN.test(t[tool]))) {
      return `"tools.${tool}" must be major.minor or null`;
    }
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
  return Number.isInteger(value) && value >= 0 && value <= Number.MAX_SAFE_INTEGER;
}

function nullOr(value, allowed) {
  return value === null || allowed.includes(value);
}
