# Telemetry

culm sends anonymous usage data, and prints the exact payload the first
time it does. This page is the complete specification,
not a summary. If something is not listed here, it is not sent. The code
that builds the payload is `src/telemetry/payload.rs`, and a test there
fails if a key is added without this page changing too.

**Opt out:** set `CULM_TELEMETRY=0` (or `false`, `off`, `no`), or
`DO_NOT_TRACK=1`. It is already off in CI. No feature is degraded when
telemetry is off.

---

## Why this page exists

Every tool in this category collects usage data and describes it as
"anonymous usage data" in one vague sentence. Publishing the exact
payload and printing it on first run costs nothing and is a
differentiator. It also constrains us: the payload cannot quietly grow,
because growing it means editing this file.

## The exact payload

One event per command. Every key is always present, `null` where it
does not apply. This is schema 3, sent from culm 0.1.2; culm 0.1.1 sends
schema 2, which is the same without the keys marked "schema 3" below.

```json
{
  "schema": 3,
  "id": "9f2c1e7a-4b3d-4a51-9c8e-2f0d6b7a1e33",
  "version": "0.1.2",
  "os": "linux",
  "arch": "x86_64",
  "command": "new",
  "exit": "ok",
  "error_kind": null,
  "error_code": null,
  "duration_ms": 4210,
  "scaffold": {
    "shape": "monorepo",
    "layers": ["base/base", "shape/monorepo", "frontend/nuxt", "backend/axum"],
    "selected_via": "wizard",
    "preset": null,
    "install": "ok",
    "install_failed_manager": null,
    "saved_preset": false,
    "cancelled_at": null,
    "held_back": 1,
    "install_ms": 3100,
    "wizard_changed": ["backend", "runner"]
  },
  "run": null,
  "graph": null,
  "cache": null,
  "tools": {
    "rustc": "1.99",
    "cargo": "1.99",
    "node": "24.22",
    "pnpm": "10.4",
    "go": null
  }
}
```

For `culm run`, `scaffold` and `tools` are `null` and `run` is:

```json
{
  "projects": 3,
  "tasks": 7,
  "cacheable": 5,
  "hits": 4,
  "misses": 1,
  "parallel": true,
  "affected": false,
  "remote_cache": false,
  "ms_saved": 21400,
  "verify_mismatches": 0
}
```

For `culm graph`, `graph` is `{ "format": "list" }` or `{ "format": "dot" }`.
For `culm cache`, `cache` is `{ "action": "stats" }` or `{ "action": "clear" }`.

| Field | Meaning |
| --- | --- |
| `schema` | Payload version. Incremented when this page changes. |
| `id` | Random UUID v4, generated once per machine. |
| `version` | culm's own version. |
| `os`, `arch` | Platform only. Not a kernel version, not a hostname. |
| `command` | The subcommand name alone: `new`, `run`, `cache`, `graph`. |
| `exit` | `ok`, `error`, or `cancelled`. |
| `error_kind` | On `error` only, the stage that failed, from a closed set: `select`, `preset_fetch`, `resolve`, `merge`, `write`, `install`, `version_resolve`, `config`, `graph`, `task_failed`, `cache`, `other`. **Never an error message.** |
| `error_code` | Schema 3. On `error` only, which error it was, from the closed list in `src/telemetry/codes.rs` (74 codes such as `cli_destination_not_empty`, `merge_scalar_conflict`, `runner_task_failed`, `install_failed`, or `other`). The code names the error's type, chosen in the code, never derived from its text. **Never an error message.** |
| `duration_ms` | Wall-clock time of the invocation. |
| `scaffold.shape` | `single-app`, `monorepo`, or `tooling-only`. |
| `scaffold.layers` | The resolved layer ids, defaults included. **Only layers that ship in culm**; a layer from `--layers-dir` or a typo is dropped. |
| `scaffold.selected_via` | `flags`, `preset`, or `wizard`. |
| `scaffold.preset` | A first-party preset's name, otherwise only `file`, `git`, or `unknown`. **Never the path or URL.** |
| `scaffold.install` | `ok`, `failed`, `skipped` (`--no-install`), or `none` (nothing to install). |
| `scaffold.install_failed_manager` | `pnpm`, `cargo`, or `go`. Never its arguments. |
| `scaffold.saved_preset` | Whether `--save-preset` was used. Never its name. |
| `scaffold.cancelled_at` | The wizard screen (`shape`, `frontend`, ...) or `confirm` where the user backed out. |
| `scaffold.held_back` | Schema 3. How many packages the release age guard pinned below their newest release. `null` when no version was resolved. Never the package names. |
| `scaffold.install_ms` | Schema 3. Wall-clock time of the install phase, `null` when none ran. |
| `scaffold.wizard_changed` | Schema 3. Wizard screen names whose answer differed from that screen's default, `null` without the wizard. Screen names only, from a closed set. |
| `run.*` | Counts and flags only. Never a task name, project name, or command. |
| `run.ms_saved` | Schema 3. The originally recorded run time of every task restored from cache, summed. |
| `run.verify_mismatches` | Schema 3. Tasks whose `--verify` run differed from their cache entry. A mismatch stops the run, so 0 or 1. |
| `graph.format` | Schema 3. `list` or `dot`. |
| `cache.action` | Schema 3. `stats` or `clear`. |
| `tools.*` | Schema 3, `culm new` only. The `major.minor` version of `rustc`, `cargo`, `node`, `pnpm`, and `go`, or `null` when absent or not answering within 200 ms. **Never the patch level, build hash, vendor suffix, or install path.** |

## What it is for

- **Which layers matter:** what gets maintained, and what gets cut when
  the template CI matrix grows too wide.
- **Where scaffolds break:** `error_kind`, `error_code`, and `install_failed_manager`
  say which stage and which package manager, without any error text.
- **Where people give up:** `cancelled_at`.
- **Whether the defaults are right:** `wizard_changed` names the screens
  where people did not keep the default.
- **Whether the release age guard costs anything:** `held_back` and
  `install_ms`.
- **Which toolchain versions to support:** `tools`.
- **Whether the cache is correct, from every user:** `verify_mismatches`
  is the r0.2 checkpoint's failure signal, and `ms_saved` is what the
  runner is worth.
- **Whether the runner earns its place:** `run` says whether anyone uses
  it and whether the cache hits, which the runner's kill criteria need.

It cannot say why someone stopped using culm. That comes from issues.

## Never sent

This list is binding.

- Repository names, directory names, or any filesystem path
- File contents of any kind
- Task names, project names, or command strings from `culm.toml`
- Error messages or package manager output (`error_code` names the
  error's type; its text, which carries paths and names, is never sent)
- Preset sources: no git URL, no file path, no saved preset name
- Environment variable names or values
- Git remotes, branch names, commit hashes, or author information
- IP addresses beyond what the transport unavoidably exposes, which is
  not stored
- Anything derived from hostname, MAC address, username, or serial number
- A tool's full version, build hash, vendor suffix, or path: `tools` is
  `major.minor` only, and only for `culm new`

`culm.toml` contains user-authored shell commands. Sending them "to
debug failures" would be the obvious next request, and it is refused here
in advance. `error_kind` and `error_code` are the answer to that request.

## The anonymous id

- UUID v4 from a cryptographic RNG. Not derived from any machine
  property, so it cannot be reversed to a machine.
- Written once to `culm/telemetry_id` in the platform's config directory
  (`$XDG_CONFIG_HOME` on Linux).
- Deleting that file produces a new, unlinkable id. Deleting it is
  supported, not a workaround.
- It identifies a machine, not a person.

## First-run notice

Printed to stderr once, before the first payload is sent, with the
payload itself:

```
culm sends anonymous usage data. This is the exact payload:
{ ...the JSON above, for this run... }
Full specification: https://github.com/Zunetra/culm/blob/main/docs/TELEMETRY.md
Opt out:            CULM_TELEMETRY=0
```

## When it is off

- `CULM_TELEMETRY` set to `0`, `false`, `off`, or `no`. This beats
  everything else.
- `DO_NOT_TRACK=1`.
- `CI` set to anything but empty, `false`, or `0`, unless `CULM_TELEMETRY`
  is `1`, `true`, `on`, or `yes`. CI runs would otherwise dominate the
  numbers and make them meaningless; the explicit yes is how a receiver
  is tested from a workflow.
- No config directory to keep the id in.

An empty or unrecognised `CULM_TELEMETRY` (such as `2`) is not a
decision, so the rules after it still apply.

## Transport

- Sent after the command completes, never blocking it for more than
  500ms in total. For `culm new`, toolchain detection runs first, all
  tools at once, and uses at most 200ms of that.
- Failure is silent and never retried. Telemetry must not be able to slow
  down or break a command.
- The receiver (`telemetry/worker/`) rejects any key not on this page,
  and any `error_code` or tool version outside the formats above.
- Stored events are read only by the owner, through a private dashboard
  (`telemetry/dashboard/`) behind Cloudflare Access.

## What this does and does not measure

**Does:** how many machines run culm, which layers matter, which
commands are used, where they fail, and which platforms need binaries.

**Does not:** how many humans, how many companies, or anything about
revenue. The number that matters commercially comes from accounts on the
hosted remote cache. See [CONTEXT.md](CONTEXT.md#decisions) decision 9.
