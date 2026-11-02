# Telemetry

Telemetry is **off by default**. When you opt in, culm sends anonymous
usage data. This page is the complete specification,
not a summary. If something is not listed here, it is not sent. The code
that builds the payload is `src/telemetry/payload.rs`, and a test there
fails if a key is added without this page changing too.

**Opt in:** set `CULM_TELEMETRY=1` (or `true`, `on`, `yes`). Unset it, or
set any other value, to turn it off again. No feature is degraded when
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
does not apply.

```json
{
  "schema": 2,
  "id": "9f2c1e7a-4b3d-4a51-9c8e-2f0d6b7a1e33",
  "version": "0.1.0",
  "os": "linux",
  "arch": "x86_64",
  "command": "new",
  "exit": "ok",
  "error_kind": null,
  "duration_ms": 4210,
  "scaffold": {
    "shape": "monorepo",
    "layers": ["base/base", "shape/monorepo", "frontend/nuxt", "backend/axum"],
    "selected_via": "wizard",
    "preset": null,
    "install": "ok",
    "install_failed_manager": null,
    "saved_preset": false,
    "cancelled_at": null
  },
  "run": null
}
```

For `culm run`, `scaffold` is `null` and `run` is:

```json
{
  "projects": 3,
  "tasks": 7,
  "cacheable": 5,
  "hits": 4,
  "misses": 1,
  "parallel": true,
  "affected": false,
  "remote_cache": false
}
```

| Field | Meaning |
| --- | --- |
| `schema` | Payload version. Incremented when this page changes. |
| `id` | Random UUID v4, generated once per machine. |
| `version` | culm's own version. |
| `os`, `arch` | Platform only. Not a kernel version, not a hostname. |
| `command` | The subcommand name alone: `new`, `run`, `cache`, `graph`. |
| `exit` | `ok`, `error`, or `cancelled`. |
| `error_kind` | On `error` only, the stage that failed, from a closed set: `select`, `preset_fetch`, `resolve`, `merge`, `write`, `install`, `version_resolve`, `config`, `graph`, `task_failed`, `cache`, `other`. **Never an error message.** |
| `duration_ms` | Wall-clock time of the invocation. |
| `scaffold.shape` | `single-app`, `monorepo`, or `tooling-only`. |
| `scaffold.layers` | The resolved layer ids, defaults included. **Only layers that ship in culm**; a layer from `--layers-dir` or a typo is dropped. |
| `scaffold.selected_via` | `flags`, `preset`, or `wizard`. |
| `scaffold.preset` | A first-party preset's name, otherwise only `file`, `git`, or `unknown`. **Never the path or URL.** |
| `scaffold.install` | `ok`, `failed`, `skipped` (`--no-install`), or `none` (nothing to install). |
| `scaffold.install_failed_manager` | `pnpm`, `cargo`, or `go`. Never its arguments. |
| `scaffold.saved_preset` | Whether `--save-preset` was used. Never its name. |
| `scaffold.cancelled_at` | The wizard screen (`shape`, `frontend`, ...) or `confirm` where the user backed out. |
| `run.*` | Counts and flags only. Never a task name, project name, or command. |

## What it is for

- **Which layers matter:** what gets maintained, and what gets cut when
  the template CI matrix grows too wide.
- **Where scaffolds break:** `error_kind` and `install_failed_manager`
  say which stage and which package manager, without any error text.
- **Where people give up:** `cancelled_at`.
- **Whether the runner earns its place:** `run` says whether anyone uses
  it and whether the cache hits, which the runner's kill criteria need.

It cannot say why someone stopped using culm. That comes from issues.

## Never sent

This list is binding.

- Repository names, directory names, or any filesystem path
- File contents of any kind
- Task names, project names, or command strings from `culm.toml`
- Error messages or package manager output
- Preset sources: no git URL, no file path, no saved preset name
- Environment variable names or values
- Git remotes, branch names, commit hashes, or author information
- IP addresses beyond what the transport unavoidably exposes, which is
  not stored
- Anything derived from hostname, MAC address, username, or serial number

`culm.toml` contains user-authored shell commands. Sending them "to
debug failures" would be the obvious next request, and it is refused here
in advance. `error_kind` is the answer to that request.

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

## When it is on

Only when both of these hold:

- `CULM_TELEMETRY` is set to `1`, `true`, `on`, or `yes`. Unset, empty,
  or any other value leaves it off. An explicit opt-in holds even in CI,
  which is how a receiver is tested from a workflow.
- There is a config directory to keep the id in.

`DO_NOT_TRACK` and `CI` need no rule of their own: without an explicit
opt-in, telemetry is already off.

## Transport

- Sent after the command completes, never blocking it for more than
  500ms.
- Failure is silent and never retried. Telemetry must not be able to slow
  down or break a command.
- The receiver (`telemetry/worker/`) rejects any key not on this page.

## What this does and does not measure

**Does:** how many machines run culm, which layers matter, which
commands are used, where they fail, and which platforms need binaries.

**Does not:** how many humans, how many companies, or anything about
revenue. The number that matters commercially comes from accounts on the
hosted remote cache. See [CONTEXT.md](CONTEXT.md#decisions) decision 9.
