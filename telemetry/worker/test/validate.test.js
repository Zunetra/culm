// Runs with `npm test` (node --test), no dependencies: the validator is plain
// JavaScript, so it is tested without wrangler or a database.

import { test } from "node:test";
import assert from "node:assert/strict";
import { validate, toRow } from "../src/index.js";

const ID = "9f2c1e7a-4b3d-4a51-9c8e-2f0d6b7a1e33";

function schema2() {
  return {
    schema: 2, id: ID, version: "0.1.1", os: "linux", arch: "x86_64",
    command: "new", exit: "ok", error_kind: null, duration_ms: 4210,
    scaffold: {
      shape: "single-app", layers: ["base/base", "backend/axum"], selected_via: "preset",
      preset: "backend", install: "ok", install_failed_manager: null,
      saved_preset: false, cancelled_at: null,
    },
    run: null,
  };
}

function schema3() {
  const body = schema2();
  body.schema = 3;
  body.version = "0.1.2";
  body.error_code = null;
  body.graph = null;
  body.cache = null;
  body.tools = { rustc: "1.99", cargo: "1.99", node: "24.22", pnpm: null, go: null };
  Object.assign(body.scaffold, { held_back: 1, install_ms: 3100, wizard_changed: null });
  return body;
}

function run3() {
  const body = schema3();
  body.command = "run";
  body.scaffold = null;
  body.tools = null;
  body.run = {
    projects: 1, tasks: 2, cacheable: 1, hits: 1, misses: 0, parallel: false,
    affected: false, remote_cache: false, ms_saved: 1840, verify_mismatches: 0,
  };
  return body;
}

test("a schema 2 payload from culm 0.1.1 is accepted", () => {
  assert.equal(validate(schema2()), null);
});

test("schema 3 payloads for new, run, graph and cache are accepted", () => {
  assert.equal(validate(schema3()), null);
  assert.equal(validate(run3()), null);
  const graph = run3();
  graph.command = "graph";
  graph.run = null;
  graph.graph = { format: "dot" };
  assert.equal(validate(graph), null);
  const cache = run3();
  cache.command = "cache";
  cache.run = null;
  cache.cache = { action: "clear" };
  assert.equal(validate(cache), null);
});

test("schema 3 fields are refused in a schema 2 payload", () => {
  const body = schema2();
  body.error_code = null;
  assert.match(validate(body), /error_code/);
});

test("a schema 2 payload missing nothing new is not mistaken for schema 3", () => {
  const body = schema3();
  delete body.tools;
  assert.match(validate(body), /tools/);
});

test("an error must carry a known error code, and only an error", () => {
  const failed = schema3();
  failed.exit = "error";
  failed.error_kind = "install";
  failed.error_code = "install_failed";
  assert.equal(validate(failed), null);

  failed.error_code = "not_a_code";
  assert.match(validate(failed), /error_code/);

  failed.error_code = null;
  assert.match(validate(failed), /error_code/);

  const ok = schema3();
  ok.error_code = "install_failed";
  assert.match(validate(ok), /error_code/);
});

test("anything that could carry free text is refused", () => {
  const message = schema3();
  message.error_message = "/home/alice/acme/culm.toml: No such file";
  assert.match(validate(message), /not part of the payload/);

  const fullVersion = schema3();
  fullVersion.tools.rustc = "1.99.1";
  assert.match(validate(fullVersion), /tools.rustc/);

  const pathAsPreset = schema3();
  pathAsPreset.scaffold.preset = "./team/our-setup.toml";
  assert.match(validate(pathAsPreset), /preset/);

  const screen = schema3();
  screen.scaffold.wizard_changed = ["backend", "my-secret-project"];
  assert.match(validate(screen), /wizard_changed/);
});

test("counts must be non-negative integers", () => {
  const body = run3();
  body.run.ms_saved = -1;
  assert.match(validate(body), /ms_saved/);
  body.run.ms_saved = 1.5;
  assert.match(validate(body), /ms_saved/);
});

test("a row never takes a column name from the payload", () => {
  const columns = Object.keys(toRow(schema3())).sort();
  assert.deepEqual(columns, Object.keys(toRow(schema2())).sort());
  assert.ok(columns.includes("tool_rustc"));
  assert.equal(toRow(schema2()).error_code, null);
  assert.equal(toRow(schema3()).tool_node, "24.22");
});
