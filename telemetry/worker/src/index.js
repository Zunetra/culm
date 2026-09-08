// The culm telemetry receiver.
//
// Accepts exactly the payload src/telemetry/payload.rs sends, validates its
// shape, and inserts one row per event. Nothing here reshapes or enriches
// the payload: what the Rust side sends is what lands in the table, so a
// question about "what does culm collect" has one place to answer it from -
// this file and schema.sql, not this file and some transform in between.

const REQUIRED_STRING_FIELDS = ["id", "version", "os", "arch", "command"];
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

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

    await env.DB.prepare(
      `INSERT INTO events (machine_id, version, os, arch, command, layers, preset)
       VALUES (?, ?, ?, ?, ?, ?, ?)`,
    )
      .bind(
        body.id,
        body.version,
        body.os,
        body.arch,
        body.command,
        JSON.stringify(body.layers),
        body.preset ?? null,
      )
      .run();

    // No body, no cache: this is a write-only endpoint, and the client is
    // fire-and-forget on a short timeout regardless of what comes back.
    return new Response(null, { status: 204 });
  },
};

/* Returns a message naming what is wrong, or `null` when the payload is
 * exactly the shape decision 9 describes. */
function validate(body) {
  if (typeof body !== "object" || body === null || Array.isArray(body)) {
    return "payload must be a JSON object";
  }

  for (const field of REQUIRED_STRING_FIELDS) {
    if (typeof body[field] !== "string" || body[field].length === 0) {
      return `"${field}" must be a non-empty string`;
    }
  }

  if (!UUID_PATTERN.test(body.id)) {
    return '"id" must be a UUID';
  }

  if (!Array.isArray(body.layers) || !body.layers.every((l) => typeof l === "string")) {
    return '"layers" must be an array of strings';
  }

  if (body.preset !== null && body.preset !== undefined && typeof body.preset !== "string") {
    return '"preset" must be a string or null';
  }

  return null;
}
