// === The health endpoint, over the real application

import assert from "node:assert/strict";
import { test } from "node:test";

import { createApp } from "../src/fastify/app.js";

/* inject drives the router in process, so the test binds no port and cannot
 * collide with a running service. */
test("health reports ok", async () => {
  const app = createApp();
  const response = await app.inject({ method: "GET", url: "/health" });

  assert.equal(response.statusCode, 200);
  assert.deepEqual(response.json(), { status: "ok" });
});

test("an unknown path returns the shared error body", async () => {
  const app = createApp();
  const response = await app.inject({ method: "GET", url: "/nope" });

  assert.equal(response.statusCode, 404);
  assert.deepEqual(response.json(), { error: "not found" });
});
