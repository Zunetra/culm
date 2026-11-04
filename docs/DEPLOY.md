# Deploying and releasing

culm has exactly two things that leave this repository:

| What | Where it goes | How often | Section |
| --- | --- | --- | --- |
| **The `culm` binary** | crates.io, GitHub Releases (later npm and Homebrew) | Every version | [Releasing culm](#-releasing-culm) |
| **The telemetry receiver**, `telemetry/worker/` | Cloudflare Workers and D1 | Once, then only when its code changes | [Deploying the telemetry receiver](#-deploying-the-telemetry-receiver) |

Nothing else is deployed. There is no website and no server for the CLI
itself: a user's `cargo install culm` is the whole delivery.

Back to the [README](../README.md).

---

## 📦 Releasing culm

Pushing to `develop` or `main` never releases anything. It only runs
`ci.yml`. A release is two deliberate actions: publishing the crate, and
pushing a tag.

### Before every release

1. Everything for the release is merged into `main`, and CI is green on
   it.
2. `version` in `Cargo.toml` is the new version, and `Cargo.lock` agrees
   (`cargo build` updates it). crates.io never accepts the same version
   twice, and a published version can never be deleted, only yanked.
3. `version` in the root `package.json` matches. It is not published,
   but keeping it in step avoids confusion.
4. The gate passes locally:
   ```bash
   cargo test
   cargo lint
   cargo fmt --check
   cargo publish --dry-run
   ```

### Publish to crates.io

From `main`, with a clean working tree:

```bash
cargo login            # once per machine, with a token from crates.io/settings/tokens
cargo publish
```

Instead of `cargo login`, you can keep the token in `.env`:

```bash
cp .env.example .env              # uncomment CARGO_REGISTRY_TOKEN and paste the token
set -a && . ./.env && set +a
cargo publish
```

Check it worked: <https://crates.io/crates/culm> shows the new version,
and `cargo install culm` on another machine installs it.

To publish from GitHub Actions instead, add a `CARGO_REGISTRY_TOKEN`
repository secret (Settings, Secrets and variables, Actions) and use the
`publish` input described below. Do one or the other for a given
version, never both.

### Tag, so the binaries get built

```bash
git checkout main && git pull
git tag -s v0.1.1 -m "v0.1.1"
git push origin v0.1.1
```

The tag must match `Cargo.toml`'s version with a `v` in front. Pushing it
starts `release.yml`, which:

- builds five binaries: Linux x64 and arm64, macOS x64 and arm64,
  Windows x64
- creates the GitHub Release with those binaries, `SHA256SUMS`,
  `installer.sh`, and generated release notes

Do **not** create the release by hand on GitHub's "New release" page. The
workflow makes it.

Afterwards, open the release, choose **Edit**, tick **Set as a
pre-release** while culm is pre-alpha, and save.

### Publish to the other channels

Open **Actions, release, Run workflow**. Under **Use workflow from**,
choose the **tag** (not a branch, or every publish job skips itself), set
`publish`, and run.

| `publish` | Publishes to | Needs |
| --- | --- | --- |
| `crates` | crates.io | `CARGO_REGISTRY_TOKEN` secret |
| `npm` | npm, `culm` plus one package per platform | `NPM_TOKEN` secret, and the `culm` and `@zunetra` names claimed |
| `homebrew` | the `zunetra/homebrew-tap` formula | `HOMEBREW_TAP_TOKEN` secret, and that repository created |
| `all` | all three | all of the above |

### If something goes wrong

| Problem | Fix |
| --- | --- |
| Published a broken version to crates.io | `cargo yank --version X.Y.Z`, fix, bump to the next patch, publish again |
| Pushed a wrong tag, before publishing anything | `git push --delete origin vX.Y.Z && git tag -d vX.Y.Z`, then delete the GitHub Release it created |
| `release.yml` publish jobs all skipped | The workflow was run from a branch. Run it again from the tag |
| `cargo publish` says the version exists | Bump `version` in `Cargo.toml`; a version is never reusable |

---

## 📡 Deploying the telemetry receiver

The receiver is a Cloudflare Worker (`telemetry/worker/src/index.js`)
that validates each telemetry event and stores it as one row in a
D1 database. The exact payload is specified in
[TELEMETRY.md](TELEMETRY.md).

Telemetry is on by default from 0.1.1 (decision 9, amended 2026-11-04),
and `DEFAULT_ENDPOINT` points at this receiver, so keep it running. culm
itself works the same either way: a failed send is silent and never
delays a command by more than 500 ms.

### What it needs

- A Cloudflare account. The free tier is plenty: 100,000 Worker requests
  and 5 million D1 rows read per day, and 5 GB of storage.
- Node.js, to run `wrangler`, Cloudflare's CLI. It is installed into the
  folder by `npm install`; nothing global is needed.

> **Run every command below from `telemetry/worker/`.** Its own
> `package.json` holds the `npm run ...` scripts. From the repository
> root, `npm run db:migrate:remote` fails with `Missing script`, and a
> bare `npx wrangler ...` will not find `wrangler.toml`.

### Step 1. Install and log in

```bash
cd telemetry/worker
cp .env.example .env
npm install
npx wrangler login
```

`.env` is optional when deploying from your own machine, since
`wrangler login` is enough. Fill it in only to deploy without a browser
(CI, a server): uncomment `CLOUDFLARE_API_TOKEN` and paste a token.
wrangler reads `.env` from this folder by itself.

`wrangler login` opens a browser; approve it. Then check which accounts
your login can see:

```bash
npx wrangler whoami
```

If it lists more than one account, set the right one's id as
`account_id` in `wrangler.toml` (step 3), or wrangler will ask every time.

The line `npm warn Unknown user config "minimum-release-age"` comes from
a pnpm setting in `~/.npmrc`. npm ignores it; so can you.

### Step 2. Create the database

Only once, ever, per account:

```bash
npx wrangler d1 create culm-telemetry
```

It prints a snippet like this:

```json
{
  "d1_databases": [
    {
      "binding": "culm_telemetry",
      "database_name": "culm-telemetry",
      "database_id": "58896687-7bdd-42be-b1a6-a95b711d5425"
    }
  ]
}
```

**Do not paste that into `wrangler.toml`.** It is JSON, and the file is
TOML, so wrangler then fails with `Invalid TOML document`. Copy only the
`database_id` value.

If you have lost it: `npx wrangler d1 list`.

### Step 3. Fill in `wrangler.toml`

```toml
name = "culm-telemetry"
main = "src/index.js"
compatibility_date = "2026-09-08"

account_id = "<your account id, from wrangler whoami>"

[[d1_databases]]
binding = "DB"
database_name = "culm-telemetry"
database_id = "<the database_id from step 2>"
```

Two things to get right:

- **`binding` stays `DB`**, not the `culm_telemetry` that `d1 create`
  suggests. The code reads `env.DB`; any other name leaves the Worker
  without a database and every event fails.
- **`database_id` must be the real id.** The placeholder
  `REPLACE_ME_AFTER_D1_CREATE` produces
  `Invalid property: databaseId => Invalid uuid [code: 7400]`.

Neither id is a secret. Both are useless without a logged-in session or
an API token for the account, so the file is committed.

Check the file parses and the binding is right, without deploying:

```bash
npx wrangler deploy --dry-run
```

It must list `env.DB (culm-telemetry)  D1 Database`.

### Step 4. Create the table

```bash
npm run db:migrate:local     # optional: rehearse on wrangler's local emulator
npm run db:migrate:remote    # the real database; answer "yes"
```

`db:migrate:local` touches only `.wrangler/` on your machine, so it
succeeds even with a wrong `database_id`. Only `db:migrate:remote` proves
the configuration. `schema.sql` uses `CREATE TABLE IF NOT EXISTS`, so
running it twice is harmless.

Check the table exists:

```bash
npx wrangler d1 execute culm-telemetry --remote \
  --command "SELECT name FROM sqlite_master WHERE type = 'table'"
```

It must list `events`.

### Step 5. Deploy

```bash
npm run deploy
```

It prints the Worker's URL, such as
`https://culm-telemetry.<your-subdomain>.workers.dev`. The Worker accepts
a `POST` on any path, so the endpoint culm uses is that URL plus
`/events`.

### Step 6. Prove an event lands

In one terminal, from `telemetry/worker/`, watch requests live:

```bash
npm run tail
```

In another, build culm from the repository root, then send one real
event from a scratch folder (`culm new` creates its project in the
current directory):

```bash
cargo build
cd /tmp
CULM_TELEMETRY=1 \
CULM_TELEMETRY_ENDPOINT=https://culm-telemetry.<your-subdomain>.workers.dev/events \
~/Projects/Zunetra/culm/target/debug/culm new telemetry-check --preset backend --yes --no-install
rm -rf /tmp/telemetry-check
```

The tail shows a `POST` answered `204`. Then confirm the row:

```bash
npx wrangler d1 execute culm-telemetry --remote \
  --command "SELECT received_at, command, exit, shape FROM events ORDER BY event_id DESC LIMIT 5"
```

A `400` in the tail means the payload failed validation; the response
body names the field. That is a culm and receiver version mismatch:
deploy the receiver from the same commit as the culm that sent it.

### Step 7. Point culm at it

Set `DEFAULT_ENDPOINT` in `src/telemetry/mod.rs` to the URL plus
`/events`, and remove the comment above it saying there is no working
default. The change reaches users only in the next published version of
culm, so do this **before** [publishing](#publish-to-cratesio).

Anyone can still redirect their own runs with `CULM_TELEMETRY_ENDPOINT`.

### Optional: your own domain

To serve it at `telemetry.culm.dev` instead of `workers.dev`, the
`culm.dev` zone must be on the same Cloudflare account. Then, in the
dashboard: **Workers & Pages, culm-telemetry, Settings, Domains &
Routes, Add, Custom domain**, and enter `telemetry.culm.dev`. Update
`DEFAULT_ENDPOINT` to match, in the next release.

### Day to day

| Task | Command, from `telemetry/worker/` |
| --- | --- |
| Watch live | `npm run tail` |
| Count distinct machines | `npx wrangler d1 execute culm-telemetry --remote --command "SELECT COUNT(DISTINCT machine_id) FROM events"` |
| Most chosen layer sets | `npx wrangler d1 execute culm-telemetry --remote --command "SELECT layers, COUNT(*) FROM events GROUP BY layers ORDER BY COUNT(*) DESC LIMIT 20"` |
| Redeploy after changing `src/index.js` | `npm run deploy` |
| Roll back a bad deploy | `npx wrangler rollback` |

### Changing the payload

The payload, `schema.sql`, the receiver's validation in `src/index.js`,
and [TELEMETRY.md](TELEMETRY.md) move together, and a schema change is a
decision 9 change first. When one changes:

1. Update all four in one commit.
2. Apply new columns to the live database with an `ALTER TABLE`
   statement through `wrangler d1 execute --remote --command`, since
   `CREATE TABLE IF NOT EXISTS` does not alter an existing table.
3. Deploy the receiver **before** publishing the culm that sends the new
   field, so no event is rejected in between.

### Taking it down

```bash
npx wrangler delete                      # removes the Worker
npx wrangler d1 delete culm-telemetry    # removes the database and every event in it
```

Telemetry is on by default, so first change `DEFAULT_ENDPOINT` or the
default itself in a release. Anyone still on an older version keeps
sending to the deleted address; each send then fails silently after up
to 500 ms, once per command.
