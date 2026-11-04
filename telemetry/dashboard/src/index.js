// The private culm telemetry dashboard.
//
// A separate Worker from the public receiver in telemetry/worker/: that one
// only accepts writes, this one only reads. Every request is authorised
// first (see auth.js); an unauthorised request gets a bare 403 with no data,
// and nothing below `authorise` runs for it. Only GET and HEAD are served.

import { authorise } from "./auth.js";
import { isUuid, period as periodOf } from "./filters.js";
import { nonce, page, securityHeaders } from "./html.js";
import * as pages from "./pages.js";

const ROUTES = {
  "/": pages.overview,
  "/events": pages.events,
  "/machines": pages.machines,
  "/scaffolding": pages.scaffolding,
  "/runner": pages.runner,
  "/errors": pages.errors,
  "/toolchains": pages.toolchains,
};

export default {
  async fetch(request, env) {
    if (request.method !== "GET" && request.method !== "HEAD") {
      return plain(405, "Method Not Allowed", { Allow: "GET, HEAD" });
    }

    const email = await authorise(request, env);
    if (!email) return plain(403, "Forbidden");

    const url = new URL(request.url);
    try {
      return await route(url, env, email);
    } catch {
      /* Never echo a database or code error to the browser. */
      return plain(500, "Something went wrong reading the telemetry database.");
    }
  },
};

async function route(url, env, email) {
  const path = url.pathname.replace(/\/+$/, "") || "/";
  const period = periodOf(url).value;

  if (path === "/export.csv" || path === "/export.json") {
    const rows = await pages.exportRows(env, url);
    const stamp = new Date().toISOString().slice(0, 10);
    if (path === "/export.csv") {
      return new Response(pages.toCsv(rows), {
        headers: {
          ...securityHeaders("text/csv; charset=utf-8"),
          "Content-Disposition": `attachment; filename="culm-telemetry-${stamp}.csv"`,
        },
      });
    }
    return new Response(JSON.stringify(rows, null, 2), {
      headers: {
        ...securityHeaders("application/json; charset=utf-8"),
        "Content-Disposition": `attachment; filename="culm-telemetry-${stamp}.json"`,
      },
    });
  }

  let result = null;
  let nav = path;
  const event = path.match(/^\/events\/(\d{1,12})$/);
  const machine = path.match(/^\/machines\/([0-9a-fA-F-]{36})$/);
  if (ROUTES[path]) {
    result = await ROUTES[path](env, url);
  } else if (event) {
    result = await pages.eventDetail(env, Number(event[1]));
    nav = "/events";
  } else if (machine && isUuid(machine[1])) {
    result = await pages.machineDetail(env, machine[1].toLowerCase());
    nav = "/machines";
  }
  if (!result) return plain(404, "Not Found");

  const n = nonce();
  const html = page({ title: result.title, path: nav, email, period, body: result.body, nonce: n });
  return new Response(html, { headers: securityHeaders("text/html; charset=utf-8", n) });
}

function plain(status, text, extra = {}) {
  return new Response(text, { status, headers: { ...securityHeaders("text/plain; charset=utf-8"), ...extra } });
}
