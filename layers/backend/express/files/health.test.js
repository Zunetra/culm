// === The health endpoint, over the real application

import assert from "node:assert/strict";
import { after, before, test } from "node:test";

import { createApp } from "../app.js";

let server;
let origin;

/* Port 0 so the test never collides with a running service or with the other
 * backend's test suite. */
before(async () => {
  server = createApp().listen(0, "127.0.0.1");
  await new Promise((resolve) => server.once("listening", resolve));
  origin = `http://127.0.0.1:${server.address().port}`;
});

after(() => {
  server.close();
});

test("health reports ok", async () => {
  const response = await fetch(`${origin}/health`);

  assert.equal(response.status, 200);
  assert.deepEqual(await response.json(), { status: "ok" });
});

test("an unknown path returns the shared error body", async () => {
  const response = await fetch(`${origin}/nope`);

  assert.equal(response.status, 404);
  assert.deepEqual(await response.json(), { error: "not found" });
});
