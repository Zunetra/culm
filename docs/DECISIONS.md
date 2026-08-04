# Decisions

Settled decisions for ipile, with the reasoning that produced them. This
file exists so nothing here gets re-litigated by accident six months from
now.

**Format:** each decision states what was decided, why, and what would
have to change for it to be reopened. If a decision is reopened, edit it
in place and say what changed. Do not delete history.

**Status legend:** `LOCKED` is settled. `OPEN` still needs a call and is
listed at the bottom.

---

## 1. ipile is one tool with two halves `LOCKED`

A scaffolder and a task runner, shipped as one binary, not two products.

They are genuinely different in every commercial dimension:

| | The runner | The scaffolder |
| --- | --- | --- |
| Job | know what changed, skip what it can, cache correctly | stand up a workspace so nobody hand-wires tooling again |
| Runs | every day, hundreds of times | once per project, ever |
| Failure mode | silent wrong cache hit, ships broken code | generated project does not build, noticed in 10 seconds |
| Market | Nx, Turborepo, Moon, Bazel, Pants, Buck2 | create-nx-workspace, create-t3-app, cargo-generate, degit |
| Moat | high, if correctness is real | near zero, anyone can copy a template directory |
| Retention | high, it is in the inner loop | zero |
| Monetizable | yes, hosted cache | no |

They ship as one tool anyway because the scaffolder is the acquisition
channel and the runner is what retains. Almost nobody adopts Nx by
reading about task graphs. They run `create-nx-workspace`, like the
result, and are on Nx forever.

**Reopen if:** the scaffolder finds an audience and the runner is killed
at its v0.2 checkpoint. At that point ipile is a scaffolder, and the docs
should say so plainly rather than keep a dead half alive.

## 2. The scaffolder ships first `LOCKED`

This inverts the original ordering, where the runner was the whole
project and was ranked last of five for being the riskiest thing on the
list.

- The scaffolder is roughly 3 to 4 weeks, not 12.
- It has no catastrophic failure mode. A bad scaffold is noticed
  instantly. A bad cache hit ships broken code and is trusted while doing
  it.
- It produces real signal cheaply. If nobody runs `ipile new`, the runner
  has no audience either, and that was learned in a month instead of a
  quarter.

The runner's v0.2 checkpoint stays exactly where it is. The scaffolder
moves in front of it, it does not replace it.

## 3. The runner is a layer, and Nx is a valid answer `LOCKED`

**This is a hard constraint on the code, not a preference.** The
scaffolder must be able to emit Nx, Turborepo, or Moon configuration, and
`none` must be valid. Our own runner is one option among them, and it is
not the default until it has passed its own v0.2 correctness check.

Why it is non-negotiable:

- The scaffolder stays useful on day one even if the runner never ships.
- Nobody has to bet on unproven cache correctness to get value.
- When the runner is ready, migration is a one-line config change in a
  tool people already have installed.

If the scaffolder ever emits configuration that only our runner can
consume, the entire argument for building the scaffolder first collapses.

## 4. The unit is a layer, not a template `LOCKED`

A layer is a directory of files plus a manifest describing how it merges
into shared files. Composition happens at scaffold time. **No combination
is ever authored by hand.**

- **Shape** (pick one): monorepo, single-app
- **Frontend apps** (zero or more): nuxt, next, react-vite, vue-vite,
  astro, svelte
- **Backend apps** (zero or more): axum, actix, express, nestjs, fastify,
  go-chi
- **Toolchain** (implied, never asked): node and pnpm follow any JS app,
  rust and cargo follow any Rust app
- **Quality** (defaults on, individually toggleable): prettier, eslint,
  rustfmt, clippy, husky and lint-staged, commitlint, editorconfig
- **Runner** (pick one): nx, turborepo, moon, ipile, none
- **Infra**: docker-compose with postgres and redis, Dockerfile, GitHub
  Actions
- **Editor**: vscode, zed

Consequences that matter:

- **Organised by role, and the language falls out of the choice.** Picking
  `axum` implies Rust, implies `Cargo.toml`, implies `rustfmt`, implies
  `cargo fmt --check` in the pre-commit hook. The user never answers a
  "which language" question, because the framework choice already
  contains the answer.
- Frontend-only is `shape=single-app + nuxt`. Backend-only is
  `shape=single-app + axum`. A full polyglot monorepo is
  `shape=monorepo + nuxt + axum + docker + ci`. Same engine, no extra
  authoring.
- Roughly 35 to 40 layers covers an enormous combination space.

**The real work is the merge engine**, not the layers: patching
`package.json`, `Cargo.toml`'s members array, `pnpm-workspace.yaml`
globs, `tsconfig.base.json` paths, `.gitignore`, husky hook bodies, and
CI yaml, from N independent layers, deterministically, in any order.
That is the hard part, it is the thing nobody has built well, and it is
the only part worth being proud of. Everything else is copying files.

Full specification in [01-scaffolder.md](01-scaffolder.md).

## 5. Template files never contain version numbers `LOCKED`

Pinned versions are the single biggest cause of scaffolders rotting.

- No version numbers in template files. After scaffolding, shell out to
  `pnpm add -D prettier` and `cargo add axum` and let the package
  managers write the version and the lockfile.
- **Ranged only where the config shape is coupled to a major.** A layer
  shipping an eslint flat config is bound to eslint 9, so that layer
  declares `compat: ">=9 <10"` and resolves the latest within the range.
  Unranged everywhere else.
- **"Latest stable" means "latest that has been public for N days."**
  Klassivox already sets `minimumReleaseAge` in its
  `pnpm-workspace.yaml` as a supply-chain guard, and installing
  genuinely-latest defeats it. Apply the same rule to resolution and ship
  it as a scaffolder default.
- **Nightly CI is what makes this safe.** Scaffold the preset matrix,
  install, build, lint, test. Its job is to catch the day `latest` breaks
  a layer.

No separate version registry is needed. npm and crates.io already are
that registry. The tool simply refuses to hardcode what they say.

## 6. Presets are first-class from v0.1, distributed by git URL `LOCKED`

A preset is a small file: a name, a list of layers, and pre-answered
wizard questions.

- **First-party presets shipped in the binary:** `monorepo` (polyglot),
  `frontend`, `backend`.
- **The `monorepo` preset is derived from Klassivox**, which is the
  proving fixture. Nothing public is named Klassivox.
- **User presets work from day one** via
  `ipile new myapp --preset github:olorunshogo/my-preset`. Publishing is
  pushing a git repository.
- **After any scaffold, offer to save the run as a local preset.** This
  is what converts a once-per-project tool into something with a reason
  to exist twice.
- **A hosted community registry is v2 or later, deliberately.** Hosting
  other people's presets means owning a code-execution supply chain,
  because presets run install commands. That means moderation, takedowns,
  and a security posture. Git URLs test the demand for free first.

## 7. JS/TS and Rust first, then Go, then JVM, then C/C++ `LOCKED`

Not caution, sequencing. **JS and Rust are the hardest pair.** JS has a
package manager that writes a lockfile and hoists a `node_modules` tree.
Rust has a workspace manifest with a members array and a separate
lockfile. If the merge engine handles `package.json` and
`Cargo.toml [workspace].members` and keeps `pnpm-workspace.yaml` globs
consistent all at once, then Go's `go.work`, Gradle settings, and CMake
subdirectories are the same shape with less friction.

C and C++ last, because CMake has no single blessed convention and that
alone can consume a month.

## 8. The Rust binary is the product, npm is the front door `LOCKED`

Nobody runs `cargo install` to scaffold a Vue app. Nobody runs `npx` for
a build tool they invoke 200 times a day. So both, with npm as the
primary funnel.

**The npm package ships prebuilt binaries as platform-specific
`optionalDependencies` with `os` and `cpu` fields**, the pattern esbuild,
Biome, `@swc/core`, and Rspack use. The package manager installs exactly
one.

**Not a postinstall download script.** Those break under
`--ignore-scripts`, offline CI, corporate proxies, and sandboxed
environments.

## 9. Telemetry is opt-out, and the payload is published verbatim `LOCKED`

Registry download counts are a trend line, not a user count. CI inflates
them, mirrors undercount them, and neither answers "how many people."

- Anonymous random UUID per machine, generated once. Never derived from
  hostname, MAC address, or repository path.
- Sent: anonymous id, tool version, OS and architecture, command name,
  and for the scaffolder, which layers and preset were selected. That
  last field is the genuinely valuable product data.
- **Never sent:** repository names, file paths, task names, environment
  variable values, command strings, git remotes. Note that `ipile.yaml`
  contains user commands, so the temptation to send them "for debugging"
  is real and is refused here in advance.
- **The exact JSON payload is published in the docs and printed on first
  run**, not summarised as "anonymous usage data." Every other tool in
  this category is quietly cagey about this. Being the one that is not is
  cheap and is a differentiator.
- Opt out with `IPILE_TELEMETRY=0` or a config key.

The number that actually matters commercially comes from accounts on the
hosted remote cache, not telemetry. Telemetry measures the free top of
the funnel. Accounts measure the business.

Full payload in [TELEMETRY.md](TELEMETRY.md).

## 10. The name is ipile, and culm is reserved `LOCKED`

"Grain" was abandoned because Grain is an existing programming language
at grain-lang.org, and `create-grain` was live on npm in an adjacent
category.

`ipile` is free on npm, `create-ipile`, crates.io, and `ipile.dev`, with
no product collision. `culm` is equally clean and is held for a future
product. Reserved names are recorded in
[../../handbook/naming/RESERVED.md](../../handbook/naming/RESERVED.md).

Full sweep and the eliminated candidates in [NAMING.md](NAMING.md).

## 11. One repository per project, abstract later `LOCKED`

This **replaces** the earlier plan in Klassivox's
`docs/Projects/README.md`, which put all five Rust projects in a single
`rust-infra` Cargo workspace sharing a `crates/cas` folder.

Each project gets its own repository. Each has different needs. Shared
code gets extracted when there is a second real consumer, not before.

**What `cas` is,** since the earlier docs assume it without saying:
a *content-addressable store*. It hashes a file's bytes, stores the file
under that hash as its name, and hands the hash back. Ask for the hash
later and you get the exact bytes back, or an error if they are missing
or corrupted. Two identical files stored twice occupy one slot, because
identical bytes produce an identical hash.

It matters to the runner because a build cache is exactly that shape.
"Given these inputs, here are the output bytes" is a lookup keyed by a
hash of the inputs.

**Decision:** ipile builds `crates/cas` internally, unpublished. If
project 01's job queue later needs the same thing, extract it then, with
two real consumers shaping the API instead of one imagined one. This
costs nothing now, because `cas` is not needed until the runner's v0.2,
which is months away.

## 12. The organisation is Zunetra `LOCKED`

Zunetra sits above both Klassivox (school platform) and ipile (developer
tooling), which share nothing. The org name is therefore deliberately
sector-neutral.

`culm` was explicitly rejected as an org name, because it is held as a
future *product* name and using it for the org would spend it for
nothing. `Nomia` was the first choice and was already taken.

Org-level decisions live in the `handbook` repository, not here.

## 13. Klassivox is not modified `LOCKED`

Klassivox is the proving fixture and the first customer. Its
`docs/Projects/05-task-runner.md` stays exactly as written. Whether it
eventually becomes a stub pointing here is a separate call that has not
been made.

`docs/jottings/` in Klassivox remains read-only under all circumstances.

## 14. Licence is Apache-2.0 `LOCKED`

Permissive, with the patent grant. Matches the open-core doctrine: the
binary, the config format, and single-machine operation are free forever.
Hosted remote cache, distributed execution, cache analytics, and SSO are
the paid line. Never charge for the cache being correct.

## 15. ASCII everywhere a machine reads it `LOCKED`

The written brand is **Ìpìlẹ̀**. Everything else is plain ASCII `ipile`:
the folder, the repository, the binary, the npm package, the crate, the
domain, environment variables, and every identifier in the source.

Diacritics break shell paths, git remotes, URLs, and package names, and
nobody types accents at a terminal. Never introduce one into a path, an
identifier, a package name, or a command.

The tradeoff accepted: pronunciation is not self-evident to a global
audience. It is stated in the first line of the README, which is what Nx
and pnpm do with their own names.

## 16. ipile is a sibling repository under `Projects/Zunetra` `LOCKED`

```
Projects/Zunetra/
  Klassivox/      proving fixture and first customer
  ipile/          this repository
  handbook/       org decisions and findings
```

The parent folder is a container, not a repository. Each child is an
independent git repository, per decision 11.

Recorded in full, including why the org repository is named `handbook`
rather than `zunetra`, in
[../../handbook/decisions/0003-workspace-layout.md](../../handbook/decisions/0003-workspace-layout.md).

## 17. Klassivox's configuration becomes the first published preset `LOCKED`

Two distinct things, easy to conflate:

- The first-party `monorepo` preset **shipped in the binary** is derived
  from Klassivox's real configuration. Nothing shipped is named
  Klassivox.
- Separately, the Klassivox setup is published as **the first
  user-authored preset**, distributed by git URL. It is the proof that
  the preset mechanism works for someone who is not us, tested by being
  the first person who is not us to use it.

The second is what validates decision 6. A preset system nobody outside
the binary ever uses is a configuration file with extra steps.

---

## Open

Decisions not yet made. Do not treat silence here as agreement.

1. **Whether the scaffolder's first-party layer set includes Vue and
   React on day one, or only one of them.** Fewer layers means a nightly
   CI matrix that actually stays green.
2. **Where the hosted remote cache runs, and what it costs**, which does
   not matter until the runner reaches v0.4.
3. **Whether `ipile.yaml` is the config filename.** It should probably be
   settled before any file format is written, since renaming a config
   file after release is a migration.
4. **Whether Klassivox's `docs/Projects/05-task-runner.md` becomes a stub
   pointing here** once these docs are complete.
