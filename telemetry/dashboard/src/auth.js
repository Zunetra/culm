// Who may see the dashboard.
//
// Cloudflare Access stands in front of this Worker and signs a JWT for every
// request it lets through. This module verifies that JWT itself on every
// request rather than trusting that Access is configured: a missing route, a
// disabled application, or a direct request all end in a 403 here. It fails
// closed: any placeholder or empty setting refuses everyone.

const CLOCK_SKEW_SECONDS = 60;
const JWKS_TTL_MS = 5 * 60 * 1000;
const PLACEHOLDER = /^(|replace_me.*)$/i;
const EMAIL = /^[^\s@,]+@[^\s@,]+\.[^\s@,]+$/;

/* Module scope survives between requests on the same isolate, so the keys are
 * fetched once every few minutes, not once per page view. */
let jwksCache = { url: null, fetchedAt: 0, keys: new Map() };

/* Returns the signed-in email, or null for "refuse". Never throws: every
 * failure is a refusal, and the reason is not shown to the caller. */
export async function authorise(request, env, fetchImpl = fetch) {
  try {
    const bypass = devBypass(request, env);
    if (bypass) return bypass;

    const settings = settingsFrom(env);
    if (!settings) return null;

    const token = request.headers.get("Cf-Access-Jwt-Assertion");
    if (!token) return null;

    const claims = await verify(token, settings, fetchImpl);
    if (!claims) return null;

    const email = typeof claims.email === "string" ? claims.email.toLowerCase() : null;
    if (!email || !settings.allowed.includes(email)) return null;
    return email;
  } catch {
    return null;
  }
}

/* Local development only: both a DEV_BYPASS_EMAIL setting (from .dev.vars,
 * which is never deployed) and a request to localhost are required. In
 * production the hostname is the workers.dev or custom domain, so the second
 * condition cannot hold even if the setting were deployed by mistake. */
export function devBypass(request, env) {
  const email = env.DEV_BYPASS_EMAIL;
  if (typeof email !== "string" || email.length === 0) return null;
  const host = new URL(request.url).hostname;
  if (host !== "localhost" && host !== "127.0.0.1") return null;
  return email.toLowerCase();
}

export function settingsFrom(env) {
  const team = (env.TEAM_DOMAIN ?? "").trim().toLowerCase();
  const aud = (env.ACCESS_AUD ?? "").trim();
  const allowed = (env.ALLOWED_EMAILS ?? "")
    .split(",")
    .map((e) => e.trim().toLowerCase())
    .filter((e) => !PLACEHOLDER.test(e) && EMAIL.test(e));
  if (PLACEHOLDER.test(team) || PLACEHOLDER.test(aud) || allowed.length === 0) return null;
  if (!/^[a-z0-9-]+\.cloudflareaccess\.com$/.test(team)) return null;
  return { team, aud, allowed, issuer: `https://${team}` };
}

/* RS256 only, signature first, then every claim that binds the token to this
 * application: issuer, audience, and the time window. */
export async function verify(token, settings, fetchImpl = fetch, now = Date.now()) {
  const parts = token.split(".");
  if (parts.length !== 3) return null;
  const [headerB64, payloadB64, signatureB64] = parts;

  const header = decodeJson(headerB64);
  const claims = decodeJson(payloadB64);
  if (!header || !claims) return null;
  if (header.alg !== "RS256" || typeof header.kid !== "string") return null;

  const key = await signingKey(settings.team, header.kid, fetchImpl, now);
  if (!key) return null;

  const signed = new TextEncoder().encode(`${headerB64}.${payloadB64}`);
  const signature = base64UrlBytes(signatureB64);
  if (!signature) return null;
  const valid = await crypto.subtle.verify("RSASSA-PKCS1-v1_5", key, signature, signed);
  if (!valid) return null;

  if (claims.iss !== settings.issuer) return null;
  const audiences = Array.isArray(claims.aud) ? claims.aud : [claims.aud];
  if (!audiences.includes(settings.aud)) return null;

  const seconds = Math.floor(now / 1000);
  if (typeof claims.exp !== "number" || claims.exp + CLOCK_SKEW_SECONDS < seconds) return null;
  if (typeof claims.nbf === "number" && claims.nbf - CLOCK_SKEW_SECONDS > seconds) return null;
  if (typeof claims.iat === "number" && claims.iat - CLOCK_SKEW_SECONDS > seconds) return null;
  return claims;
}

/* The team's current public keys. An unknown `kid` refetches once, so a key
 * rotation is picked up at once rather than after the cache expires. */
async function signingKey(team, kid, fetchImpl, now) {
  const url = `https://${team}/cdn-cgi/access/certs`;
  const fresh = jwksCache.url === url && now - jwksCache.fetchedAt < JWKS_TTL_MS;
  if (!fresh || !jwksCache.keys.has(kid)) {
    await refreshKeys(url, fetchImpl, now);
  }
  return jwksCache.keys.get(kid) ?? null;
}

async function refreshKeys(url, fetchImpl, now) {
  const response = await fetchImpl(url, { headers: { accept: "application/json" } });
  if (!response.ok) return;
  const body = await response.json();
  const keys = new Map();
  for (const jwk of Array.isArray(body.keys) ? body.keys : []) {
    if (jwk.kty !== "RSA" || typeof jwk.kid !== "string") continue;
    const key = await crypto.subtle.importKey(
      "jwk",
      { kty: "RSA", n: jwk.n, e: jwk.e, alg: "RS256", ext: true },
      { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" },
      false,
      ["verify"],
    );
    keys.set(jwk.kid, key);
  }
  jwksCache = { url, fetchedAt: now, keys };
}

/* For tests: forget fetched keys between cases. */
export function resetKeyCache() {
  jwksCache = { url: null, fetchedAt: 0, keys: new Map() };
}

function decodeJson(segment) {
  const bytes = base64UrlBytes(segment);
  if (!bytes) return null;
  try {
    const value = JSON.parse(new TextDecoder().decode(bytes));
    return typeof value === "object" && value !== null ? value : null;
  } catch {
    return null;
  }
}

function base64UrlBytes(segment) {
  if (!/^[A-Za-z0-9_-]*$/.test(segment)) return null;
  const base64 = segment.replace(/-/g, "+").replace(/_/g, "/");
  const padded = base64 + "=".repeat((4 - (base64.length % 4)) % 4);
  try {
    const binary = atob(padded);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
    return bytes;
  } catch {
    return null;
  }
}
