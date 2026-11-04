// What reaches SQL, HTML, and a downloaded CSV.

import { test } from "node:test";
import assert from "node:assert/strict";
import { eventFilters, period } from "../src/filters.js";
import { escape, securityHeaders } from "../src/html.js";
import { toCsv } from "../src/pages.js";

const url = (q) => new URL(`https://dash.example/events?${q}`);

test("filters keep valid values and drop anything else", () => {
  const good = eventFilters(url("period=7&command=run&exit=error&version=0.1.2&layer=backend/axum&error_code=runner_task_failed&schema=3"));
  assert.equal(good.values.command, "run");
  assert.equal(good.values.layer, "backend/axum");
  assert.equal(good.values.error_code, "runner_task_failed");
  assert.equal(good.params.length, 1 + 6);

  const hostile = eventFilters(url("command=run'%20OR%201=1--&version=1;DROP%20TABLE%20events&layer=../../etc&error_code=__proto__&os=LINUX&machine=x&schema=9"));
  assert.deepEqual(
    Object.entries(hostile.values).filter(([k, v]) => k !== "period" && v !== ""),
    [],
  );
  assert.equal(hostile.where, "WHERE received_at >= datetime('now', ?)");
});

test("SQL text never contains a value from the URL", () => {
  const f = eventFilters(url("version=0.1.2&layer=backend/axum"));
  assert.ok(!f.where.includes("0.1.2"));
  assert.ok(!f.where.includes("axum"));
});

test("an unknown period falls back to 30 days; all means unbounded", () => {
  assert.deepEqual(period(url("period=99999")).params, ["-30 days"]);
  assert.equal(period(url("period=all")).clause, "");
});

test("HTML escaping covers every special character", () => {
  assert.equal(escape(`<script>"x"&'y'</script>`), "&lt;script&gt;&quot;x&quot;&amp;&#39;y&#39;&lt;/script&gt;");
  assert.equal(escape(null), "");
});

test("CSV export quotes fields and defuses spreadsheet formulas", () => {
  const csv = toCsv([{ a: "=HYPERLINK(\"x\")", b: "plain", c: "has,comma", d: null, e: "-1+1" }]);
  const [, row] = csv.trim().split("\r\n");
  assert.equal(row, `"'=HYPERLINK(""x"")",plain,"has,comma",,'-1+1`);
});

test("every response forbids framing, scripts, caching, and sniffing", () => {
  const h = securityHeaders("text/html", "abc");
  assert.match(h["Content-Security-Policy"], /default-src 'none'/);
  assert.match(h["Content-Security-Policy"], /frame-ancestors 'none'/);
  assert.doesNotMatch(h["Content-Security-Policy"], /script-src|unsafe-inline/);
  assert.equal(h["X-Frame-Options"], "DENY");
  assert.match(h["Cache-Control"], /no-store/);
  assert.equal(h["X-Content-Type-Options"], "nosniff");
});
