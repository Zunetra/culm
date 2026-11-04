// Query parameters, validated before any of them reaches SQL.
//
// Every value is checked against a closed list or a strict pattern and then
// bound as a parameter; nothing from the URL is ever spliced into SQL text.
// An invalid value is dropped, as if it had not been given.

import { ERROR_CODE_MEANINGS } from "./codes.js";

const PERIOD_DAYS = { 7: 7, 30: 30, 90: 90, 365: 365 };
const COMMANDS = ["new", "run", "cache", "graph"];
const EXITS = ["ok", "error", "cancelled"];
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export const PAGE_SIZE = 50;

/* `period` defaults to 30 days; `all` means no time bound. */
export function period(url) {
  const value = url.searchParams.get("period") ?? "30";
  if (value === "all") return { value, clause: "", params: [] };
  const days = PERIOD_DAYS[value] ?? 30;
  return {
    value: String(days),
    clause: "received_at >= datetime('now', ?)",
    params: [`-${days} days`],
  };
}

/* The Events view's filters, plus the period. Returns the cleaned values (to
 * refill the form and build links) and the WHERE clause with its params. */
export function eventFilters(url) {
  const p = period(url);
  const get = (name) => (url.searchParams.get(name) ?? "").trim();
  const values = {
    period: p.value,
    version: match(get("version"), /^[0-9A-Za-z.+-]{1,32}$/),
    command: COMMANDS.includes(get("command")) ? get("command") : "",
    exit: EXITS.includes(get("exit")) ? get("exit") : "",
    os: match(get("os"), /^[a-z0-9_]{1,16}$/),
    layer: match(get("layer"), /^[a-z0-9-]+\/[a-z0-9-]+$/),
    error_code: Object.hasOwn(ERROR_CODE_MEANINGS, get("error_code")) ? get("error_code") : "",
    machine: match(get("machine"), UUID),
    schema: ["2", "3"].includes(get("schema")) ? get("schema") : "",
  };

  const clauses = [];
  const params = [];
  if (p.clause) {
    clauses.push(p.clause);
    params.push(...p.params);
  }
  const equal = { version: "version", command: "command", exit: "exit", os: "os", error_code: "error_code", machine: "machine_id" };
  for (const [field, column] of Object.entries(equal)) {
    if (values[field]) {
      clauses.push(`${column} = ?`);
      params.push(values[field]);
    }
  }
  if (values.schema) {
    clauses.push("schema = ?");
    params.push(Number(values.schema));
  }
  if (values.layer) {
    clauses.push("EXISTS (SELECT 1 FROM json_each(events.layers) WHERE json_each.value = ?)");
    params.push(values.layer);
  }
  return { values, where: clauses.length ? `WHERE ${clauses.join(" AND ")}` : "", params };
}

export function pageNumber(url) {
  const n = Number.parseInt(url.searchParams.get("page") ?? "1", 10);
  return Number.isInteger(n) && n >= 1 && n <= 100_000 ? n : 1;
}

export function isUuid(value) {
  return UUID.test(value);
}

/* Builds a query string from cleaned values, skipping empty ones. */
export function query(values, extra = {}) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries({ ...values, ...extra })) {
    if (value !== "" && value !== null && value !== undefined) params.set(key, String(value));
  }
  const text = params.toString();
  return text ? `?${text}` : "";
}

function match(value, pattern) {
  return pattern.test(value) ? value : "";
}
