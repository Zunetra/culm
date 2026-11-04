# culm telemetry dashboard

A private, read-only website over the telemetry database that
`telemetry/worker/` fills. Only the owner can open it.

- **Pages:** overview, every event (filterable, with each stored row),
  machines and their timelines, scaffolding (layers, combinations,
  wizard funnel, installs), the runner (cache hit rate, time saved,
  `--verify` mismatches), errors (by code, with what each code means),
  toolchains, and CSV or JSON export.
- **Security:** Cloudflare Access in front (email code or GitHub, owner
  only), and the Worker verifies Access's signed token on every request
  itself. With its settings still `REPLACE_ME`, it refuses everyone. No
  scripts, no third-party resources, no caching, read-only queries with
  bound parameters only.

Setup, step by step: [docs/DEPLOY.md, "Deploying the dashboard"](../../docs/DEPLOY.md#-deploying-the-dashboard).

```sh
pnpm install
pnpm test                          # auth, filters, escaping, CSV export
cp .dev.vars.example .dev.vars    # local-only sign-in bypass
pnpm run dev                       # http://localhost:8787
pnpm run deploy
```

`src/codes.js` explains each error code. It is generated from culm's
Rust error messages, and `tests/telemetry_codes.rs` in culm checks its
keys match the codes culm sends.
