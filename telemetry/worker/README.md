# culm telemetry receiver

Accepts the anonymous usage payload `src/telemetry/mod.rs` sends (decision 9
in `docs/CONTEXT.md`) and stores one row per event in a Cloudflare D1
database. The full, tested deploy walkthrough, including the mistakes that are
easy to make, is in [docs/DEPLOY.md](../../docs/DEPLOY.md).

## What it needs

A Cloudflare account (free tier is enough for this volume) and Node.js
installed locally, to run `wrangler` - Cloudflare's own CLI.

## First-time setup

From this directory (`telemetry/worker/`):

```sh
pnpm install
pnpm exec wrangler login
```

The second command opens a browser to authorize `wrangler` against your
Cloudflare account. Nothing is created yet.

## Create the database

```sh
pnpm exec wrangler d1 create culm-telemetry
```

This prints a `database_id`. Copy it into `wrangler.toml`, replacing
`REPLACE_ME_AFTER_D1_CREATE`.

## Apply the schema

```sh
pnpm run db:migrate:remote
```

Runs `schema.sql` against the real, hosted database. Use
`pnpm run db:migrate:local` first if you want to try it against wrangler's
local D1 emulator before touching the real one.

## Deploy

```sh
pnpm run deploy
```

Prints the Worker's live URL, something like
`https://culm-telemetry.<your-subdomain>.workers.dev`.

## Wire it up

Two places need that URL:

1. `src/telemetry/mod.rs`'s `DEFAULT_ENDPOINT` constant, so a released
   binary sends there by default.
2. Nowhere else, day to day - `CULM_TELEMETRY_ENDPOINT` overrides it without
   a rebuild, which is how this was tested against a throwaway endpoint
   before this file existed.

## Upgrading an existing database

`schema.sql` creates the table for a fresh database. A database created
before schema 3 needs the new columns added once:

```sh
pnpm run db:migrate:3:remote
```

Do this, then `pnpm run deploy`, **before** publishing the culm release
that sends schema 3. docs/DEPLOY.md, "Rolling out schema 3", has the
full order and checks. `pnpm test` runs the receiver's validation tests.

## Querying what lands

```sh
pnpm exec wrangler d1 execute culm-telemetry --remote \
  --command "SELECT layers, COUNT(*) FROM events GROUP BY layers ORDER BY COUNT(*) DESC LIMIT 20"
```

Or `pnpm exec wrangler d1 execute culm-telemetry --remote --command "SELECT COUNT(DISTINCT machine_id) FROM events"`
for a distinct-machine count. There's no dashboard here on purpose - this is
the minimum that works, not a product. A real dashboard (Grafana pointed at
D1, or exporting to somewhere with charts) is worth building once there's
enough data for it to say something.

## Watching it live

```sh
pnpm run tail
```

Streams every request the Worker handles as it happens - useful right after
a deploy, to confirm a real `culm new` run is actually reaching it.

## Cost

Cloudflare's free tier covers Workers (100,000 requests/day) and D1
(5GB storage, 5 million rows read/day) well past what a tool at this stage
would generate. Nothing here should cost anything until culm has an amount
of adoption worth a different conversation.
