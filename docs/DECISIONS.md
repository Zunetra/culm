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

**Decision:** ipile builds `cas` internally as a module, unpublished. If
project 01's job queue later needs the same thing, extract it to its own
crate then, with two real consumers shaping the API instead of one
imagined one. This costs nothing now, because `cas` is not needed until
the runner's v0.2, which is months away.

**Clarified 2026-08-04.** This previously said ipile "builds `crates/cas`
internally", which reads as a directory layout mandate and was taken as
one. It is not. ipile is a single crate with one `src/`, no workspace and
no `crates/` directory, per the same abstract-later reasoning above.
Splitting a module into a crate later is mechanical. Un-splitting a
premature workspace is not.

## 12. The organisation is Zunetra `LOCKED`

Zunetra sits above both Klassivox (school platform) and ipile (developer
tooling), which share nothing. The org name is therefore deliberately
sector-neutral.

`culm` was explicitly rejected as an org name, because it is held as a
future *product* name and using it for the org would spend it for
nothing. `Nomia` was the first choice and was already taken.

Org-level decisions live in the `handbook` repository, not here.

## 13. Klassivox is not modified unless explicitly instructed `LOCKED`

Klassivox is the proving fixture and the first customer. It lives in a
separate repository, and work in this one does not reach into it. No
amount of "the docs are now out of sync" justifies an edit there.

The exception is the owner explicitly asking for a change in Klassivox,
naming it. That instruction is authorisation, and it is the only thing
that is. A generic instruction that merely sounds repository-wide, such
as "fix the docs" or "clean this up everywhere", is not.

Whether `docs/Projects/05-task-runner.md` eventually becomes a stub
pointing here is a separate call that has not been made.

`docs/jottings/` in Klassivox is stricter still. Only an explicit
instruction naming that specific file authorises a change to it.

**Reworded 2026-08-04.** This previously read as an absolute ban, "not
modified for any reason". That was overtaken by an explicit instruction
to change licence references in Klassivox's `docs/Projects/`, which the
absolute wording made impossible to obey without contradicting a
`LOCKED` decision. The default is unchanged. Only the escape hatch is
now written down.

## 14. Licence is MIT `LOCKED`

Maximum permissiveness and near-zero friction to adopt. The open-core
line is unchanged: the binary, the config format, and single-machine
operation are free forever. Hosted remote cache, distributed execution,
cache analytics, and SSO are the paid line. Never charge for the cache
being correct.

**Changed 2026-08-04.** This previously named a licence carrying an
express patent grant. MIT has none. That grant was judged not worth the
extra licence surface for a tool at this stage.

**Reopen if:** a commercial adopter's legal review asks for a patent
grant, or ipile takes outside contributions at a scale where a
contribution licence matters.

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

## 18. All six frontend layers ship in v0.1, and Frontend is exactly-one `LOCKED`

**Settles open decision 1**, which asked whether Vue and React both ship
on day one. Both do, and so do their meta-frameworks.

The first-party frontend set is `vue-vite`, `nuxt`, `react-vite`, `next`,
`astro`, and `svelte`. Backend layers are deferred. Adding a member to an
existing category is cheap: it shares the toolchain, quality, runner, and
infra layers already paid for. Adding a category is what is expensive,
and this adds none.

**Cardinality depends on the workspace shape.** A flat per-category rule
cannot express this, because "which framework does this app use" and "how
many apps are there" are two different questions:

| Shape | Frontend | Backend | Toolchain |
| --- | --- | --- | --- |
| `single-app` | zero or one | zero or one | zero or more |
| `monorepo` | zero or more | zero or more | zero or more |
| `tooling-only` | **none** | **none** | one or more |

`tooling-only` is what its name says, for dropping ipile onto a repository
that already exists. It takes no framework layers at all, because a framework layer
scaffolds an app and an app implies a shape. Naming both is a
contradiction, and it is rejected rather than half-scaffolded.

The toolchain flips from implied to required under `none`. Everywhere
else the framework choice implies the toolchain, per §2 of
[01-scaffolder.md](01-scaffolder.md). With no framework there is nothing
left to imply it, so the toolchain becomes the explicit answer to
"tooling for which ecosystem", `node-pnpm` or `rust-cargo`. The quality
layers then follow from it through their own `requires`.

It is named `tooling-only` rather than `none` because a shape named for
what it withholds tells the user nothing about what they get.

**A layer's identity is `<category>/<id>`, not the bare id.** `none` is a
legitimate answer for more than one category, and the merge engine keys
line blocks by identity, so two layers called `none` would silently share
a block. The stronger reason is decision 6: presets are distributed by
git URL, so third-party layers will eventually ship an id we do not
control, and a stranger's `nuxt` colliding with ours is a matter of time.
`requires` and `conflicts` name qualified ids for the same reason.

**Every workspace gets a `base` layer**, exactly one, whatever the shape.
It carries the ignores that belong to no framework, the OS and editor
noise every repository accumulates, plus a committed `.env.example`. Each
framework layer still ignores only its own build output, which is what
keeps layers independent. Without `base`, a Rust tooling-only selection
would ship a `.gitignore` holding nothing but `/target`.

Deliberately not one big comprehensive ignore file. That is what ipile's
own `.gitignore` was on 2026-08-04, the Visual Studio .NET template,
hundreds of lines of rules for tools this project does not use, and it
was missing `/target`. A generated file full of irrelevance teaches
people to stop reading it.

One app runs on one framework, so a single app takes at most one of each.
A monorepo is many apps, and a Nuxt marketing site beside a React admin
panel, or a Rust service beside a Node BFF, is the reason someone reaches
for a monorepo at all. Forbidding that would forbid the case ipile exists
to serve.

A single-app selection additionally needs at least one frontend or
backend. That rule spans two categories, so it lives in validation rather
than in a cardinality.

**Cardinality is a property of the category, never pairwise `conflicts`
entries between layers.** This matters more than the rule itself:

- Changing a category is one edit in one place, with no manifest changes,
  no format change, and no migration.
- Adding a seventh frontend never touches the six that already exist.
- The rule is stated once, so it cannot drift across manifests.
- `conflicts` stays for what it is actually for, genuine
  incompatibilities that cross categories.

**What this costs** is the thing to watch. A monorepo can now select
several frontends, so nightly CI cannot cover every reachable selection.
Two things carry that weight instead: presets are what CI builds, and the
merge engine turns a bad combination into a loud error naming both
layers rather than a subtly wrong file. If a generated workspace ever
fails to build in a way CI did not catch, that is the sharp kill
criterion in [01-scaffolder.md](01-scaffolder.md) §10, and the answer is
to narrow cardinality again rather than to patch around it.

**A consequence for layer authors:** layers in a many-per-workspace
category must not contribute colliding scalars. If every frontend wrote
`scripts.dev`, two frontends in one monorepo would be an instant merge
conflict. Scripts are namespaced per layer instead.

**Revised 2026-08-04**, same day it was written. The first version made
Frontend `exactly-one` outright, on CI-matrix grounds. That was wrong: it
optimised for a green matrix at the cost of the polyglot monorepo, which
is the product's whole reason to exist.

## 19. Templates ship in the binary and are comprehensive `LOCKED`

**Layers carry their own template files.** `layers/<category>/<id>/files/`
is copied into the generated workspace at scaffold time, with
`{{project_name}}` interpolation. ipile never fetches a template from a
git URL at scaffold time.

- Nightly CI can build what ships in the binary. It cannot build what is
  downloaded from a repository we do not control. §8 of
  [01-scaffolder.md](01-scaffolder.md) says untested templates are how
  every scaffolder dies.
- A scaffold that depends on a third-party repository being reachable and
  unchanged is a scaffold that breaks silently, months later, for
  somebody else.
- The template that shipped with a given ipile version is the one that
  version emits, forever.

Git URLs remain the distribution path for **presets**, per decision 6. A
preset is a list of layers. A layer is not fetched.

**Templates are comprehensive, not toy.** A generated backend is expected
to have configuration loading, error types, a health check, and an
integration test. A `main.rs` that prints hello is not a starting point
anybody keeps. The reference for the expected depth is
`https://github.com/dev-davexoyinbo/actix-web-starter-template`, which
was cloned into `docs/` on 2026-08-04 purely as a quality bar and is to
be deleted once the layers reach it.

**Comprehensiveness is composed, never copied wholesale.** That reference
bundles a Dockerfile, a compose file, SeaORM entities and migrations, and
a messaging crate. Copying it into one layer would recreate the
combination-authoring trap decision 4 exists to prevent: infra would be
duplicated against the `infra` layers, and `actix` would silently contain
a data layer, making `actix + prisma` incoherent. So it is split along
category lines:

- `backend/actix` owns the app crates and its own tooling config
- `data/sea-orm` owns entities and migrations
- `infra/dockerfile` and `infra/docker-compose` own containers

Cross-category dependencies are expressed with `requires`, not
`conflicts`. `backend/actix` requiring `data/sea-orm` makes
`actix + prisma` illegal automatically, because a single app takes at
most one data layer. No pairwise rules to maintain.

**The dividing line for any file:** if only one layer touches it, it is
copied. If several might, it is merged, and copying it would let one
layer clobber another.

---

## Open

Decisions not yet made. Do not treat silence here as agreement.

1. **Where the hosted remote cache runs, and what it costs**, which does
   not matter until the runner reaches v0.4.
2. **Whether `ipile.yaml` is the config filename.** It should probably be
   settled before any file format is written, since renaming a config
   file after release is a migration.
3. **Whether Klassivox's `docs/Projects/05-task-runner.md` becomes a stub
   pointing here** once these docs are complete.

Open decision 1, the first-party frontend set, was settled as decision 18
on 2026-08-04.
