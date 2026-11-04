// HTML, charts, and response headers.
//
// Everything is rendered on the server: no client script at all, so the
// Content-Security-Policy can forbid scripts outright. Charts are inline SVG
// with sizes as attributes, which the policy allows without 'unsafe-inline'.

export const PAGES = [
  ["/", "Overview"],
  ["/events", "Events"],
  ["/machines", "Machines"],
  ["/scaffolding", "Scaffolding"],
  ["/runner", "Runner"],
  ["/errors", "Errors"],
  ["/toolchains", "Toolchains"],
];

export const PERIODS = [
  ["7", "7 days"],
  ["30", "30 days"],
  ["90", "90 days"],
  ["365", "1 year"],
  ["all", "All time"],
];

export function escape(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/* Every response, HTML or not, carries the same strict headers. */
export function securityHeaders(contentType, nonce) {
  const style = nonce ? `style-src 'nonce-${nonce}'` : "style-src 'none'";
  return {
    "Content-Type": contentType,
    "Content-Security-Policy": [
      "default-src 'none'",
      style,
      "img-src 'self' data:",
      "form-action 'self'",
      "base-uri 'none'",
      "frame-ancestors 'none'",
    ].join("; "),
    "X-Frame-Options": "DENY",
    "X-Content-Type-Options": "nosniff",
    "Referrer-Policy": "no-referrer",
    "Cache-Control": "no-store, max-age=0",
    "Pragma": "no-cache",
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Resource-Policy": "same-origin",
    "Strict-Transport-Security": "max-age=63072000; includeSubDomains",
    "Permissions-Policy":
      "accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=(), interest-cohort=()",
    "X-Robots-Tag": "noindex, nofollow",
  };
}

export function nonce() {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  return btoa(String.fromCharCode(...bytes)).replace(/[+/=]/g, "");
}

export function page({ title, path, email, period, body, nonce: n }) {
  const nav = PAGES.map(
    ([href, label]) =>
      `<a href="${href}?period=${escape(period)}"${href === path ? ' aria-current="page"' : ""}>${label}</a>`,
  ).join("");
  const periods = PERIODS.map(
    ([value, label]) =>
      `<a href="${escape(path)}?period=${value}"${value === period ? ' aria-current="true"' : ""}>${label}</a>`,
  ).join("");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex, nofollow">
<title>${escape(title)} · culm telemetry</title>
<style nonce="${n}">${STYLES}</style>
</head>
<body>
<header>
  <div class="brand">culm telemetry</div>
  <nav class="pages">${nav}</nav>
  <div class="who">${escape(email)} · <a href="/cdn-cgi/access/logout">Sign out</a></div>
</header>
<main>
  <div class="titlebar"><h1>${escape(title)}</h1><nav class="periods" aria-label="Period">${periods}</nav></div>
  ${body}
</main>
</body>
</html>`;
}

export function stat(label, value, note = "") {
  return `<div class="stat"><div class="value">${escape(value)}</div><div class="label">${escape(label)}</div>${
    note ? `<div class="note">${escape(note)}</div>` : ""
  }</div>`;
}

export function section(title, content, hint = "") {
  return `<section><h2>${escape(title)}</h2>${hint ? `<p class="hint">${escape(hint)}</p>` : ""}${content}</section>`;
}

export function empty(message = "No events in this period.") {
  return `<p class="empty">${escape(message)}</p>`;
}

/* Horizontal bars: one row per label, the bar drawn as SVG so its width needs
 * no inline style. `rows` is [{ label, value, note? }]. */
export function bars(rows, { unit = "", href = null } = {}) {
  if (!rows.length) return empty();
  const max = Math.max(...rows.map((r) => r.value), 1);
  const items = rows
    .map((r) => {
      const width = Math.max(1, Math.round((r.value / max) * 200));
      const label = href ? `<a href="${escape(href(r))}">${escape(r.label)}</a>` : escape(r.label);
      return `<tr><th scope="row">${label}${r.note ? `<div class="note">${escape(r.note)}</div>` : ""}</th>
<td class="bar"><svg width="200" height="14" viewBox="0 0 200 14" aria-hidden="true"><rect class="fill" x="0" y="1" width="${width}" height="12" rx="2"/></svg></td>
<td class="num">${escape(formatNumber(r.value))}${escape(unit)}</td></tr>`;
    })
    .join("");
  return `<table class="bars">${items}</table>`;
}

/* A column chart over days. `rows` is [{ day: "YYYY-MM-DD", value }], already sorted. */
export function columns(rows, { title = "" } = {}) {
  if (!rows.length) return empty();
  const max = Math.max(...rows.map((r) => r.value), 1);
  const width = Math.max(rows.length * 14, 280);
  const height = 120;
  const step = width / rows.length;
  const bars = rows
    .map((r, i) => {
      const h = Math.max(1, Math.round((r.value / max) * (height - 20)));
      return `<rect class="fill" x="${(i * step + 2).toFixed(1)}" y="${height - h}" width="${Math.max(step - 4, 2).toFixed(1)}" height="${h}" rx="2"><title>${escape(r.day)}: ${escape(r.value)}</title></rect>`;
    })
    .join("");
  const first = escape(rows[0].day);
  const last = escape(rows[rows.length - 1].day);
  return `<figure class="columns"><svg viewBox="0 0 ${width} ${height}" width="100%" height="${height}" preserveAspectRatio="none" role="img" aria-label="${escape(title)}">${bars}</svg>
<figcaption><span>${first}</span><span>peak ${escape(formatNumber(max))}</span><span>${last}</span></figcaption></figure>`;
}

/* A plain table. `headers` is a list of labels, `rows` a list of arrays of
 * already-escaped cell HTML, so callers decide what is a link. */
export function table(headers, rows, { numeric = [] } = {}) {
  if (!rows.length) return empty();
  const head = headers.map((h, i) => `<th scope="col"${numeric.includes(i) ? ' class="num"' : ""}>${escape(h)}</th>`).join("");
  const body = rows
    .map((cells) => `<tr>${cells.map((c, i) => `<td${numeric.includes(i) ? ' class="num"' : ""}>${c}</td>`).join("")}</tr>`)
    .join("");
  return `<div class="scroll"><table class="data"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`;
}

export function formatNumber(value) {
  if (value === null || value === undefined || value === "") return "";
  const n = Number(value);
  if (!Number.isFinite(n)) return String(value);
  return n.toLocaleString("en-US", { maximumFractionDigits: 1 });
}

export function formatMs(ms) {
  if (ms === null || ms === undefined) return "";
  const n = Number(ms);
  if (n < 1000) return `${Math.round(n)} ms`;
  if (n < 60_000) return `${(n / 1000).toFixed(1)} s`;
  if (n < 3_600_000) return `${(n / 60_000).toFixed(1)} min`;
  return `${(n / 3_600_000).toFixed(1)} h`;
}

export function percent(part, whole) {
  if (!whole) return "";
  return `${Math.round((part / whole) * 100)}%`;
}

const STYLES = `
:root{--bg:#f7f8f6;--panel:#ffffff;--ink:#16211b;--muted:#5d6b62;--line:#dfe5df;--accent:#016630;--accent-soft:#e3f1e8;--bad:#b42318;--bad-soft:#fdecea;color-scheme:light dark}
@media (prefers-color-scheme:dark){:root{--bg:#0b1f16;--panel:#11291d;--ink:#e8f1eb;--muted:#9fb3a6;--line:#24412f;--accent:#4cc382;--accent-soft:#163826;--bad:#ff8a80;--bad-soft:#3a1714}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--ink);font:15px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif}
a{color:var(--accent)}
header{display:flex;flex-wrap:wrap;gap:8px 20px;align-items:center;padding:12px 16px;border-bottom:1px solid var(--line);background:var(--panel)}
.brand{font-weight:700}
nav.pages{display:flex;flex-wrap:wrap;gap:4px}
nav a{padding:4px 10px;border-radius:6px;text-decoration:none;color:var(--muted)}
nav a[aria-current]{background:var(--accent-soft);color:var(--accent);font-weight:600}
.who{margin-left:auto;color:var(--muted);font-size:13px}
main{max-width:1200px;margin:0 auto;padding:16px}
.titlebar{display:flex;flex-wrap:wrap;justify-content:space-between;align-items:center;gap:8px}
h1{font-size:22px;margin:8px 0}
h2{font-size:16px;margin:0 0 10px}
section{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:16px;margin:16px 0}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(320px,1fr));gap:16px}
.grid section{margin:0}
.stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:12px;margin:16px 0}
.stat{background:var(--panel);border:1px solid var(--line);border-radius:10px;padding:14px}
.stat .value{font-size:26px;font-weight:700}
.stat .label{color:var(--muted);font-size:13px}
.note,.hint{color:var(--muted);font-size:12px}
.hint{margin:-4px 0 10px}
.empty{color:var(--muted);font-style:italic}
table{border-collapse:collapse;width:100%}
.bars th{text-align:left;font-weight:500;padding:3px 10px 3px 0;max-width:340px;word-break:break-word}
.bars td{padding:3px 0}
.bar{width:210px}
.num{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap;padding-left:10px}
.fill{fill:var(--accent)}
.scroll{overflow-x:auto}
.data th,.data td{border-bottom:1px solid var(--line);padding:6px 8px;text-align:left;vertical-align:top}
.data th{color:var(--muted);font-weight:600;font-size:13px}
.data td{font-size:13px}
code,.mono{font-family:ui-monospace,SFMono-Regular,Menlo,monospace;font-size:12px}
figure.columns{margin:0}
figcaption{display:flex;justify-content:space-between;color:var(--muted);font-size:12px}
form.filters{display:flex;flex-wrap:wrap;gap:8px;align-items:end}
form.filters label{display:flex;flex-direction:column;font-size:12px;color:var(--muted)}
input,select{font:inherit;padding:5px 8px;border:1px solid var(--line);border-radius:6px;background:var(--bg);color:var(--ink)}
button{font:inherit;padding:6px 14px;border:0;border-radius:6px;background:var(--accent);color:#fff;cursor:pointer}
.alert{background:var(--bad-soft);color:var(--bad);border-radius:8px;padding:10px 12px;font-weight:600}
.ok{color:var(--accent);font-weight:600}
.bad{color:var(--bad);font-weight:600}
.pager{display:flex;gap:12px;margin-top:10px}
dl.kv{display:grid;grid-template-columns:max-content 1fr;gap:4px 16px;margin:0}
dl.kv dt{color:var(--muted)}
dl.kv dd{margin:0;word-break:break-word}
@media (max-width:600px){.bar{width:110px}.bar svg{width:100px}.who{margin-left:0}}
`;
