# Deploying and releasing

culm has exactly three things that leave this repository:

| What | Where it goes | How often | Section |
| --- | --- | --- | --- |
| **The `culm` binary** | crates.io, GitHub Releases (later npm and Homebrew) | Every version | [Releasing culm](#-releasing-culm) |
| **The telemetry receiver**, `telemetry/worker/` | Cloudflare Workers and D1 | Once, then only when its code changes | [Deploying the telemetry receiver](#-deploying-the-telemetry-receiver) |
| **The private dashboard**, `telemetry/dashboard/` | Cloudflare Workers, behind Cloudflare Access | Once, then only when its code changes | [Deploying the dashboard](#-deploying-the-dashboard) |

Nothing else is deployed. There is no website and no server for the CLI
itself: a user's `cargo install culm` is the whole delivery.

Back to the [README](../README.md).

## 📍 Where things stand

As of 2026-11-04:

| Piece | State |
| --- | --- |
| crates.io | `0.1.1` published (sends telemetry schema 2). `Cargo.toml` is at `0.1.2` (schema 3), not published yet |
| Receiver | Live at `https://culm-telemetry.shownzy001.workers.dev`. Schema 3 migration applied (40 columns), redeployed, accepts schema 2 and 3 |
| Database | D1 `culm-telemetry` (`58896687-7bdd-42be-b1a6-a95b711d5425`), account `Shownzy001@gmail.com's Account` |
| Dashboard | Deployed at `https://culm-telemetry-dashboard.shownzy001.workers.dev`. Cloudflare Access **not set up yet**, so it answers `403 Forbidden` to everyone |
| npm, Homebrew, GitHub binaries | Not set up yet |

What is left, in order:

1. Set up Cloudflare Access for the dashboard:
   [Deploying the dashboard](#-deploying-the-dashboard), steps 3 to 7.
2. Publish 0.1.2 and tag it: [Releasing culm](#-releasing-culm).
3. The npm and Homebrew channels:
   [Publish to the other channels](#publish-to-the-other-channels).

Every command in this file also runs from the repository root, as
`pnpm telemetry:*` and `pnpm dashboard:*`; see
[Every command, from the repository root](#every-command-from-the-repository-root).

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
git tag -s v0.1.2 -m "v0.1.2"
git push origin v0.1.2
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

### Every command, from the repository root

Each step below also works from inside the Worker's folder, but you never
have to `cd`: every command has a root shortcut in the root
`package.json`.

| From the root | Does |
| --- | --- |
| `pnpm install` | Installs wrangler for both Workers |
| `pnpm telemetry:test` | The receiver's validation tests |
| `pnpm telemetry:dev` | Runs the receiver locally |
| `pnpm telemetry:deploy:dry-run` | Checks the receiver's config without deploying |
| `pnpm telemetry:deploy` | Deploys the receiver |
| `pnpm telemetry:tail` | Watches the live receiver's requests |
| `pnpm telemetry:migrate` | Creates the table and indexes from `schema.sql` on the live database, if missing; safe to rerun |
| `pnpm telemetry:migrate:local` / `pnpm telemetry:migrate:remote` | Creates the table from `schema.sql` |
| `pnpm telemetry:migrate:3:local` / `pnpm telemetry:migrate:3:remote` | Adds the schema 3 columns to an older table, once |
| `pnpm dashboard:test` | The dashboard's security and rendering tests |
| `pnpm dashboard:dev` | Runs the dashboard locally |
| `pnpm dashboard:deploy:dry-run` | Checks the dashboard's config without deploying |
| `pnpm dashboard:deploy` | Deploys the dashboard |
| `pnpm dashboard:tail` | Watches the live dashboard's requests |
| `pnpm workers:test` | Both Workers' tests |
| `pnpm telemetry:wrangler <args>` | Any wrangler command, run for the receiver |
| `pnpm dashboard:wrangler <args>` | Any wrangler command, run for the dashboard |

The two `wrangler` shortcuts take anything wrangler does, for example:

```bash
pnpm telemetry:wrangler login
pnpm telemetry:wrangler d1 list
pnpm telemetry:wrangler d1 execute culm-telemetry --remote --command "SELECT COUNT(*) FROM events"
```

### What it needs

- A Cloudflare account. The free tier is plenty: 100,000 Worker requests
  and 5 million D1 rows read per day, and 5 GB of storage.
- Node.js and pnpm, to run `wrangler`, Cloudflare's CLI. One
  `pnpm install` installs it for both Workers; nothing global is needed.

> **Where to run these.** The steps below are written from inside
> `telemetry/worker/`. From the repository root, use the shortcut instead
> (table above): `pnpm telemetry:deploy` rather than `pnpm run deploy`,
> and `pnpm telemetry:wrangler ...` rather than `pnpm exec wrangler ...`.
> A plain `pnpm run db:migrate:remote` at the root fails with
> `Missing script`, because that script belongs to the Worker.

### Step 1. Install and log in

```bash
cd telemetry/worker
cp .env.example .env
pnpm install
pnpm exec wrangler login
```

`.env` is optional when deploying from your own machine, since
`wrangler login` is enough. Fill it in only to deploy without a browser
(CI, a server): uncomment `CLOUDFLARE_API_TOKEN` and paste a token.
wrangler reads `.env` from this folder by itself.

`wrangler login` opens a browser; approve it. Then check which accounts
your login can see:

```bash
pnpm exec wrangler whoami
```

If it lists more than one account, set the right one's id as
`account_id` in `wrangler.toml` (step 3), or wrangler will ask every time.

**The first `pnpm install` stops with `ERR_PNPM_IGNORED_BUILDS`.** pnpm
does not run a dependency's install scripts until you approve them, and
wrangler needs two: `esbuild` and `workerd` download their binaries that
way. From the repository root, run

```bash
pnpm approve-builds
```

tick `esbuild`, `workerd`, and `sharp` (wrangler's own dependencies),
confirm, and run `pnpm install` again. pnpm records the approval in
`pnpm-workspace.yaml`; commit it so the next install needs no prompt.

The two Worker folders form a pnpm workspace with the repository root
(`pnpm-workspace.yaml`), so one `pnpm install` from any of them installs
both, into the root `pnpm-lock.yaml`. Any `pnpm run ...` checks the
install first and installs if it is missing.

These folders use pnpm, like the rest of the repository. `pnpm install`
honours `minimum-release-age` in `~/.npmrc`, so a wrangler release
younger than that is refused until it is old enough: that is the same
supply-chain guard culm applies to the workspaces it scaffolds.

### Step 2. Create the database

Only once, ever, per account:

```bash
pnpm exec wrangler d1 create culm-telemetry
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

If you have lost it: `pnpm exec wrangler d1 list`.

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
pnpm exec wrangler deploy --dry-run
```

It must list `env.DB (culm-telemetry)  D1 Database`.

### Step 4. Create the table

```bash
pnpm run db:migrate:local     # optional: rehearse on wrangler's local emulator
pnpm run db:migrate:remote    # the real database; answer "yes"
```

`db:migrate:local` touches only `.wrangler/` on your machine, so it
succeeds even with a wrong `database_id`. Only `db:migrate:remote` proves
the configuration. `schema.sql` uses `CREATE TABLE IF NOT EXISTS`, so
running it twice is harmless.

Check the table exists:

```bash
pnpm exec wrangler d1 execute culm-telemetry --remote \
  --command "SELECT name FROM sqlite_master WHERE type = 'table'"
```

It must list `events`.

### Step 5. Deploy

```bash
pnpm run deploy
```

It prints the Worker's URL, such as
`https://culm-telemetry.<your-subdomain>.workers.dev`. The Worker accepts
a `POST` on any path, so the endpoint culm uses is that URL plus
`/events`.

### Step 6. Prove an event lands

In one terminal, from `telemetry/worker/`, watch requests live:

```bash
pnpm run tail
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
pnpm exec wrangler d1 execute culm-telemetry --remote \
  --command "SELECT received_at, command, exit, shape FROM events ORDER BY event_id DESC LIMIT 5"
```

A `400` in the tail means the payload failed validation; the response
body names the field. That is a culm and receiver version mismatch:
deploy the receiver from the same commit as the culm that sent it.

### Step 7. Point culm at it

Set `DEFAULT_ENDPOINT` in `src/telemetry/mod.rs` to the URL plus
`/events`. It already is:
`https://culm-telemetry.shownzy001.workers.dev/events`. Change it only if
the receiver moves (a new account, or your own domain below). The change
reaches users only in the next published version of culm, so do it
**before** [publishing](#publish-to-cratesio).

Anyone can still redirect their own runs with `CULM_TELEMETRY_ENDPOINT`.

### Optional: your own domain

To serve it at `telemetry.culm.dev` instead of `workers.dev`, the
`culm.dev` zone must be on the same Cloudflare account. Then, in the
dashboard: **Workers & Pages, culm-telemetry, Settings, Domains &
Routes, Add, Custom domain**, and enter `telemetry.culm.dev`. Update
`DEFAULT_ENDPOINT` to match, in the next release.

### Day to day

| Task | From the repository root |
| --- | --- |
| Watch live | `pnpm telemetry:tail` |
| Count distinct machines | `pnpm telemetry:wrangler d1 execute culm-telemetry --remote --command "SELECT COUNT(DISTINCT machine_id) FROM events"` |
| Most chosen layer sets | `pnpm telemetry:wrangler d1 execute culm-telemetry --remote --command "SELECT layers, COUNT(*) FROM events GROUP BY layers ORDER BY COUNT(*) DESC LIMIT 20"` |
| Redeploy after changing `src/index.js` | `pnpm telemetry:test && pnpm telemetry:deploy` |
| See past deploys | `pnpm telemetry:wrangler deployments list` |
| Roll back a bad deploy | `pnpm telemetry:wrangler rollback` |

Or open the dashboard, once Access is set up.

### Changing the payload

The payload, `schema.sql`, the receiver's validation in `src/index.js`,
and [TELEMETRY.md](TELEMETRY.md) move together, and a schema change is a
decision 9 change first. When one changes:

1. Update all four in one commit, plus a numbered file in
   `telemetry/worker/migrations/` with the `ALTER TABLE` statements,
   since `CREATE TABLE IF NOT EXISTS` does not alter an existing table.
2. Apply that migration to the live database.
3. Deploy the receiver **before** publishing the culm that sends the new
   field, so no event is rejected in between.

### Rolling out schema 3

Schema 3 adds error codes, toolchain versions, time saved by the cache,
`--verify` mismatches, and more ([TELEMETRY.md](TELEMETRY.md)). culm
0.1.1 sends schema 2; 0.1.2 sends schema 3. Steps 1 to 3 were done on
2026-11-04; they are kept here for a fresh database or account. Do these
in order, from `telemetry/worker/` (root shortcuts in brackets),
**before** publishing 0.1.2:

1. **Run the receiver's own tests:**
   ```bash
   pnpm test                       # (pnpm telemetry:test)
   ```
2. **Add the new columns to the live database** (once only; a second
   run fails with `duplicate column name` and changes nothing):
   ```bash
   pnpm run db:migrate:3:remote    # (pnpm telemetry:migrate:3:remote)
   ```
   Answer `yes`. Check it worked:
   ```bash
   pnpm exec wrangler d1 execute culm-telemetry --remote \
     --command "SELECT COUNT(*) AS columns FROM pragma_table_info('events')"
   ```
   It must say `40`.
3. **Deploy the receiver:**
   ```bash
   pnpm run deploy                 # (pnpm telemetry:deploy)
   ```
   It now accepts schema 2 and schema 3, so 0.1.1 users keep working.
4. **Prove a schema 3 event lands**, as in
   [Step 6](#step-6-prove-an-event-lands), using a culm built from this
   repository. Then check the new columns:
   ```bash
   pnpm exec wrangler d1 execute culm-telemetry --remote \
     --command "SELECT schema, error_code, tool_rustc, tool_node FROM events ORDER BY event_id DESC LIMIT 3"
   ```
5. **Only now** publish the culm release that sends schema 3
   ([Releasing culm](#-releasing-culm)).

### Taking it down

```bash
pnpm exec wrangler delete                      # removes the Worker
pnpm exec wrangler d1 delete culm-telemetry    # removes the database and every event in it
```

Telemetry is on by default, so first change `DEFAULT_ENDPOINT` or the
default itself in a release. Anyone still on an older version keeps
sending to the deleted address; each send then fails silently after up
to 500 ms, once per command.

---

## 🔐 Deploying the dashboard

The dashboard (`telemetry/dashboard/`) is a private website that shows
everything in the telemetry database: overview, every event, machines,
scaffolding, the runner, errors, toolchains, and CSV or JSON export. It
is a **separate Worker** from the receiver and only ever reads.

Only you can open it. Two locks protect it:

1. **Cloudflare Access** stands in front of it. Opening it shows
   Cloudflare's login page, where you sign in with a one-time code sent
   to your email, or with GitHub. Anyone else is stopped there and never
   reaches the dashboard.
2. **The dashboard checks again itself.** On every request it verifies
   the signed token Access attaches: the signature, that it is for this
   application, that it has not expired, and that the email is on its
   own allow list. A request without a valid token gets `403 Forbidden`
   and no data, even if Access were somehow switched off.

While its settings are still `REPLACE_ME`, the dashboard refuses
**everyone**, you included. Deploying it before setting up Access is
safe; it just shows `Forbidden` until step 6 is done.

**Your Cloudflare account is not the same as Cloudflare Access.** The
account is your login to Cloudflare, used to deploy. Cloudflare Access is
a product inside that account (under **Zero Trust**) that puts a sign-in
page in front of a website. Having the account does not set Access up;
steps 3 to 6 do.

The steps are written from `telemetry/dashboard/`. From the repository
root, use `pnpm dashboard:deploy` for `pnpm run deploy`, `pnpm
dashboard:dev` for `pnpm run dev`, and `pnpm dashboard:wrangler ...` for
`pnpm exec wrangler ...`.

Cloudflare Access is part of Cloudflare Zero Trust, which is free for up
to 50 users. It may ask for a payment method when you first enable Zero
Trust; the free plan is not charged.

### Step 1. Look at it locally first (optional)

From `telemetry/dashboard/`:

```bash
pnpm install
cp .dev.vars.example .dev.vars        # sets a local-only sign-in bypass
pnpm exec wrangler d1 execute culm-telemetry --local --file=../worker/schema.sql
pnpm run dev
```

From the root, the last two are
`pnpm dashboard:wrangler d1 execute culm-telemetry --local --file=../worker/schema.sql`
and `pnpm dashboard:dev`.

Open <http://localhost:8787>. Locally the dashboard reads its **own**
local copy of the database, in `telemetry/dashboard/.wrangler/`. It is
separate from the receiver's local copy, which is why the third command
creates the table for it. It starts empty, so every page says there is
nothing yet; that is expected. The live data is only reachable through
the deployed dashboard.

The bypass in `.dev.vars` works **only** for requests to `localhost` or
`127.0.0.1`, `.dev.vars` is never deployed, and it is gitignored.

### Step 2. Deploy the Worker

```bash
cd telemetry/dashboard
pnpm install
pnpm exec wrangler login        # skip if already logged in
pnpm run deploy
```

It prints the dashboard's address, such as
`https://culm-telemetry-dashboard.shownzy001.workers.dev`. Open it: you
should see `Forbidden`. That is correct for now.

### Step 3. Turn on Zero Trust and pick a team name

1. Open the Cloudflare dashboard, choose the **Shownzy001@gmail.com's
   Account** account, and click **Zero Trust** in the left sidebar.
2. The first time, it asks for a **team name**. Choose something short,
   such as `zunetra`. Your **team domain** is then
   `zunetra.cloudflareaccess.com`. Write it down.
3. Choose the **Free** plan and finish the setup.

If Zero Trust is already set up, find the team domain under
**Settings, Custom Pages** (or **Settings, General**): it is shown as
`<team>.cloudflareaccess.com`.

### Step 4. Add the two ways to sign in

In Zero Trust, go to **Settings, Authentication, Login methods**.

**One-time PIN (email code).** Click **Add new**, choose **One-time
PIN**, and save. Nothing else to fill in.

**GitHub.**

1. In another tab, open GitHub, **Settings, Developer settings, OAuth
   Apps, New OAuth App**, and fill in:
   - **Application name:** `culm telemetry dashboard`
   - **Homepage URL:** `https://<team>.cloudflareaccess.com`
   - **Authorization callback URL:**
     `https://<team>.cloudflareaccess.com/cdn-cgi/access/callback`

   with your team name in place of `<team>`.
2. Click **Register application**, then **Generate a new client
   secret**. Copy the **Client ID** and the **Client secret** (the secret
   is shown once).
3. Back in Zero Trust, **Login methods, Add new, GitHub**: paste the
   Client ID and Client secret, save, then click **Test**. A GitHub
   page asks you to authorise the app; approve it, and the test reports
   success.

### Step 5. Put the dashboard behind Access

In Zero Trust, go to **Access, Applications, Add an application**, and
choose **Self-hosted**.

1. **Application name:** `culm telemetry dashboard`.
2. **Session duration:** `24 hours` (or shorter).
3. **Public hostname:** subdomain `culm-telemetry-dashboard`, domain
   `shownzy001.workers.dev`; that is, exactly the address step 2 printed.
4. **Login methods:** untick **Accept all available identity
   providers**, then tick only **One-time PIN** and **GitHub**.
5. Click **Next** to reach **Policies**, and add one:
   - **Policy name:** `Owner only`
   - **Action:** `Allow`
   - **Include:** selector **Emails**, value your own email address.
     For GitHub sign-in, use the email on your GitHub account, or add it
     as a second value.
6. Save the application.
7. Open the application again and find **Application Audience (AUD)
   Tag** under its **Overview** (or **Basic information**). It is a long
   string of letters and numbers. Copy it.

If the Cloudflare dashboard offers it instead, enabling **Cloudflare
Access** from the Worker's own page (**Workers & Pages,
culm-telemetry-dashboard, Settings, Domains & Routes**, the
`workers.dev` row) creates the same kind of application; then edit its
policy and login methods as in 4 and 5, and copy its AUD tag.

### Step 6. Tell the dashboard about Access

Edit `telemetry/dashboard/wrangler.toml`, replacing the three
`REPLACE_ME` values:

```toml
[vars]
TEAM_DOMAIN = "zunetra.cloudflareaccess.com"        # your team domain, from step 3
ACCESS_AUD = "4714c1358e65fe4b408ad6d432a5f878f08194bdb4752441fd56faefa9b2b6f2"   # your AUD tag, from step 5
ALLOWED_EMAILS = "you@example.com"                   # comma-separated if more than one
```

None of these is a secret: they let the dashboard *check* a token, never
create one. Then deploy again:

```bash
pnpm run deploy
```

### Step 7. Check both locks

1. **In a private (incognito) window**, open the dashboard address. You
   must see Cloudflare's sign-in page, not the dashboard. Sign in with
   the email code, then try GitHub: both must open the dashboard, with
   your email shown at the top right.
2. **Without Access**, the dashboard must refuse:
   ```bash
   curl -i https://culm-telemetry-dashboard.shownzy001.workers.dev/
   ```
   must answer with Cloudflare's login redirect (`302`) or `403
   Forbidden`, never the dashboard.
3. **Someone else** cannot get in: signing in with an email not on the
   policy is refused by Access itself.

### If something is wrong

| You see | Cause and fix |
| --- | --- |
| `Forbidden` after signing in | A setting in `wrangler.toml` does not match: the team domain, the AUD tag, or your email (`ALLOWED_EMAILS` is compared without case). Fix it and `pnpm run deploy` again |
| The dashboard opens without any sign-in page | The Access application's hostname does not match the dashboard's address. Fix it in step 5. The dashboard still refuses without a token, so you see `Forbidden`, not data |
| GitHub sign-in fails | The OAuth app's callback URL must be exactly `https://<team>.cloudflareaccess.com/cdn-cgi/access/callback` |
| `Something went wrong reading the telemetry database` (live) | The database is missing columns. Run the schema 3 migration ([Rolling out schema 3](#rolling-out-schema-3)) |
| `Something went wrong reading the telemetry database` (local) | The local copy has no table yet. Run the `d1 execute ... --local --file=../worker/schema.sql` line from step 1 |
| `Forbidden` on `localhost` | `.dev.vars` is missing, or `DEV_BYPASS_EMAIL` in it is empty. Copy it from `.dev.vars.example` and restart `pnpm run dev` |

### Day to day

- Sign out: the **Sign out** link at the top right.
- Change who can see it: edit the Access policy **and** `ALLOWED_EMAILS`,
  then `pnpm run deploy`.
- Ship a change: `pnpm dashboard:test && pnpm dashboard:deploy` from the
  root.
- Undo a bad deploy: `pnpm dashboard:wrangler rollback`.
- Remove it: `pnpm dashboard:wrangler delete`, then delete the Access
  application. The receiver and its data are not affected.

