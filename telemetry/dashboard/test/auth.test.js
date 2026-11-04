// The dashboard's only line of defence besides Cloudflare Access itself, so
// every way a token can be wrong is tried here, with real RS256 signatures.
// Runs with `npm test` (node --test); Node's WebCrypto matches the Worker's.

import { test, beforeEach } from "node:test";
import assert from "node:assert/strict";
import { authorise, devBypass, resetKeyCache, settingsFrom } from "../src/auth.js";

const TEAM = "zunetra.cloudflareaccess.com";
const AUD = "aud-tag-1234";
const OWNER = "owner@example.com";
const ENV = { TEAM_DOMAIN: TEAM, ACCESS_AUD: AUD, ALLOWED_EMAILS: `${OWNER}, second@example.com` };
const URL_PROD = "https://culm-telemetry-dashboard.example.workers.dev/";

const b64url = (bytes) => Buffer.from(bytes).toString("base64url");
const json64 = (value) => b64url(new TextEncoder().encode(JSON.stringify(value)));

async function keyPair() {
  return crypto.subtle.generateKey(
    { name: "RSASSA-PKCS1-v1_5", modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: "SHA-256" },
    true,
    ["sign", "verify"],
  );
}

const signer = await keyPair();
const stranger = await keyPair();
const publicJwk = await crypto.subtle.exportKey("jwk", signer.publicKey);

function jwksFetch(kid = "k1") {
  let calls = 0;
  const fetchImpl = async (url) => {
    calls += 1;
    assert.equal(url, `https://${TEAM}/cdn-cgi/access/certs`);
    return new Response(JSON.stringify({ keys: [{ ...publicJwk, kid, alg: "RS256", use: "sig" }] }));
  };
  fetchImpl.calls = () => calls;
  return fetchImpl;
}

async function token(overrides = {}, { key = signer.privateKey, header = {} } = {}) {
  const now = Math.floor(Date.now() / 1000);
  const claims = { iss: `https://${TEAM}`, aud: [AUD], email: OWNER, iat: now, nbf: now, exp: now + 600, ...overrides };
  const head = json64({ alg: "RS256", kid: "k1", typ: "JWT", ...header });
  const body = json64(claims);
  const signature = await crypto.subtle.sign("RSASSA-PKCS1-v1_5", key, new TextEncoder().encode(`${head}.${body}`));
  return `${head}.${body}.${b64url(new Uint8Array(signature))}`;
}

function request(jwt, url = URL_PROD) {
  return new Request(url, { headers: jwt ? { "Cf-Access-Jwt-Assertion": jwt } : {} });
}

beforeEach(() => resetKeyCache());

test("a valid Access token for the owner is let in", async () => {
  assert.equal(await authorise(request(await token()), ENV, jwksFetch()), OWNER);
});

test("email matching ignores case, and every listed address works", async () => {
  assert.equal(await authorise(request(await token({ email: "Owner@Example.COM" })), ENV, jwksFetch()), OWNER);
  assert.equal(await authorise(request(await token({ email: "second@example.com" })), ENV, jwksFetch()), "second@example.com");
});

test("no token is refused", async () => {
  assert.equal(await authorise(request(null), ENV, jwksFetch()), null);
});

test("a signed-in stranger is refused", async () => {
  assert.equal(await authorise(request(await token({ email: "someone@else.com" })), ENV, jwksFetch()), null);
  assert.equal(await authorise(request(await token({ email: undefined })), ENV, jwksFetch()), null);
});

test("a token signed by any other key is refused", async () => {
  const forged = await token({}, { key: stranger.privateKey });
  assert.equal(await authorise(request(forged), ENV, jwksFetch()), null);
});

test("a token with its payload changed after signing is refused", async () => {
  const [head, , sig] = (await token({ email: "someone@else.com" })).split(".");
  const tampered = `${head}.${json64({ iss: `https://${TEAM}`, aud: [AUD], email: OWNER, exp: 9999999999 })}.${sig}`;
  assert.equal(await authorise(request(tampered), ENV, jwksFetch()), null);
});

test("another application's audience or another team's issuer is refused", async () => {
  assert.equal(await authorise(request(await token({ aud: ["other-app"] })), ENV, jwksFetch()), null);
  assert.equal(await authorise(request(await token({ iss: "https://evil.cloudflareaccess.com" })), ENV, jwksFetch()), null);
});

test("an expired or not-yet-valid token is refused", async () => {
  const now = Math.floor(Date.now() / 1000);
  assert.equal(await authorise(request(await token({ exp: now - 3600 })), ENV, jwksFetch()), null);
  assert.equal(await authorise(request(await token({ nbf: now + 3600 })), ENV, jwksFetch()), null);
  assert.equal(await authorise(request(await token({ exp: undefined })), ENV, jwksFetch()), null);
});

test("alg none, HS256, and an unknown key id are refused", async () => {
  const unsigned = `${json64({ alg: "none", kid: "k1" })}.${json64({ email: OWNER })}.`;
  assert.equal(await authorise(request(unsigned), ENV, jwksFetch()), null);
  assert.equal(await authorise(request(await token({}, { header: { alg: "HS256" } })), ENV, jwksFetch()), null);
  assert.equal(await authorise(request(await token({}, { header: { kid: "nope" } })), ENV, jwksFetch()), null);
});

test("garbage tokens are refused without throwing", async () => {
  for (const junk of ["", "a.b", "a.b.c", "!!!.???.***", "x".repeat(10000)]) {
    assert.equal(await authorise(request(junk), ENV, jwksFetch()), null, junk.slice(0, 20));
  }
});

test("placeholder or missing settings refuse everyone, even with a valid token", async () => {
  const valid = request(await token());
  for (const broken of [
    { ...ENV, TEAM_DOMAIN: "REPLACE_ME" },
    { ...ENV, ACCESS_AUD: "REPLACE_ME" },
    { ...ENV, ALLOWED_EMAILS: "REPLACE_ME" },
    { ...ENV, ALLOWED_EMAILS: "" },
    { ...ENV, TEAM_DOMAIN: "evil.example.com" },
    {},
  ]) {
    assert.equal(settingsFrom(broken), null);
    assert.equal(await authorise(valid, broken, jwksFetch()), null);
  }
});

test("a fetched key is cached, and a rotated key is picked up at once", async () => {
  const fetchImpl = jwksFetch();
  await authorise(request(await token()), ENV, fetchImpl);
  await authorise(request(await token()), ENV, fetchImpl);
  assert.equal(fetchImpl.calls(), 1);
  /* Unknown kid: one refetch, then refusal. */
  await authorise(request(await token({}, { header: { kid: "rotated" } })), ENV, fetchImpl);
  assert.equal(fetchImpl.calls(), 2);
});

test("the development bypass works only on localhost and only when set", async () => {
  const env = { ...ENV, DEV_BYPASS_EMAIL: "dev@example.com" };
  assert.equal(devBypass(new Request("http://localhost:8788/"), env), "dev@example.com");
  assert.equal(devBypass(new Request("http://127.0.0.1:8788/"), env), "dev@example.com");
  for (const host of [URL_PROD, "http://localhost.evil.com/", "http://127.0.0.1.nip.io/", "http://[::1]/", "http://0.0.0.0/"]) {
    assert.equal(devBypass(new Request(host), env), null, host);
    assert.equal(await authorise(new Request(host), env, jwksFetch()), null, host);
  }
  assert.equal(devBypass(new Request("http://localhost:8788/"), ENV), null);
  assert.equal(devBypass(new Request("http://localhost:8788/"), { ...ENV, DEV_BYPASS_EMAIL: "" }), null);
});
