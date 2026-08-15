# Context

**The one document for culm.** Settled decisions, coding conventions, and
the things a fresh session would otherwise rediscover the hard way.

State is not tracked here. The code and the git history track state.
Org-level decisions live in
[../../handbook/decisions/](../../handbook/decisions/) and are not
repeated.

**How to use this file:** when the owner corrects something, the
correction is appended under the section it belongs to. Corrections are
authoritative and outrank anything stated elsewhere, including
[../CLAUDE.md](../CLAUDE.md).

**Last updated:** 2026-08-15. `DECISIONS.md` was folded into this file on
that date. **Decision numbers are preserved**, because comments in
`src/`, in `layers/*/*/layer.toml`, and in `tests/` cite them by number.

---

## Decisions

All `LOCKED`. Settled, and not to be re-litigated by accident. Reopen
conditions are stated where one exists.

**1. culm is one tool with two halves.** A scaffolder and a task runner,
one binary, not two products. They differ in every commercial dimension:
the scaffolder runs once per project and has near zero moat, the runner
runs hundreds of times a day and is monetizable through a hosted cache.
They ship together because the scaffolder is the acquisition channel and
the runner is what retains. Almost nobody adopts Nx by reading about task
graphs; they run `create-nx-workspace` and are on Nx forever.
*Reopen if:* the runner is killed at its v0.2 checkpoint, at which point
culm is a scaffolder and the docs should say so plainly.

**2. The scaffolder ships first.** Roughly three to four weeks against
twelve, and it has no catastrophic failure mode: a bad scaffold is
noticed in ten seconds, a bad cache hit ships broken code and is trusted
while doing it. It also produces real signal cheaply. The runner's v0.2
checkpoint does not move.

**3. The runner is a layer, and Nx is a valid answer.** A hard constraint
on the code. The scaffolder must be able to emit Nx, Turborepo, or Moon
configuration, and `none` must be valid. Our runner is one option among
them and is not the default until it passes its own correctness check.
**If the scaffolder ever emits configuration only culm's runner can
consume, the whole argument for building the scaffolder first
collapses.**

**4. The unit is a layer, not a template.** A layer is a directory of
files plus a manifest describing how it merges into shared files.
Composition happens at scaffold time and **no combination is ever
authored by hand**. Categories: shape, frontend, backend, data,
toolchain, quality, runner, infra, editor, base. Picking `axum` implies
Rust, implies `Cargo.toml`, implies `rustfmt`, implies `cargo fmt
--check` in the pre-commit hook, so the user never answers a "which
language" question. **The real work is the merge engine**, not the
layers: patching `package.json`, `Cargo.toml` members, `pnpm-workspace`
globs, `tsconfig` paths, `.gitignore`, hook bodies and CI yaml from N
independent layers, deterministically, in any order. Everything else is
copying files. Full spec in [01-scaffolder.md](01-scaffolder.md).

**5. Template files never contain version numbers.** Pinned versions are
the single biggest cause of scaffolders rotting. Shell out to `pnpm add`
and `cargo add` after scaffolding and let the package managers write
versions and lockfiles. **Ranged only where the config shape is coupled
to a major**, so an eslint flat config layer declares `compat: ">=9
<10"`. "Latest stable" means latest that has been public for N days,
matching the `minimumReleaseAge` supply-chain guard. npm and crates.io
already are the version registry; the tool refuses to hardcode what they
say.

**6. Presets are first-class from v0.1, distributed by git URL.** A
preset is a name, a description, and a list of qualified layer ids.
First-party presets ship in the binary: `monorepo`, `frontend`,
`backend`. User presets work from day one via
`--preset github:user/repo`, so publishing is pushing a repository. After
any scaffold, offer to save the run as a local preset, which is what
converts a once-per-project tool into one with a reason to exist twice.
**A hosted registry is v2 or later, deliberately**: presets run install
commands, so hosting other people's means owning a code-execution supply
chain, with moderation and takedowns. Git URLs test the demand for free
first.

**7. JS/TS and Rust first, then Go, then JVM, then C/C++.** Sequencing,
not caution. JS and Rust are the hardest pair: a package manager that
writes a lockfile and hoists `node_modules`, against a workspace manifest
with a members array and a separate lockfile. Handle those together and
`go.work`, Gradle settings, and CMake subdirectories are the same shape
with less friction. C and C++ last, because CMake has no blessed
convention and that alone can eat a month.

**8. The Rust binary is the product, npm is the front door.** Distributed
on four channels, with prebuilt binaries shipped as platform-specific
`optionalDependencies`. Now an org decision:
[handbook 0004](../../handbook/decisions/0004-distribution-channels.md).

**9. Telemetry is opt-out, and the payload is published verbatim.**
Registry download counts are a trend line, not a user count. Sent:
anonymous per-machine UUID generated once, tool version, OS and
architecture, command name, and which layers and preset were selected.
That last field is the genuinely valuable product data. **Never sent:**
repository names, file paths, task names, environment variable values,
command strings, git remotes. `culm.yaml` contains user commands, so the
temptation to send them "for debugging" is real and is refused here in
advance. The exact JSON is printed on first run, not summarised as
"anonymous usage data"; every other tool in this category is quietly
cagey about it, and being the one that is not is cheap. Opt out with
`CULM_TELEMETRY=0`. Full payload in [TELEMETRY.md](TELEMETRY.md).

**10. This project is culm, and ipile is the bootstrap tool.** Grain was
abandoned for colliding with the Grain programming language. Four
characters, which matters for a command typed daily. Changed 2026-08-06:
this previously read "the name is ipile, and culm is reserved". The
sweep and every eliminated candidate is in
[../../handbook/naming/NAMING.md](../../handbook/naming/NAMING.md).

**11. One repository per project, abstract later.** Replaces the earlier
plan putting five Rust projects in one `rust-infra` workspace sharing
`crates/cas`. Shared code is extracted when a second real consumer
exists, not before. `cas` is a *content-addressable store*: it hashes a
file's bytes, stores the file under that hash, and hands the hash back. A
build cache is exactly that shape, which is why the runner needs one.
culm builds `cas` internally as a module, unpublished, and it is not
needed until the runner's v0.2. **Clarified 2026-08-04:** this once said
"builds `crates/cas` internally", which was read as a layout mandate. It
is not one.

**12. The organisation is Zunetra.** Deliberately sector-neutral, since
Klassivox and culm share nothing. `culm` was refused as an org name
because it is held as a product name. See
[handbook 0001](../../handbook/decisions/0001-org-name.md).

**13. Klassivox is not modified unless explicitly instructed.** It is the
proving fixture and first customer, in its own repository, and work here
does not reach into it. No amount of "the docs are now out of sync"
justifies an edit there. **The only authorisation is the owner asking for
a Klassivox change, naming it.** A generic instruction that merely sounds
repository-wide, such as "fix the docs everywhere", is not.
`docs/jottings/` is stricter still: only an instruction naming that file.
**Reworded 2026-08-04**, because the previous absolute ban made an
explicit instruction impossible to obey without contradicting a `LOCKED`
decision. The default is unchanged; only the escape hatch is written
down.

**14. Licence is MIT.** Maximum permissiveness, near zero adoption
friction. The open-core line: the binary, the config format, and
single-machine operation are free forever; hosted remote cache,
distributed execution, cache analytics, and SSO are paid. **Never charge
for the cache being correct.** Changed 2026-08-04 from a licence carrying
an express patent grant, judged not worth the extra surface at this
stage. *Reopen if:* a commercial adopter's legal review asks for a patent
grant.

**15. ASCII everywhere a machine reads it.** Plain `culm` in the folder,
repository, binary, npm package, crate, domain, environment variables,
and every identifier. Diacritics break shell paths, git remotes, URLs,
and package names, and nobody types accents at a terminal.

**16. culm is a sibling repository under `Projects/Zunetra`.** The parent
is a container, not a repository. Recorded in full, including why the org
repository is `handbook`, in
[handbook 0003](../../handbook/decisions/0003-workspace-layout.md).

**17. Klassivox's configuration becomes the first published preset.** Two
distinct things: the first-party `monorepo` preset shipped in the binary
is *derived from* Klassivox and nothing shipped is named Klassivox;
separately, the Klassivox setup is published as the first user-authored
preset over a git URL. The second is what validates decision 6, because a
preset system nobody outside the binary ever uses is a configuration file
with extra steps.

**18. All six frontend layers ship in v0.1, and cardinality is a property
of the category.** The first-party set is `vue-vite`, `nuxt`,
`react-vite`, `next`, `astro`, `svelte`. Adding a member to an existing
category is cheap; adding a category is what is expensive.

| Shape | Frontend | Backend | Toolchain |
| --- | --- | --- | --- |
| `single-app` | zero or one | zero or one | zero or more |
| `monorepo` | zero or more | zero or more | zero or more |
| `tooling-only` | **none** | **none** | one or more |

`tooling-only` is for dropping culm onto an existing repository. It takes
no framework layers, because a framework layer scaffolds an app and an
app implies a shape; naming both is a contradiction and is rejected
rather than half-scaffolded. It is named for what it gives, not for what
it withholds. Under it the toolchain flips from implied to required,
since with no framework nothing is left to imply it.

**A layer's identity is `<category>/<id>`, never the bare id.** `none` is
a legitimate answer in more than one category, and the merge engine keys
line blocks by identity. The stronger reason is decision 6: third-party
layers will eventually ship ids we do not control, and a stranger's
`nuxt` colliding with ours is a matter of time. `requires` and
`conflicts` name qualified ids for the same reason.

**Every workspace gets exactly one `base` layer**, carrying the ignores
that belong to no framework plus a committed `.env.example`. Each
framework layer ignores only its own build output, which is what keeps
layers independent. Deliberately not one big comprehensive ignore file: a
generated file full of irrelevance teaches people to stop reading it.

**Cardinality lives on the category, never in pairwise `conflicts`.**
Changing it is one edit in one place, adding a seventh frontend never
touches the six that exist, and the rule cannot drift across manifests.
`conflicts` stays for genuine cross-category incompatibilities.
**What it costs:** a monorepo can select several frontends, so CI cannot
cover every reachable selection. Presets are what CI builds, and the
merge engine turns a bad combination into a loud error naming both
layers. **A consequence for layer authors:** layers in a many-per-workspace
category must not contribute colliding scalars, which is why scripts are
namespaced per layer.
**Revised 2026-08-04**, the same day it was written. The first version
made Frontend `exactly-one` on CI-matrix grounds, which optimised for a
green matrix at the cost of the polyglot monorepo, the product's whole
reason to exist.

**19. Templates ship in the binary and are comprehensive.** Layers carry
their own files in `layers/<category>/<id>/files/`, copied at scaffold
time with `{{project_name}}` interpolation. **culm never fetches a
template at scaffold time**: CI can build what ships in the binary and
cannot build what is downloaded from a repository we do not control, and
the template that shipped with a given version is the one that version
emits forever. Git URLs remain the distribution path for *presets*, per
decision 6. A preset is a list of layers; a layer is not fetched.

**The bar is comprehensive, not toy.** A generated backend has
configuration loading, error types, a health check, and an integration
test. A `main.rs` that prints hello is not a starting point anybody
keeps. The reference was a clone of
`dev-davexoyinbo/actix-web-starter-template`, kept in `docs/` from
2026-08-04 and **deleted on 2026-08-15 once the backends reached it**;
`backend/actix` is now the reference. Its copyright was `2025 Oyinbo
David Bayode`, MIT, so anything still derived from it owes attribution.

**Comprehensiveness is composed, never copied wholesale.** That reference
bundled a Dockerfile, a compose file, SeaORM entities and a messaging
crate. Copying it into one layer would recreate exactly the
combination-authoring trap decision 4 exists to prevent: `actix` would
silently contain a data layer, making `actix + prisma` incoherent. So it
splits along category lines, and cross-category dependencies use
`requires`, not `conflicts`. **The dividing line for any file:** if only
one layer touches it, it is copied; if several might, it is merged,
because copying would let one layer clobber another.

**20. Presets are TOML, not YAML.** Decided 2026-08-14. `presets/<name>.toml`
with `name`, `description`, and a `layers` list. The crate already parses
TOML for every `layer.toml`, so a second serialisation format would be a
dependency bought for one file shape, and `serde_yaml` is archived and
unmaintained. First-party presets compile into the binary the same way
layers do, because a preset naming an unresolvable layer is a shortcut
that fails only for the user who takes it. This does not settle open
question 2, which is about a different file.

**21. Generated JavaScript is ESM only.** Decided 2026-08-14, on finding
`frontend/svelte` and `backend/express` could not coexist. There is no
CommonJS option and no per-layer choice. **`toolchain/node-pnpm` owns
`"type": "module"`** in the root `package.json`: it is the layer every JS
layer already requires and already owns `private = true`, so the module
system gets one owner rather than a key each app layer argues over. Every
JS and TS layer emits `import`/`export`.
**Why one answer rather than a negotiated one.** SvelteKit, Astro and
Next all load ESM config. `backend/express` correctly refused to set
`type` at the root, on the grounds that one app's module system is not
its call to make for everything beside it, which left `svelte + express`
broken. Forcing `.mjs` everywhere leaves two module systems alive so the
next pair finds a new way to disagree, and a rule for that specific pair
is exactly what decision 4 forbids. The module system is a property of
the workspace, so it is settled once at the workspace level.
**The escape hatch is per file:** a file that must be CommonJS names
itself `.cjs`. Nothing sets `type` back.
**What it costs:** express and fastify convert. `backend/nestjs` is the
real risk, since Nest targets CommonJS by default and `ts-jest` under ESM
is fiddly. *Reopen if:* a first-party layer cannot be ESM at any
reasonable cost. Preferring CommonJS is not a reason.

### Open

Not yet decided. Do not treat silence as agreement.

1. **Where the hosted remote cache runs, and what it costs.** Does not
   matter until the runner reaches v0.4.
2. **Whether `culm.yaml` is the config filename.** Should be settled
   before any file format is written, since renaming a config file after
   release is a migration. Decision 20 is a nudge towards TOML but
   decided a different file.
3. **Whether Klassivox's `docs/Projects/05-task-runner.md` becomes a stub
   pointing here.**

---

## Comment conventions

Comment on non-obvious WHY, never WHAT. Comments are warranted where the
reasoning is not visible in the code, not as decoration every few lines.

**Section headings.** `// === Section` is the top-level divider.
`// == Subsection` exists for a section genuinely large enough, and
`// = Sub-subsection` exists and is rare; a file needing that depth
should probably be split. Plain `//` is never a heading.

**Doc comments.** `///` is encouraged on public items, **one sentence per
line**. Break at the full stop and start a new `///` line. Never wrap one
sentence across several, so a diff touching one sentence touches one
line.

```rust
/// Identifies the layer a contribution came from.
/// Ordered, because rendered block order is defined as sorted by layer id.
pub struct LayerId(String);
```

**Ordinary comments use `/* ... */`, not `//`**, for single lines as well
as paragraphs. Do not stack bare `//` lines.

**No em dash** in any comment, commit message, document, or PR
description. Use a plain hyphen or rewrite the sentence.

READMEs follow
[handbook 0005](../../handbook/decisions/0005-readme-standard.md).

---

## Rust and layout

**culm is a single crate.** One package, one `src/`, edition 2024. Not a
Cargo workspace, no `crates/` directory. Decision 11 says abstract later,
the merge engine is `src/merge/` rather than a crate, and splitting a
module out later is mechanical while un-splitting a premature workspace
is not.

- `rustfmt.toml` and `clippy.toml` stay at defaults.
- `cargo clippy --all-targets -- -D warnings` must pass before any
  commit.
- Errors are `thiserror` in library code, `anyhow` at the binary
  boundary.

---

## Modularity and separation of concerns

**A module is a directory with a `mod.rs`, not one long file.** Single
crate does not mean single file.

**One concern per file.** The test is whether the file has one reason to
change. `src/layer/` splits into `category.rs`, `manifest.rs`,
`validate.rs` and `error.rs` because what a category is, what a manifest
holds, whether a selection is legal, and how failure is reported are four
separate questions.

**`mod.rs` declares and re-exports and holds no logic**, so internal
reorganisation does not break callers. A `mod.rs` that has grown its own
functions should have been a submodule.

**Errors live in their own `error.rs`** once a module has more than a
couple of variants. The error type is the module's public contract for
failure.

**Tests sit beside the code they test**, in `#[cfg(test)] mod tests`
under a `// === Tests` heading. Cross-module behaviour goes in `tests/`,
where the crate is exercised the way a caller would use it.

**Split when a file earns it, not preemptively.** A 40 line module does
not need a directory.

**The second caller is when a function moves, and it moves immediately.**
Writing the same logic twice is two answers to one question, and they
drift the first time somebody corrects one of them.

**Shared code goes where its subject lives, not in a `util` bucket.** A
crate-wide `util/` has no reason to change, which means it has no
boundary and accumulates whatever is homeless. A genuinely homeless
helper usually means the module it belongs to has not been written yet.

**Prefer a generic over an iterator to a concrete container.** A helper
taking `impl IntoIterator<Item = &Manifest>` serves a `Vec`, a
`BTreeMap`'s values, and a filter chain without any caller allocating to
fit its signature.

**A new category, strategy, or layer must not require editing unrelated
files.** Anything discovered by walking a directory or iterating
`ALL_CATEGORIES` scales; anything hardcoded as a list of names is a bug
waiting for the next addition.

---

## Safety and allocation

**No unsafe code.** `#![forbid(unsafe_code)]` sits at the top of both
`src/lib.rs` and `src/main.rs`, so the compiler enforces it rather than a
document asking nicely.

**Borrow by default.** `&str` over `String`, `&Path` over `PathBuf`,
iterators over eagerly collected `Vec`s.

**Cloning is allowed where it buys something**, deliberately not banned.
The scaffolder runs once per project, so a clone upholding an invariant
is a good trade: `JsonDoc::merge` clones the document and its provenance
map so a conflict halfway through leaves the document untouched, and
hand-rolling rollback would be more code and more ways to be wrong to
save microseconds. In the runner, a clone on a hot path needs a comment
saying why. **Never clone merely to quiet the borrow checker.** That
means the ownership design is wrong and the clone is hiding it.

---

## The total order invariant

Any comparator the merge engine sorts with must be a **total order
consistent with equality**, reporting `Equal` only when the values
genuinely are equal.

It was violated once, and the failure is the worked example. Array
entries are sorted so output does not depend on the order layers were
applied. The comparator ordered JSON numbers through `f64`, which carries
53 bits of precision, so integers above 2^53 collapsed onto the same
value and compared `Equal` while being different. `sort_by` is stable, so
it left them in arrival order, and arrival order is layer order: the
rendered bytes changed depending on which layer went first, which is
exactly the property the engine exists to guarantee. The fix breaks the
tie on the number's exact textual form. Regression test:
`integers_beyond_f64_precision_still_sort_deterministically`.

Two lessons worth carrying:

- The property test missed it because its generators drew from small
  pools and could never produce two numbers that collapse. A property
  test is only as strong as the inputs it can reach.
- The failure was silent. Nobody traces a mysterious byte difference back
  to floating point precision, and silent wrong output is the same
  category as the runner's worst failure mode.

---

## Git

**Committing is an explicit instruction, never an assumption.** Finish
the work and leave it uncommitted. Not because the change looks complete,
not because a milestone was reached, not because a task was called done.

**Pushing is a separate explicit instruction.** Permission to commit is
not permission to push.

**Every commit is signed.** `git commit -S`, or a configured
`commit.gpgsign`.

**Every commit message is one line**, `<type>: <description>`, using
`feat`, `fix`, `refactor`, `chore`, `test`, `docs`, `perf`, `style`. No
body, no bullets, no trailers, no co-authorship attribution.

---

## What is being built, and in what order

**The goal.** A user picks a stack and gets a workspace they would have
been happy to write themselves. The quality bar is decision 19:
configuration loading, error types, a health check, an integration test,
real tooling config.

**What exists.** The merge engine with `json-deep`, `line-block`, and
`toml-members`. The layer format with categories, shape-dependent
cardinality, manifest parsing, and selection validation. The scaffold
module: interpolation, copy plan, disk writing. Twenty six layers across
base, shape, frontend, backend, data, toolchain, quality, and runner,
compiled into the binary by `include_dir` with `Catalog::load` kept as
the `--layers-dir` escape hatch. `culm new`, non-interactive, in
`src/cli/`. The install step in `src/install/`, running `pnpm add` and
`cargo add` in the generated workspace, with `--no-install` printing the
plan. Presets in `src/preset/`, TOML, compiled in the same way.

**What does not exist yet.** The wizard. Git URL presets, and saving a
run as a preset. The `hook-script` and `yaml-steps` strategies.
Telemetry. The runner half. The `infra` and `editor` layers, which have
categories and no members. Five of the six frontends, which are
manifest-only. There is no ordinary CI workflow either: `.github/` holds
the template matrix and nothing else, so the clippy gate is still run by
hand.

**The order from here.**

1. `frontend/next`, `astro`, `react-vite`, `vue-vite`, `svelte` to the
   decision 19 bar. `frontend/nuxt` is the reference; the others emit
   scripts without an application.
2. The `infra` and `editor` layers: docker-compose, dockerfile,
   github-actions, husky, commitlint, vscode, editorconfig.
3. The wizard. Deliberately last: it is the part everyone builds first
   and the part that matters least, and flags have to work anyway.

The order actually followed so far was an interleave rather than a
sequence: layers to the quality bar, then the CLI, then the installs,
then one frontend, then presets. Doing the CLI before every layer was
finished meant no CLI test was written against a surface that then had to
change.

**Two rules that shape every layer authored from here.**

- Scripts in a many-per-workspace category are namespaced by layer id
  (`dev:nuxt`, `dev:nestjs`), because two layers writing different values
  to the same `package.json` key is a hard merge error. Quality layers
  are not namespaced, since at most one of each is selected and `format`
  has a single owner.
- A layer never names another layer in its content. Not in a package
  name, not in a script, not in an ignore rule. A test fails if one does.

---

## How a template stays current

`.github/workflows/templates.yml` scaffolds each layer that emits an
application, then builds and tests it. Weekly on a cron, and on demand.

**There is nothing to bump.** Decision 5 keeps versions out of templates,
so `pnpm add` and `cargo add` resolve whatever is current on the day the
job runs. The build is the version check. Renovate and Dependabot have
nothing to act on, and adding them would be a category error.

**What it catches is shape, not version.** A patch release never breaks a
template. A router convention being replaced, a config file being
renamed, or a required file appearing does. Those land once or twice a
year per framework.

**A red run is a signal to re-harvest by hand.** Re-run that framework's
own generator, harvest the changed files, strip whatever version the
generator wrote, and push. **Nothing automated may rewrite a template:**
those files are the quality bar, and a bot committing framework bytes
nobody read means the first person to see a regression is a user.

**It is also the manual verification gate.** `cargo test` cannot reach a
network, so no test here can prove a template builds. A newly authored or
harvested layer goes through this workflow with `workflow_dispatch`
before it is called done. Only layers that emit an application belong in
the matrix; a manifest-only layer would fail for a reason unrelated to
upstream, which would make a red run mean nothing.

---

## Traps already identified, do not rediscover

- **Never author a combination of layers.** The moment a special case
  exists for a specific pair, the combinatorial death has started. A kill
  criterion, not a style note.
- **Never emit configuration only culm's runner can consume.** Decision
  3.
- **Never put a version number in a layer template file.** Decision 5.
- **Never modify Klassivox unless explicitly told to**, naming it.
  Decision 13.
- **The runner is expected to possibly die at r0.2.** Planned, not
  failure. [02-runner.md](02-runner.md) §10.

---

## Context a fresh session will not have

- **Merged files are never interpolated.** `Variables::render` runs over
  copied `[[files]]` only; whatever `FileSet::render()` produces is
  written verbatim. The visible consequence is that no layer names the
  root `package.json`, so every scaffolded Node workspace gets one
  without a `name`. Either a merge value has to interpolate too, or a
  layer has to own that key. Found while writing `tests/cli.rs`, and
  deliberately left unfixed because it is a product question.
- **A cargo install is pointed at the crate its layer ships**, derived in
  `src/install/` from the layer's own `[[files]]` entry ending in
  `Cargo.toml`. The root manifest is a `[workspace]` with no `[package]`,
  so an unqualified `cargo add` there has nothing to add to. Deriving the
  path keeps the layer format from growing a field whose only correct
  value is one the layer already stated.
- The name was Grain until it collided with the Grain programming
  language. Every rejected alternative and its reason is in
  [../../handbook/naming/NAMING.md](../../handbook/naming/NAMING.md), so
  the search is not repeated.
- The org was almost Nomia. It was taken. github.com/Zunetra now hosts
  `culm` and `handbook`.
- `Projects/Mine` was renamed to `Projects/Zunetra` on 2026-08-04.
- `docs/prompt/` held the original working notes and was a locked
  directory. The owner deleted it on 2026-08-04 and asked for the lock to
  go with it. Do not restore it.
- `docs/Left.md` held the work queue for the T1 to T7 slice and was
  deleted on 2026-08-15, as that file itself instructed. Everything from
  it worth keeping is in this document.
- `docs/DECISIONS.md` and `docs/NAMING.md` were folded into this file and
  into the handbook on 2026-08-15. Decision numbers were preserved
  because the code cites them.
