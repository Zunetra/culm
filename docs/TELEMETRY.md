# Telemetry

ipile sends anonymous usage data. This page is the complete specification,
not a summary. If something is not listed here, it is not sent.

**Opt out:** set `IPILE_TELEMETRY=0`, or run `ipile config set
telemetry false`. No feature is degraded when telemetry is off.

---

## Why this page exists

Every tool in this category collects usage data and describes it as
"anonymous usage data" in one vague sentence. Publishing the exact
payload and printing it on first run costs nothing and is a
differentiator. It also constrains us: the payload cannot quietly grow,
because growing it means editing this file.

## The exact payload

```json
{
  "schema": 1,
  "anonymous_id": "9f2c1e7a-4b3d-4a51-9c8e-2f0d6b7a1e33",
  "version": "0.1.0",
  "os": "linux",
  "arch": "x86_64",
  "command": "new",
  "duration_ms": 4210,
  "exit": "ok",
  "scaffold": {
    "shape": "monorepo",
    "preset": "polyglot-monorepo",
    "layers": ["nuxt", "axum", "prettier", "eslint", "rustfmt", "husky"],
    "runner": "nx"
  }
}
```

| Field | Meaning |
| --- | --- |
| `schema` | Payload version. Incremented when this document changes. |
| `anonymous_id` | Random UUID v4, generated once, stored in the config directory. |
| `version` | ipile's own version. |
| `os`, `arch` | Platform only. Not a kernel version, not a hostname. |
| `command` | The subcommand name alone. `new`, `run`, `graph`, `cache`. |
| `duration_ms` | Wall-clock time of the invocation. |
| `exit` | `ok`, `error`, or `cancelled`. Never an error message. |
| `scaffold` | Present only for `ipile new`. Which layers were chosen. |

The `scaffold` block is the only field with genuine product value. Which
layers people actually select decides what gets maintained and what gets
cut when the nightly matrix grows too wide.

## Never sent

This list is binding.

- Repository names, directory names, or any filesystem path
- File contents of any kind
- Task names, target names, or command strings from `ipile.yaml`
- Environment variable names or values
- Git remotes, branch names, commit hashes, or author information
- IP addresses beyond what the transport unavoidably exposes, which is
  not stored
- Anything derived from hostname, MAC address, username, or serial number

`ipile.yaml` contains user-authored shell commands. Sending them "to
debug failures" would be the obvious next request, and it is refused here
in advance so it does not get added quietly later.

## The anonymous id

- UUID v4 from a cryptographic RNG. Not derived from any machine
  property, so it cannot be reversed to a machine.
- Written once to `$XDG_CONFIG_HOME/ipile/id` or the platform
  equivalent.
- Deleting that file produces a new, unlinkable id. Deleting it is
  supported, not a workaround.
- It identifies a machine, not a person. Two developers sharing a laptop
  are one id. One developer on three machines is three.

## First-run notice

Printed once, before the first payload is ever sent:

```
ipile sends anonymous usage data: a random id, the version, your OS and
architecture, which subcommand ran, and which layers you selected. It
never sends paths, file contents, environment variables, or commands.

Full payload: https://ipile.dev/telemetry
Opt out:      IPILE_TELEMETRY=0
```

## Transport

- Sent after the command completes, never blocking it.
- Failure is silent and never retried. Telemetry must not be able to slow
  down or break a scaffold.
- Disabled automatically when `CI` is set, unless `IPILE_TELEMETRY=1` is
  explicit. CI runs would otherwise dominate the numbers and make them
  meaningless.

## What this does and does not measure

**Does:** how many machines run ipile, which layers matter, which
commands are used, and which platforms need binaries.

**Does not:** how many humans, how many companies, or anything about
revenue. Registry download counts are worse still: CI inflates them and
mirrors undercount them.

The number that matters commercially comes from accounts on the hosted
remote cache, which are exact and non-inferred. Telemetry measures the
free top of the funnel. Accounts measure the business. See
[DECISIONS.md](DECISIONS.md) decision 9.
