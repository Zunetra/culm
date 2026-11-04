// The dashboard's pages: each one runs read-only, parameterised SELECTs and
// returns { title, body }. Every value shown passes through `escape`.
//
// Rows from schema 2 clients have the schema 3 columns NULL; every query
// below treats NULL as "not reported", never as zero.

import { ERROR_CODE_MEANINGS } from "./codes.js";
import { PAGE_SIZE, eventFilters, isUuid, pageNumber, period, query } from "./filters.js";
import {
  bars, columns, empty, escape, formatMs, formatNumber, percent, section, stat, table,
} from "./html.js";

const WIZARD_SCREENS = [
  "shape", "frontend", "backend", "data", "toolchain", "quality", "infra", "editor", "runner",
];
const TOOLS = ["rustc", "cargo", "node", "pnpm", "go"];

async function all(env, sql, params = []) {
  const result = await env.DB.prepare(sql).bind(...params).all();
  return result.results ?? [];
}

async function first(env, sql, params = []) {
  return (await env.DB.prepare(sql).bind(...params).first()) ?? {};
}

/* WHERE for the period plus any fixed conditions (literal SQL from this file only). */
function scoped(url, ...conditions) {
  const p = period(url);
  const clauses = [...conditions];
  if (p.clause) clauses.unshift(p.clause);
  return { where: clauses.length ? `WHERE ${clauses.join(" AND ")}` : "", params: p.params, period: p.value };
}

function eventLink(id) {
  return `<a href="/events/${Number(id)}">#${Number(id)}</a>`;
}

function machineLink(id) {
  return isUuid(id) ? `<a class="mono" href="/machines/${escape(id)}">${escape(id.slice(0, 8))}</a>` : escape(id);
}

function exitBadge(exit) {
  const cls = exit === "ok" ? "ok" : exit === "error" ? "bad" : "";
  return `<span class="${cls}">${escape(exit)}</span>`;
}

function codeCell(code) {
  if (!code) return "";
  const meaning = ERROR_CODE_MEANINGS[code] ?? "";
  return `<code>${escape(code)}</code>${meaning ? `<div class="note">${escape(meaning)}</div>` : ""}`;
}

function parseList(json) {
  try {
    const value = JSON.parse(json);
    return Array.isArray(value) ? value : [];
  } catch {
    return [];
  }
}

/* Fills the days with no events so a quiet stretch shows as a gap, not as nothing. */
function fillDays(rows, days) {
  if (!rows.length) return rows;
  const byDay = new Map(rows.map((r) => [r.day, r.value]));
  const end = new Date(`${rows[rows.length - 1].day}T00:00:00Z`);
  const start = days
    ? new Date(Date.UTC(end.getUTCFullYear(), end.getUTCMonth(), end.getUTCDate() - Number(days) + 1))
    : new Date(`${rows[0].day}T00:00:00Z`);
  const out = [];
  for (let d = start; d <= end && out.length < 730; d = new Date(d.getTime() + 86_400_000)) {
    const day = d.toISOString().slice(0, 10);
    out.push({ day, value: byDay.get(day) ?? 0 });
  }
  return out;
}

function quantiles(values) {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const at = (q) => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))];
  return { min: sorted[0], median: at(0.5), p90: at(0.9), max: sorted[sorted.length - 1], count: sorted.length };
}

function distribution(label, values, format) {
  const q = quantiles(values);
  if (!q) return empty("Nothing reported yet.");
  return `<dl class="kv"><dt>${escape(label)} reported</dt><dd>${formatNumber(q.count)}</dd>
<dt>Fastest</dt><dd>${escape(format(q.min))}</dd><dt>Median</dt><dd>${escape(format(q.median))}</dd>
<dt>90th percentile</dt><dd>${escape(format(q.p90))}</dd><dt>Slowest</dt><dd>${escape(format(q.max))}</dd></dl>`;
}

function eventsTable(rows) {
  return table(
    ["Event", "Received (UTC)", "Machine", "Version", "Command", "Exit", "Error code", "Platform", "Duration"],
    rows.map((r) => [
      eventLink(r.event_id),
      escape(r.received_at),
      machineLink(r.machine_id),
      escape(r.version),
      escape(r.command),
      exitBadge(r.exit),
      codeCell(r.error_code),
      escape(`${r.os} / ${r.arch}`),
      escape(formatMs(r.duration_ms)),
    ]),
    { numeric: [8] },
  );
}

// === Overview

export async function overview(env, url) {
  const s = scoped(url);
  const [totals, everything, returning, newMachines, perDay, versions, platforms, commands, recent] =
    await Promise.all([
      first(env, `SELECT COUNT(*) AS events, COUNT(DISTINCT machine_id) AS machines,
        MIN(received_at) AS first_seen, MAX(received_at) AS last_seen,
        SUM(exit = 'ok') AS ok, SUM(exit = 'error') AS errors FROM events ${s.where}`, s.params),
      first(env, "SELECT COUNT(*) AS events, COUNT(DISTINCT machine_id) AS machines FROM events"),
      first(env, `SELECT COUNT(*) AS n FROM (SELECT machine_id FROM events ${s.where}
        GROUP BY machine_id HAVING COUNT(DISTINCT date(received_at)) >= 2)`, s.params),
      s.period === "all"
        ? first(env, "SELECT COUNT(DISTINCT machine_id) AS n FROM events")
        : first(env, `SELECT COUNT(*) AS n FROM (SELECT machine_id, MIN(received_at) AS first_seen
            FROM events GROUP BY machine_id) WHERE first_seen >= datetime('now', ?)`, s.params),
      all(env, `SELECT date(received_at) AS day, COUNT(*) AS value FROM events ${s.where}
        GROUP BY day ORDER BY day`, s.params),
      all(env, `SELECT version AS label, COUNT(*) AS value, COUNT(DISTINCT machine_id) AS machines
        FROM events ${s.where} GROUP BY version ORDER BY value DESC`, s.params),
      all(env, `SELECT os || ' / ' || arch AS label, COUNT(DISTINCT machine_id) AS value
        FROM events ${s.where} GROUP BY label ORDER BY value DESC`, s.params),
      all(env, `SELECT command, COUNT(*) AS value, SUM(exit = 'ok') AS ok, SUM(exit = 'error') AS errors,
        SUM(exit = 'cancelled') AS cancelled FROM events ${s.where} GROUP BY command ORDER BY value DESC`, s.params),
      all(env, `SELECT * FROM events ${s.where} ORDER BY event_id DESC LIMIT 10`, s.params),
    ]);

  if (!everything.events) {
    return {
      title: "Overview",
      body: section("No events yet", `<p>Nothing has arrived since the receiver was deployed. Run any
        <code>culm</code> command with telemetry on (the default outside CI) and it appears here.</p>`),
    };
  }

  const stats = `<div class="stats">
${stat("Events", formatNumber(totals.events), `${formatNumber(everything.events)} all time`)}
${stat("Machines", formatNumber(totals.machines), `${formatNumber(everything.machines)} all time`)}
${stat("New machines", formatNumber(newMachines.n ?? 0), "first seen in this period")}
${stat("Returning machines", formatNumber(returning.n ?? 0), "active on 2 or more days")}
${stat("Success rate", percent(totals.ok ?? 0, totals.events) || "n/a", `${formatNumber(totals.errors ?? 0)} errors`)}
${stat("Latest event", totals.last_seen ? totals.last_seen.slice(0, 16) : "none", "UTC")}
</div>`;

  const commandRows = commands.map((c) => ({
    label: c.command,
    value: c.value,
    note: `${percent(c.ok, c.value)} ok, ${formatNumber(c.errors)} errors, ${formatNumber(c.cancelled)} cancelled`,
  }));

  const body = `${stats}
${section("Events per day", columns(fillDays(perDay, s.period === "all" ? null : s.period), { title: "Events per day" }))}
<div class="grid">
${section("Commands", bars(commandRows, { href: (r) => `/events${query({ period: s.period, command: r.label })}` }))}
${section("culm versions", bars(versions.map((v) => ({ label: v.label, value: v.value, note: `${formatNumber(v.machines)} machines` })),
    { href: (r) => `/events${query({ period: s.period, version: r.label })}` }), "Events per version; upgrades show as the newest version taking over.")}
${section("Platforms", bars(platforms), "Distinct machines per OS and architecture.")}
</div>
${section("Latest events", eventsTable(recent))}`;
  return { title: "Overview", body };
}

// === Events

export async function events(env, url) {
  const f = eventFilters(url);
  const page = pageNumber(url);
  const [count, rows, versions, codes] = await Promise.all([
    first(env, `SELECT COUNT(*) AS n FROM events ${f.where}`, f.params),
    all(env, `SELECT * FROM events ${f.where} ORDER BY event_id DESC LIMIT ? OFFSET ?`,
      [...f.params, PAGE_SIZE, (page - 1) * PAGE_SIZE]),
    all(env, "SELECT DISTINCT version FROM events ORDER BY version DESC LIMIT 100"),
    all(env, "SELECT DISTINCT error_code FROM events WHERE error_code IS NOT NULL ORDER BY error_code"),
  ]);
  const v = f.values;
  const option = (value, current, label = value) =>
    `<option value="${escape(value)}"${value === current ? " selected" : ""}>${escape(label)}</option>`;
  const select = (name, label, options) =>
    `<label>${escape(label)}<select name="${name}">${option("", v[name], "any")}${options.map((o) => option(o, v[name])).join("")}</select></label>`;
  const text = (name, label, placeholder) =>
    `<label>${escape(label)}<input name="${name}" value="${escape(v[name])}" placeholder="${escape(placeholder)}" maxlength="64"></label>`;

  const form = `<form class="filters" method="get" action="/events">
<input type="hidden" name="period" value="${escape(v.period)}">
${select("command", "Command", ["new", "run", "cache", "graph"])}
${select("exit", "Exit", ["ok", "error", "cancelled"])}
${select("version", "Version", versions.map((r) => r.version))}
${select("error_code", "Error code", codes.map((r) => r.error_code))}
${select("schema", "Schema", ["2", "3"])}
${text("os", "OS", "linux")}
${text("layer", "Layer", "backend/axum")}
${text("machine", "Machine id", "full id")}
<button type="submit">Filter</button>
<a href="/events?period=${escape(v.period)}">Clear</a>
</form>`;

  const total = count.n ?? 0;
  const pages = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const pager = `<div class="pager">${page > 1 ? `<a href="/events${query(v, { page: page - 1 })}">Newer</a>` : ""}
<span>Page ${page} of ${pages} · ${formatNumber(total)} events</span>
${page < pages ? `<a href="/events${query(v, { page: page + 1 })}">Older</a>` : ""}</div>`;
  const exports = `<p>Export these ${formatNumber(total)} events:
<a href="/export.csv${query(v)}">CSV</a> · <a href="/export.json${query(v)}">JSON</a></p>`;

  return {
    title: "Events",
    body: `${section("Filter", form)}${section("Events", `${exports}${eventsTable(rows)}${pager}`)}`,
  };
}

export async function eventDetail(env, id) {
  const row = await first(env, "SELECT * FROM events WHERE event_id = ?", [id]);
  if (!row.event_id) return null;
  const items = Object.entries(row).map(([key, value]) => {
    let shown;
    if (value === null) shown = `<span class="note">not reported</span>`;
    else if (key === "layers" || key === "wizard_changed") shown = escape(parseList(value).join(", ") || "(none)");
    else if (key === "machine_id") shown = machineLink(value);
    else if (key === "error_code") shown = codeCell(value);
    else if (key.endsWith("_ms")) shown = `${escape(formatNumber(value))} <span class="note">(${escape(formatMs(value))})</span>`;
    else shown = escape(value);
    return `<dt>${escape(key)}</dt><dd>${shown}</dd>`;
  });
  return {
    title: `Event #${Number(id)}`,
    body: section("Stored row", `<dl class="kv">${items.join("")}</dl>`,
      "Exactly what the receiver stored. Columns a schema 2 client never sent show as not reported."),
  };
}

// === Machines

export async function machines(env, url) {
  const s = scoped(url);
  const page = pageNumber(url);
  const [count, rows] = await Promise.all([
    first(env, `SELECT COUNT(DISTINCT machine_id) AS n FROM events ${s.where}`, s.params),
    all(env, `SELECT machine_id, MIN(received_at) AS first_seen, MAX(received_at) AS last_seen,
        COUNT(*) AS events, COUNT(DISTINCT date(received_at)) AS days,
        GROUP_CONCAT(DISTINCT version) AS versions, MAX(os || ' / ' || arch) AS platform,
        SUM(command = 'new') AS scaffolds, SUM(command = 'run') AS runs, SUM(exit = 'error') AS errors
      FROM events ${s.where} GROUP BY machine_id ORDER BY last_seen DESC LIMIT ? OFFSET ?`,
      [...s.params, PAGE_SIZE, (page - 1) * PAGE_SIZE]),
  ]);
  const total = count.n ?? 0;
  const pages = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const list = table(
    ["Machine", "First seen", "Last seen", "Events", "Active days", "Scaffolds", "Runs", "Errors", "Versions", "Platform"],
    rows.map((r) => [
      machineLink(r.machine_id), escape(r.first_seen), escape(r.last_seen), escape(formatNumber(r.events)),
      escape(r.days), escape(r.scaffolds), escape(r.runs), escape(r.errors),
      escape((r.versions ?? "").split(",").join(", ")), escape(r.platform),
    ]),
    { numeric: [3, 4, 5, 6, 7] },
  );
  const pager = `<div class="pager">${page > 1 ? `<a href="/machines${query({ period: s.period, page: page - 1 })}">Newer</a>` : ""}
<span>Page ${page} of ${pages} · ${formatNumber(total)} machines</span>
${page < pages ? `<a href="/machines${query({ period: s.period, page: page + 1 })}">Older</a>` : ""}</div>`;
  return {
    title: "Machines",
    body: section("Machines active in this period", `${list}${pager}`,
      "One row per anonymous id. An id is random and unlinkable; deleting its file on that machine starts a new one."),
  };
}

export async function machineDetail(env, id) {
  const [summary, rows] = await Promise.all([
    first(env, `SELECT COUNT(*) AS events, MIN(received_at) AS first_seen, MAX(received_at) AS last_seen,
      COUNT(DISTINCT date(received_at)) AS days, SUM(exit = 'error') AS errors,
      SUM(run_ms_saved) AS ms_saved FROM events WHERE machine_id = ?`, [id]),
    all(env, "SELECT * FROM events WHERE machine_id = ? ORDER BY event_id DESC LIMIT 500", [id]),
  ]);
  if (!summary.events) return null;
  const stats = `<div class="stats">
${stat("Events", formatNumber(summary.events))}
${stat("Active days", formatNumber(summary.days))}
${stat("Errors", formatNumber(summary.errors ?? 0))}
${stat("Time saved by cache", formatMs(summary.ms_saved ?? 0))}
${stat("First seen", (summary.first_seen ?? "").slice(0, 10))}
${stat("Last seen", (summary.last_seen ?? "").slice(0, 10))}
</div>`;
  return {
    title: `Machine ${id.slice(0, 8)}`,
    body: `${stats}${section("Timeline", eventsTable(rows), "Newest first, up to 500 events.")}`,
  };
}

// === Scaffolding

export async function scaffolding(env, url) {
  const s = scoped(url, "command = 'new'");
  const ok = scoped(url, "command = 'new'", "exit = 'ok'");
  const wizard = scoped(url, "command = 'new'", "selected_via = 'wizard'");
  const [totals, layers, combos, shapes, via, presets, cancels, changed, installs, managers, installMs, held] =
    await Promise.all([
      first(env, `SELECT COUNT(*) AS n, SUM(exit = 'ok') AS ok, SUM(exit = 'error') AS errors,
        SUM(exit = 'cancelled') AS cancelled, SUM(saved_preset) AS saved FROM events ${s.where}`, s.params),
      all(env, `SELECT j.value AS label, COUNT(*) AS value FROM events, json_each(events.layers) AS j
        ${ok.where} GROUP BY label ORDER BY value DESC, label`, ok.params),
      all(env, `SELECT layers, COUNT(*) AS value FROM events ${ok.where}
        GROUP BY layers ORDER BY value DESC LIMIT 20`, ok.params),
      all(env, `SELECT COALESCE(shape, '(none)') AS label, COUNT(*) AS value FROM events ${s.where}
        GROUP BY label ORDER BY value DESC`, s.params),
      all(env, `SELECT COALESCE(selected_via, '(not reached)') AS label, COUNT(*) AS value FROM events ${s.where}
        GROUP BY label ORDER BY value DESC`, s.params),
      all(env, `SELECT preset AS label, COUNT(*) AS value FROM events ${s.where} AND preset IS NOT NULL
        GROUP BY label ORDER BY value DESC`, s.params),
      all(env, `SELECT cancelled_at AS label, COUNT(*) AS value FROM events ${s.where} AND cancelled_at IS NOT NULL
        GROUP BY label`, s.params),
      all(env, `SELECT j.value AS label, COUNT(*) AS value FROM events, json_each(events.wizard_changed) AS j
        ${wizard.where} AND wizard_changed IS NOT NULL GROUP BY label`, wizard.params),
      all(env, `SELECT COALESCE(install, '(not reached)') AS label, COUNT(*) AS value FROM events ${s.where}
        GROUP BY label ORDER BY value DESC`, s.params),
      all(env, `SELECT install_failed_manager AS label, COUNT(*) AS value FROM events ${s.where}
        AND install_failed_manager IS NOT NULL GROUP BY label ORDER BY value DESC`, s.params),
      all(env, `SELECT install_ms FROM events ${s.where} AND install_ms IS NOT NULL LIMIT 20000`, s.params),
      first(env, `SELECT COUNT(held_back) AS reported, SUM(held_back > 0) AS affected, SUM(held_back) AS packages
        FROM events ${s.where}`, s.params),
    ]);

  if (!totals.n) return { title: "Scaffolding", body: section("Scaffolding", empty("No culm new runs in this period.")) };

  const wizardRuns = via.find((r) => r.label === "wizard")?.value ?? 0;
  const cancelBy = new Map(cancels.map((c) => [c.label, c.value]));
  let remaining = wizardRuns;
  const funnel = [];
  for (const screen of [...WIZARD_SCREENS, "confirm"]) {
    funnel.push({ label: screen, value: remaining, note: cancelBy.get(screen) ? `${cancelBy.get(screen)} left here` : "" });
    remaining -= cancelBy.get(screen) ?? 0;
  }
  funnel.push({ label: "scaffolded", value: Math.max(remaining, 0) });
  const changedBy = new Map(changed.map((c) => [c.label, c.value]));
  const changedRows = WIZARD_SCREENS.filter((screen) => changedBy.has(screen))
    .map((screen) => ({ label: screen, value: changedBy.get(screen) }));

  const comboRows = combos.map((c) => ({
    label: parseList(c.layers).filter((l) => l !== "base/base").join(" + ") || "(base only)",
    value: c.value,
  }));

  const stats = `<div class="stats">
${stat("Scaffolds", formatNumber(totals.n))}
${stat("Succeeded", percent(totals.ok ?? 0, totals.n), `${formatNumber(totals.ok ?? 0)} workspaces`)}
${stat("Failed", formatNumber(totals.errors ?? 0))}
${stat("Cancelled", formatNumber(totals.cancelled ?? 0))}
${stat("Presets saved", formatNumber(totals.saved ?? 0))}
${stat("Held back by age guard", formatNumber(held.packages ?? 0), `in ${formatNumber(held.affected ?? 0)} of ${formatNumber(held.reported ?? 0)} installs`)}
</div>`;

  const body = `${stats}
<div class="grid">
${section("Layers", bars(layers, { href: (r) => `/events${query({ period: s.period, layer: r.label, command: "new" })}` }), "How often each layer was in a successful scaffold.")}
${section("Layer combinations", bars(comboRows), "Exact sets, base/base left out. The top 20.")}
${section("Shapes", bars(shapes))}
${section("How layers were chosen", bars(via))}
${section("Presets", bars(presets), "First-party names, or only file or git for others.")}
${section("Installs", bars(installs))}
${section("Wizard funnel", wizardRuns ? bars(funnel) : empty("No wizard runs in this period."), "How many wizard sessions reached each screen.")}
${section("Wizard answers changed from default", changedRows.length ? bars(changedRows) : empty("No changes reported (schema 3 only)."), "Screens where people did not keep the default: candidates for a better default.")}
${section("Failed package managers", bars(managers))}
${section("Install time", distribution("Installs", installMs.map((r) => r.install_ms), formatMs), "Schema 3 only.")}
</div>`;
  return { title: "Scaffolding", body };
}

// === Runner

export async function runner(env, url) {
  const s = scoped(url, "command = 'run'");
  const [totals, perDay, savedPerDay, tasks, mismatches] = await Promise.all([
    first(env, `SELECT COUNT(*) AS runs, COUNT(DISTINCT machine_id) AS machines, SUM(exit = 'error') AS errors,
      SUM(run_tasks) AS tasks, SUM(run_cacheable) AS cacheable, SUM(run_hits) AS hits, SUM(run_misses) AS misses,
      SUM(run_ms_saved) AS ms_saved, COUNT(run_ms_saved) AS saved_reported,
      SUM(run_verify_mismatches) AS mismatches, SUM(run_parallel) AS parallel,
      SUM(run_affected) AS affected, SUM(run_remote_cache) AS remote FROM events ${s.where}`, s.params),
    all(env, `SELECT date(received_at) AS day, COUNT(*) AS value FROM events ${s.where}
      GROUP BY day ORDER BY day`, s.params),
    all(env, `SELECT date(received_at) AS day, COALESCE(SUM(run_ms_saved), 0) AS value FROM events ${s.where}
      GROUP BY day ORDER BY day`, s.params),
    all(env, `SELECT CAST(run_tasks AS TEXT) AS label, COUNT(*) AS value FROM events ${s.where}
      AND run_tasks IS NOT NULL GROUP BY run_tasks ORDER BY run_tasks`, s.params),
    all(env, `SELECT * FROM events ${s.where} AND run_verify_mismatches > 0 ORDER BY event_id DESC LIMIT 50`, s.params),
  ]);

  if (!totals.runs) return { title: "Runner", body: section("Runner", empty("No culm run in this period.")) };

  const looked = (totals.hits ?? 0) + (totals.misses ?? 0);
  const alert = (totals.mismatches ?? 0) > 0
    ? `<p class="alert">${formatNumber(totals.mismatches)} --verify mismatch(es): a cached result differed from a real run.
       This is the r0.2 checkpoint's failure signal. The runs are listed below.</p>`
    : `<p class="ok">No --verify mismatches in this period.</p>`;
  const days = s.period === "all" ? null : s.period;

  const stats = `<div class="stats">
${stat("Runs", formatNumber(totals.runs), `${formatNumber(totals.machines)} machines`)}
${stat("Cache hit rate", percent(totals.hits ?? 0, looked) || "n/a", `${formatNumber(totals.hits ?? 0)} of ${formatNumber(looked)} cacheable tasks`)}
${stat("Time saved", formatMs(totals.ms_saved ?? 0), totals.saved_reported ? `${formatMs((totals.ms_saved ?? 0) / totals.saved_reported)} per run` : "schema 3 only")}
${stat("Tasks run", formatNumber(totals.tasks ?? 0))}
${stat("Failed runs", formatNumber(totals.errors ?? 0))}
${stat("--verify mismatches", formatNumber(totals.mismatches ?? 0))}
</div>`;

  const flags = [
    { label: "--parallel", value: totals.parallel ?? 0 },
    { label: "--affected", value: totals.affected ?? 0 },
    { label: "remote cache configured", value: totals.remote ?? 0 },
  ];

  const body = `${stats}${section("Cache correctness", alert + (mismatches.length ? eventsTable(mismatches) : ""))}
<div class="grid">
${section("Runs per day", columns(fillDays(perDay, days), { title: "Runs per day" }))}
${section("Time saved per day", columns(fillDays(savedPerDay, days).map((r) => ({ ...r, value: Math.round(r.value / 1000) })), { title: "Seconds saved per day" }), "Seconds, from cache hits.")}
${section("Flags used", bars(flags), "Runs using each option.")}
${section("Tasks per run", bars(tasks), "How big a typical run is.")}
</div>`;
  return { title: "Runner", body };
}

// === Errors

export async function errors(env, url) {
  const s = scoped(url, "exit = 'error'");
  const scope = scoped(url);
  const [totals, byCommand, kinds, codes, perDay, managers, recent] = await Promise.all([
    first(env, `SELECT COUNT(*) AS events, SUM(exit = 'error') AS errors,
      COUNT(DISTINCT CASE WHEN exit = 'error' THEN machine_id END) AS machines FROM events ${scope.where}`, scope.params),
    all(env, `SELECT command AS label, SUM(exit = 'error') AS value, COUNT(*) AS total FROM events ${scope.where}
      GROUP BY command ORDER BY value DESC`, scope.params),
    all(env, `SELECT COALESCE(error_kind, 'other') AS label, COUNT(*) AS value FROM events ${s.where}
      GROUP BY label ORDER BY value DESC`, s.params),
    all(env, `SELECT error_code, COUNT(*) AS value, COUNT(DISTINCT machine_id) AS machines,
      MAX(received_at) AS last_seen FROM events ${s.where} GROUP BY error_code ORDER BY value DESC`, s.params),
    all(env, `SELECT date(received_at) AS day, COUNT(*) AS value FROM events ${s.where}
      GROUP BY day ORDER BY day`, s.params),
    all(env, `SELECT install_failed_manager AS label, COUNT(*) AS value FROM events ${s.where}
      AND install_failed_manager IS NOT NULL GROUP BY label ORDER BY value DESC`, s.params),
    all(env, `SELECT * FROM events ${s.where} ORDER BY event_id DESC LIMIT 20`, s.params),
  ]);

  const stats = `<div class="stats">
${stat("Errors", formatNumber(totals.errors ?? 0))}
${stat("Error rate", percent(totals.errors ?? 0, totals.events) || "n/a", `of ${formatNumber(totals.events ?? 0)} events`)}
${stat("Machines affected", formatNumber(totals.machines ?? 0))}
</div>`;
  if (!totals.errors) return { title: "Errors", body: `${stats}${section("Errors", empty("No errors in this period."))}` };

  const codeTable = table(
    ["Error code", "Events", "Machines", "Last seen", ""],
    codes.map((c) => [
      c.error_code ? codeCell(c.error_code) : `<span class="note">not reported (schema 2)</span>`,
      escape(formatNumber(c.value)),
      escape(formatNumber(c.machines)),
      escape(c.last_seen),
      c.error_code ? `<a href="/events${query({ period: s.period, exit: "error", error_code: c.error_code })}">events</a>` : "",
    ]),
    { numeric: [1, 2] },
  );
  const commandRows = byCommand.filter((c) => c.value > 0)
    .map((c) => ({ label: c.label, value: c.value, note: `${percent(c.value, c.total)} of ${formatNumber(c.total)} runs` }));

  const body = `${stats}
${section("By error code", codeTable, "Codes name the exact error type. The message itself is never sent.")}
<div class="grid">
${section("Errors per day", columns(fillDays(perDay, s.period === "all" ? null : s.period), { title: "Errors per day" }))}
${section("By stage", bars(kinds))}
${section("By command", bars(commandRows))}
${section("Failed package managers", bars(managers))}
</div>
${section("Latest errors", eventsTable(recent))}`;
  return { title: "Errors", body };
}

// === Toolchains

export async function toolchains(env, url) {
  const s = scoped(url, "command = 'new'", "schema >= 3");
  const reported = await first(env, `SELECT COUNT(DISTINCT machine_id) AS machines FROM events ${s.where}`, s.params);
  if (!reported.machines) {
    return {
      title: "Toolchains",
      body: section("Toolchains", empty("No culm new runs from a schema 3 client in this period. Tool versions arrive from culm 0.1.2 on.")),
    };
  }
  const sections = await Promise.all(TOOLS.map(async (tool) => {
    /* Each machine counted once per version, from its newest report. */
    const rows = await all(env, `SELECT COALESCE(v, '(not installed)') AS label, COUNT(*) AS value FROM (
        SELECT machine_id, tool_${tool} AS v, MAX(event_id) FROM events ${s.where} GROUP BY machine_id
      ) GROUP BY label ORDER BY value DESC`, s.params);
    return section(tool, bars(rows), "Distinct machines, by their latest report.");
  }));
  return {
    title: "Toolchains",
    body: `<div class="stats">${stat("Machines reporting", formatNumber(reported.machines), "culm new, schema 3")}</div>
<div class="grid">${sections.join("")}</div>`,
  };
}

// === Export

const EXPORT_LIMIT = 50_000;

export async function exportRows(env, url) {
  const f = eventFilters(url);
  return all(env, `SELECT * FROM events ${f.where} ORDER BY event_id DESC LIMIT ?`, [...f.params, EXPORT_LIMIT]);
}

/* RFC 4180 quoting, and a leading quote on anything a spreadsheet would run
 * as a formula: a stored value cannot become a command in Excel. */
export function toCsv(rows) {
  if (!rows.length) return "";
  const headers = Object.keys(rows[0]);
  const cell = (value) => {
    if (value === null || value === undefined) return "";
    let text = String(value);
    if (/^[=+\-@\t\r]/.test(text)) text = `'${text}`;
    return /[",\n\r]/.test(text) ? `"${text.replace(/"/g, '""')}"` : text;
  };
  return [headers.join(","), ...rows.map((r) => headers.map((h) => cell(r[h])).join(","))].join("\r\n") + "\r\n";
}
