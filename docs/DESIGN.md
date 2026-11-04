# Design

How culm works and what is left to build. Replaces `01-scaffolder.md`,
`02-runner.md`, and `BENCHMARK.md`. Settled decisions and conventions
live in [CONTEXT.md](CONTEXT.md); this file cites them by number.

---

## 1. What culm is

One binary, two halves (decision 1).

- **The scaffolder** stands up a polyglot workspace: prettier, eslint,
  rustfmt, clippy, tsconfig paths, ignore files, hooks, CI, and a
  workspace manifest, all composed from layers. Runs once per project.
  A bad scaffold is noticed in ten seconds.
- **The runner** runs a workspace's tasks in dependency order and caches
  them by content. Runs hundreds of times a day. A wrong cache hit ships
  broken code and is trusted while doing it.

The scaffolder ships first and is the acquisition channel; the runner is
what retains (decision 2). The scaffolder never emits configuration only
culm's runner can consume (decision 3).

**The gap.** `create-nx-workspace` will not produce a Cargo workspace,
and `cargo-generate` will not produce the JavaScript half. culm treats
the polyglot case as the main case.

---

## 2. The scaffolder

### 2.1 Layers

A layer is `layers/<category>/<id>/`: a `layer.toml` plus the files only
that layer owns. **No combination of layers is ever authored by hand**
(decision 4). Identity is always the qualified `<category>/<id>`
(decision 18).

| Category | Cardinality | Members |
| --- | --- | --- |
| base | exactly one | `base` |
| shape | exactly one | `single-app`, `monorepo`, `tooling-only` |
| frontend | 0-1 single-app, any monorepo, none tooling-only | `nuxt`, `next`, `react-vite`, `vue-vite`, `astro`, `svelte` |
| backend | as frontend | `axum`, `actix`, `express`, `fastify`, `nestjs`, `go-chi` |
| data | as frontend | `sqlx`, `prisma`, `drizzle` |
| toolchain | implied; one or more under tooling-only | `node-pnpm`, `rust-cargo`, `go-mod` |
| quality | zero or more, defaults on | `prettier`, `eslint`, `rustfmt`, `clippy`, `husky`, `commitlint`, `editorconfig` |
| runner | exactly one, default `none` | `nx`, `turborepo`, `moon`, `culm`, `none` |
| infra | zero or more | `docker-compose`, `dockerfile`, `github-actions` |
| editor | zero or more | `vscode`, `zed` |

Cardinality lives on the category in `src/layer/category.rs`, never in
pairwise `conflicts`. The user never picks a language: `axum` implies
Rust, which implies `rust-cargo`, `rustfmt`, and the matching hook.

### 2.2 The manifest

```toml
id       = "husky"
category = "quality"
requires = ["toolchain/node-pnpm"]
default  = true

[[files]]           # owned by this layer alone, {{var}} interpolated
from = "files/.husky/pre-commit"
to   = ".husky/pre-commit"
mode = 0o755

[[merge]]           # a file other layers also touch
target   = "package.json"
strategy = "json-deep"
[merge.value]
scripts = { prepare = "husky" }

[[install]]         # run after writing; no versions, ever (decision 5)
manager  = "pnpm"
dev      = true
packages = ["husky", "lint-staged"]
```

- A file another layer might touch is merged into, never copied.
- `order` is explicit wherever sequence is observable (hook bodies, CI
  steps).
- An app layer never hardcodes its own depth: it uses the layout
  variables in `src/scaffold/layout.rs` (`{{app_dir}}`, `{{to_root}}`,
  `{{cd_app_dir}}`, and the rest).

### 2.3 The merge engine

The actual product. Everything else copies files.

| Strategy | Used for |
| --- | --- |
| `json-deep` | `package.json`, tsconfig: objects merge, arrays union and sort, differing scalars are an error |
| `toml-members` | `Cargo.toml` `[workspace].members` |
| `yaml-list` | `pnpm-workspace.yaml` globs |
| `line-block` | `.gitignore`, `.env.example`: one block per layer, sorted by id |
| `hook-script` | `.husky/*`: shebang, then bodies by `order` |
| `yaml-steps` | CI workflows: steps by `order` within a job |

Rules:

- **Order independence.** Any permutation of a layer set renders
  byte-identical output. Proven by proptest.
- **Collisions are errors naming both layers**, never last-write-wins.
- **All or nothing.** The whole file set is built and validated in
  memory before anything touches disk.
- **Sorted output everywhere**, which is what makes the above testable.

### 2.4 The wizard

`culm new <name>` with no `--layer` or `--preset`, on a real terminal
only. Order: shape, frontend, backend, data, toolchain (tooling-only
only), quality, infra, editor, runner. Data offers only the layers whose
toolchain an app already chosen brings, and is skipped when none fits. Two rules: every question has a flag,
and the resolved plan is printed and confirmed before anything is
written (`--yes` skips it).

### 2.5 Presets

A preset is a name, a description, and a list of qualified layer ids, in
TOML (decision 20). Sources, in lookup order: first-party (`monorepo`,
`frontend`, `backend`, compiled in), a local `./file.toml`, a git URL
(`github:user/repo[/path.toml][@ref]`). `--save-preset NAME` writes the
fully resolved run back out. `--layer` given with `--preset` extends
the preset; a layer it already names is not repeated, and a clash (a
second shape) is a selection error. No hosted registry before v2
(decision 6).

### 2.6 Version resolution

Decision 5 as behaviour:

1. Layer files contain no version numbers.
2. After writing, `pnpm add`, `cargo add`, and `go get` write versions
   and lockfiles.
3. Where a config shape is bound to a major, the package spec carries a
   range inline, for example `typescript@<6`. Every such cap is listed in
   `tests/layers.rs` with its reason; any other version fails the test.
4. "Newest" excludes releases younger than N days (`--min-release-age`,
   default 3, `0` turns it off):
   - **pnpm** enforces it itself: `toolchain/node-pnpm` writes
     `minimumReleaseAge` (minutes) into `pnpm-workspace.yaml` for every JS
     workspace, which covers every dependency, transitive ones included.
   - **Cargo and Go** have no such setting, so culm asks crates.io and the
     Go proxy and names the version (`src/install/release_age.rs`). Cargo
     also gets a `cargo update --precise`, since the lockfile would
     otherwise resolve the newest compatible release anyway. Direct
     dependencies only.
   - A registry that cannot be reached fails the install loudly; it never
     silently skips the guard.

### 2.7 Template CI

`templates.yml` runs weekly (frameworks change shape once or twice a
year) and by hand as the gate for any new or re-harvested layer:

- every layer that emits an application, installed, built, linted, and
  tested against the live registries;
- each first-party preset on Linux and macOS;
- a two-app monorepo built through each runner backend's own tool (`nx`,
  `turbo`, `moon`, `culm`), decision 19's bar for the backends.

A red run means re-harvest that layer by hand. Nothing automated rewrites
a template.

### 2.8 Kill criteria

- **A generated workspace fails to build and CI did not catch it before
  a user did.** The layer matrix is too wide. Cut layers until CI is
  trustworthy again.
- **The merge engine needs a special case for a specific pair of
  layers.** Either the strategy set is wrong, or that layer should not
  exist.
- **Nobody uses it.** If telemetry after three months public shows
  scaffolds only from our own machines, the runner has no audience
  either. Stop.
- **Writing a layer takes longer than writing the config by hand.**

---

## 3. The runner

### 3.1 The problem

A polyglot repository has two dependency graphs and two lockfiles, and
every existing runner treats one as real and the other as a plugin. The
failure that matters most is **a wrong cache hit**: it ships a broken
build that every other developer and CI run then trusts.

Nx, Turborepo, Bazel, Moon, and Pants all exist; Moon is the closest and
should be read in full before any new runner design. **This may be the
half without a real gap.** If there is a wedge, it is narrow: cache-key
correctness across `Cargo.lock`, `pnpm-lock.yaml`, sources, and
environment inputs at once. Not "a better Nx".

### 3.2 Interface

```
culm run <task> [--parallel]     a task and its dependency closure
culm run --affected <REF>        tasks touched since REF, plus downstream
culm run <task> --no-cache       run for real, read and write nothing
culm run <task> --verify         run for real, fail if output differs from the cache
culm cache stats|clear           the local store
culm graph --format list|dot     the task graph
```

```toml
# culm.toml, one per project (decision 22)
[tasks.build]
command = "cargo build --release"
inputs = ["src/**/*.rs", "Cargo.toml", "Cargo.lock"]
outputs = ["target/release/backend"]
cache = true

[tasks.test]
command = "cargo nextest run"
inputs = ["src/**/*.rs", "tests/**/*.rs", "Cargo.toml"]
depends_on = ["build"]
cache = true
declared_env = ["RUSTFLAGS"]
```

`persistent = true` marks a task that never exits on its own, such as a
dev server: `--affected` never selects it, and naming it still runs it.
`culm new` writes it on every generated `dev`.

culm does not know what `cargo` or `vite` is: a command, input globs,
output paths. Parsing only ever gains fields, so a `culm.toml` written
today is never a migration later.

**Workspaces.** A root `culm.toml` with a `[workspace]` table makes every
other `culm.toml` under it (honouring `.gitignore`) a project, named by
its path from the root. Tasks become `<project>:<task>` and run in their
own project directory.

```toml
# apps/web/culm.toml
[project]
dependencies = ["libs/ui"]

[tasks.build]
command = "pnpm run build"
inputs = ["src/**/*", "package.json", "../../pnpm-lock.yaml"]
depends_on = ["^build"]   # build in libs/ui; also "codegen" or "libs/ui:build"
```

At the root, `culm run build` runs `build` in every project that has one;
inside a project, only that project's. Without `[workspace]`, one
`culm.toml` is the whole run and names stay bare.

### 3.3 The cache key

For each task: hash inputs by content (never mtime), check local then
remote cache, restore on a hit, run and record on a miss.

**The key includes everything, or it is a bug waiting to happen:** the
task's name, the command, every input file's content hash, the values of
`declared_env` only (never the whole environment, and unset is not
empty), each dependency's own key, the platform, and culm's own version.
Too broad costs a miss, which is merely disappointing. Too narrow serves
stale output, which is dangerous. `CacheKey::digest` only ever gains
inputs, each one a reviewed decision.

Rules that follow from it (`src/runner/cache.rs`):

- A task with no `inputs` is never cached, and neither is one whose
  dependency has no key: nothing vouches for what they read.
- A key is computed just before its task runs, since inputs are often a
  dependency's outputs.
- Input globs honour `.gitignore`; a literal path is always read. Output
  paths must stay inside the project, since they are deleted before a
  restore, which is what keeps a stale file from surviving a hit.
- Only a successful run is recorded. A restore verifies every object
  before touching the workspace, and restores executable bits.

### 3.4 `cas`

A content-addressable store (decision 11): bytes stored under their own
blake3 hash, so identical content is stored once and a corrupted object
is detected on read. A `CacheEntry` maps each output path, stdout, and
stderr to a digest, plus the exit code and duration, and is looked up by
the key's digest. Lives at the OS cache dir under `culm/cas`.

### 3.5 Milestones

| | Scope |
| --- | --- |
| r0.1 | Sequential run respecting `depends_on`, no cache |
| r0.2 | Content hashing, local cache, hit/miss reporting. **The checkpoint:** two weeks of real use, every hit checked by hand against a fresh build, before anything else is written |
| r0.3 | `--affected` via git diff, parallel execution by dependency level |
| r0.4 | Remote cache over plain HTTP, so CI and every developer share hits |
| r1.0 | Platform binaries, `graph --format dot`, the section 4 benchmark |

### 3.6 Correctness testing

A `cache_correctness` suite runs fixture workspaces twice and asserts the
second run's output bytes equal the first's. It never trusts culm's own
"cache hit" line. Single-threaded, since it drives real subprocesses and
filesystem state.

### 3.7 Kill criteria

Preserved unchanged from the original runner document. Do not soften
these.

- **r0.2's honest two-week check finds even one incorrect cache hit** a
  human would not have caught without deliberately looking for it. A
  build cache that is wrong sometimes is worse than no cache, and this is
  where that principle gets tested against reality rather than left as a
  nice sentence.
- **Moon or Pants closes the gap** named in section 2 with real adoption
  before this reaches r0.3. They have more resources and more runway on
  exactly this thesis; the honest move is switching rather than
  competing.
- **Nx's non-JS support improves enough** that the daily pain motivating
  this stops being felt on the Klassivox workspace. If the incumbent
  stops hurting, the dogfooding premise evaporates, because there is no
  more pain to be an expert in.
- **The r1.0 benchmark shows no meaningful win over Nx** on the real
  Klassivox workspace, not a synthetic one. "Roughly the same speed, but
  self-built" is not a reason for anyone else to adopt it.
- **Any point before r0.2 where writing `culm.toml` feels like more work
  than the Nx configuration already in place.** If the replacement is not
  obviously less annoying than the incumbent on day one, on the one
  workspace with perfect information, it will not be less annoying
  anywhere else.

The default expectation going in should be that this half **might stop at
r0.2**, and that stopping there, having learned graph algorithms and
content hashing and confirmed the market is too crowded to win, is
success rather than failure.

("Section 2" above is section 3.1 in this file.)

---

## 4. Benchmark method

Against Nx on a copy of the real Klassivox workspace, never a synthetic
one, and never by modifying Klassivox itself (decision 13). Needs
multi-project `culm run` first (section 5).

**Workloads**, each for both tools:

1. **Cold:** no cache, fresh clone, build every project.
2. **Warm, no changes:** every task should hit. Output bytes are diffed
   against the cold run before the timing counts.
3. **Warm, one file changed** in `packages/klassivox-ui`, a dependency of
   several apps: downstream reruns, unrelated tasks do not.

**Method:** five runs per workload, drop the first, report the median of
the other four.

**Reported:** wall clock, cache hit rate, byte-for-byte output match per
task (any mismatch disqualifies, whatever the speed), and peak memory of
the orchestrating process.

**Not claimed:** anything general about culm versus Nx, or about Nx's
ceiling. No number is published without the full method, including the
byte check.

**Status:** not measured.

---

## 5. Status

### Built

**Scaffolder**

- The layer format, catalog, selection validation, and all six merge
  strategies (`yaml-list` also carries integer and boolean settings), with
  the order independence proptest.
- 39 layers across all ten categories, every application layer verified
  by a real install and build in both shapes.
- `culm new`: flags, wizard (with a data screen filtered by toolchain),
  presets (first-party, file, git URL) extendable with `--layer`,
  `--save-preset`, installs for pnpm, cargo, and go, and the release age
  guard (section 2.6).
- Runner backend config for Nx, Turborepo, Moon, and culm, written by
  `culm new`.
- Telemetry, schema 2, Rust side and receiver code (decision 9 amended
  2026-09-26; docs/TELEMETRY.md).

**Runner**

- r0.1, r0.2 in code, r0.3: ordered runs, the content-addressed cache
  with `--no-cache` and `--verify` (its summary reports verified tasks,
  not cache hits), `--affected`, `--parallel`, and `persistent` tasks
  that `--affected` never starts.
- Multi-project workspaces (section 3.2).
- `culm graph`, `culm cache`, and a remote cache client (not wired in).
- `tests/cache_correctness.rs`, which checks restored bytes rather than
  trusting a "cache hit" line.

**Around both**

- CI: `ci.yml` (fmt, clippy, tests, `cargo deny`, `cargo audit`,
  `cargo publish --dry-run`, macOS and Windows), `templates.yml`
  (section 2.7), `release.yml` (five binaries, `SHA256SUMS`,
  `installer.sh`, and by-hand publishing to npm, crates.io, and the
  Homebrew tap).
- Distribution code for all four channels of handbook 0004: `installer.sh`,
  `npm/` (launcher and platform packages), `packaging/homebrew/`, and the
  crate metadata.

### Left: code

**Scaffolder**

- JVM, then C/C++ layers (decision 7). Large; not scheduled.
  `src/install/maven_central.rs` is the start of the JVM install path.

**Runner, held for the r0.2 checkpoint (section 3.5)**

Decision 2 says the checkpoint does not move, so these wait until two
weeks of real use find no wrong hit:

- Remote cache wiring: local, then remote, then run; upload after
  recording; an `/entries/<key>` endpoint on the client. Optionally a
  self-hostable `culm cache serve`.
- The benchmark harness (section 4): copy Klassivox to a temporary
  directory, add `culm.toml` files kept in this repository, run the
  method.

### Left: owner

Needs accounts, secrets, or time rather than code.

- **The r0.2 checkpoint:** two weeks of real use with `--verify` runs,
  every hit checked. Decides whether the held runner work happens at all.
- **Telemetry:** the receiver is deployed (2026-11-04) at
  `culm-telemetry.shownzy001.workers.dev`, and `DEFAULT_ENDPOINT` points
  at it from 0.1.1. Optional later: serve it at `telemetry.culm.dev`
  once that domain is on the account (docs/DEPLOY.md).
- **Registries:** the `culm` crate is published (0.1.0). Still open:
  claim `culm` and `@zunetra` on npm, add `NPM_TOKEN`; create
  `zunetra/homebrew-tap` and `HOMEBREW_TAP_TOKEN`; add
  `CARGO_REGISTRY_TOKEN` if crates.io publishing should go through
  `release.yml`.
- **Release:** no tag exists yet. Tag v0.1.1 so `release.yml` builds the
  binaries the shell installer and Homebrew need, then publish 0.1.1.
- **Benchmark:** run and publish it, once the harness exists.
- **Docs site:** `TELEMETRY.md` currently points at GitHub.
- **Klassivox as a published preset** (decision 17), paused by choice.
