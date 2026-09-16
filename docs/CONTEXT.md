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

**22. The config file is `culm.toml`.** Settled 2026-09-07, closing open
question 2. TOML, not YAML: decision 20 already committed to TOML
everywhere else in the project (`layer.toml`, `presets/*.toml`)
specifically so a second serialisation format is never bought for one
file shape, and `culm.toml` names the tool rather than a generic word
(`config.toml`) a real project's own tooling might already claim. Nothing
reads or writes this file yet; the runner is what will, starting at v0.2.
Settling the name now is what decision 2's own reasoning already asked
for: renaming a config file after release is a migration, so the name is
worth fixing before anything is built against it.

**23. A JS app in a monorepo owns a real `package.json`, not a namespaced
contribution to a shared root one.** Settled 2026-09-14, closing open
question 4. `backend/actix` already gets a real crate at `crates/actix`
via `toml-members`, and `backend/go-chi` already gets a real module at
`{{app_dir}}/go.mod`; JS was the one ecosystem still sharing one root
manifest with `:​<id>`-suffixed scripts. This is what makes `runner/turborepo`
and `runner/moon` writable at all: neither can orchestrate a workspace
with exactly one JS package in it, which is also why `runner/nx` has
shipped as a stub. The merge target itself needed no new capability:
`CatalogEntry::interpolated` already renders the whole manifest's raw text
against the layout variables before parsing it, so a `target =
"{{pkg_target}}"` line resolves the same way a `[[files]] to =
"{{app_dir}}/..."` path already does. `pnpm-workspace.yaml` needed no new
merge strategy either: `yaml-list`, landed 2026-08-19, was written for
exactly this case and had simply never been pointed at a layer.

Migration in progress, one layer at a time, the same discipline
`backend/go-chi` went through alone before anything else changed.
`frontend/react-vite` is the first layer through, fully: `package.json`
(with a real `name`/`private`/`type`), `vite.config.ts`,
`tsconfig.json`/`tsconfig.test.json`, `vitest.config.ts`, and its test
file all now live under `{{app_dir}}`, `config_suffix` dropped for this
layer's own scripts since a real per-app package needs no namespacing.
`eslint.react-vite.config.mjs` stays at the workspace root deliberately:
the root `eslint.config.mjs` discovers fragments by scanning its own
directory, not recursively, so a fragment moved into `{{app_dir}}` would
stop being found at all.

Verified for real in a hand-scaffolded two-app monorepo (`frontend/nuxt`
+ `frontend/react-vite`, a hand-written `pnpm-workspace.yaml` since
generating one is not built yet): `pnpm install`, then
`pnpm --filter proof-react-vite run build`, `run typecheck`, `run test`,
and the root `run lint` all passed. The first pass surfaced a real bug on
the way: `install::plan` had no case pointing `pnpm add` at a layer's own
package.json, so `react`/`react-dom` landed in the shared root manifest
instead of `react-vite`'s. Fixed the same way `go_module_dir` already
handles the equivalent case for `go get`: a `pnpm_package_dir` lookup
over the layer's own merges, and `-C <dir>` added to the command before
`add`.

**Migration finished 2026-09-16 for nine of the ten remaining layers.**
`vue-vite`, `nuxt`, `astro`, `svelte` followed `react-vite` through in an
earlier pass (undated in this log, found already done at the start of
this session). This session did the last three frontends and backends:
`next`, `express`, `fastify`, `nestjs`. `nestjs` was the heaviest of the
nine: it had never used the layout variable system at all, hardcoding
`:nestjs` script suffixes and a `src/nestjs/` source path directly, so it
was rewritten rather than migrated.

**A cross-cutting bug surfaced while verifying `next`'s monorepo shape,
and it was not new.** Every migrated layer wrote `cd {{app_dir}} &&
<command>` for its scripts. That is correct in single-app shape, where
the real package.json sits at the workspace root while the sources sit
under `{{app_dir}}`. It fails outright in monorepo shape: pnpm already
runs a package's scripts with its cwd at that package's own directory,
which is the same directory `{{app_dir}}` names once the package.json is
real, so the script tries to `cd` into a directory nested inside itself
and pnpm reports `can't cd to src/<id>`. This means `react-vite`,
`vue-vite`, `nuxt`, `astro`, and `svelte` were never actually correct in
monorepo shape despite this log's own earlier claim of a verified
two-app monorepo, because that hand-built proof happened to invoke
`vite`/`nuxt` directly rather than through the package's own npm script.
Fixed with a new layout variable, `{{cd_app_dir}}` (`src/scaffold/layout.rs`):
`"cd src && "` in single-app, empty in monorepo, since pnpm is already
where it needs to be there. Every script in all nine layers now writes
`{{cd_app_dir}}<command>` instead of hardcoding the `cd`.

Per-layer bugs the real verification pass also found, each the same
shape decision 23's `react-vite` proof already warned about (a path that
was correct before a file moved one directory deeper and was not
recalculated): `express` and `fastify`'s test file imported the app
module through `{{to_root}}{{app_dir}}/app.js`, stale after the test
moved to `{{app_dir}}/test/`; the fix is the plain relative `../app.js`
the new location actually needs. `nestjs`'s `test` script resolved jest
through a `node_modules/jest/...` path that only exists in monorepo
shape (single-app's `node_modules` sits at the workspace root, one level
above `{{app_dir}}`); fixed by invoking `jest` through PATH with
`NODE_OPTIONS=--experimental-vm-modules` instead. `next`'s own
`typescript-eslint` install targeted `{{pkg_target}}` instead of the
workspace root, so root `eslint .` could not resolve it once `next`
had its own per-app manifest in monorepo shape; fixed with `root = true`,
matching `react-vite`'s own equivalent block.

**`pnpm-workspace.yaml` generation shipped 2026-09-16, monorepo shape
only.** A single new global layout variable, `{{workspace_glob}}`
(`"src/*"` in monorepo, blank otherwise), fed to one new
`toolchain/node-pnpm` merge block. Computed globally rather than
per-entry like `app_dir`, because a tooling layer is never an app and
would otherwise always land in the namespaced branch regardless of the
real count. Blank rather than omitted deliberately: `yaml-list`
(`src/merge/yaml_list.rs`) now drops a blank entry rather than writing
it, and drops the target's key entirely when every entry offered for it
was blank, which is what makes the file simply not appear for a
single-app or tooling-only workspace rather than appearing empty.
`scaffold::write` gained the matching half: a merge that rendered to
nothing is skipped rather than written as an empty file. Verified this
does not silently start littering unrelated files: `write`'s change is
generic (any strategy, not just `yaml-list`), covered by both an
`an_empty_sequence_still_renders_as_a_sequence` regression test (a layer
naming a genuinely empty sequence on purpose still gets `name: []`) and
a new `a_blank_entry_from_an_unresolved_variable_contributes_nothing`
one for the new case.

**Enabling a real `pnpm-workspace.yaml` exposed a second, install-ordering
bug**, found the same day while confirming the feature end to end against
a live registry. Once pnpm recognises a real workspace, `pnpm -C <member>
add` runs the workspace root's own lifecycle scripts too, and `culm new`
was running each layer's installs in selection order, which commonly puts
a frontend or backend layer before the quality layers that add the root's
own devDependencies. The very first per-app install could therefore try
to run `quality/husky`'s `prepare` script before `husky` itself was
installed, failing with exit 127. `install::plan` now stable-sorts its
groups so every workspace-root install runs before every per-app one,
regardless of which layer's turn produced it first; within each of the
two partitions the original first-seen order (and the existing tests
asserting it) is untouched.

**Data layers (`drizzle`, `prisma`) are deliberately out of decision
23's scope, decided 2026-09-16.** `generate`/`migrate`/`studio` are not
"an app's" scripts the way `dev`/`build` are: they operate on a schema
independent of which backend process happens to be running, so there is
no real "which backend does this belong to" question to answer even in a
monorepo holding several. They stay namespaced (`:​<id>`-suffixed) at the
shared root package.json, as they always have been. Two other shapes
were considered and rejected: giving a data layer its own per-layer
package.json (sidesteps the multi-backend question but moves
`prisma.config.ts`/`schema.prisma` off the workspace root, fighting
Prisma's and Drizzle's own tooling conventions for no real gain), and
capping a data layer to workspaces with at most one backend (caps a
selection decision 18 otherwise allows, to solve a problem that turned
out not to exist).

**An unrelated bug found incidentally while verifying `express`:** the
boilerplate `.gitignore` (Visual Studio template) carries `[Ee]xpress/`
for an "Installshield output folder", which also matches
`layers/backend/express/` by directory name alone. This silently
excluded five of that layer's six files from git since the layer was
authored: only `server.js` was ever committed. A fresh clone was
missing `app.js`, `config.js`, `errors.js`, `eslint.config.mjs`, and
`health.test.js` the whole time, though the compiled binary never
noticed because `include_dir!` reads the working tree at build time, not
git. Fixed with a `!layers/backend/express/` negation rather than
deleting the boilerplate rule, which is still correct everywhere else in
the tree.

**What is left of the original ten:** none. `pnpm-workspace.yaml`
generation and the data-layer question, both called out as not started
in this log's previous entry, are both done now too.

### Open

Not yet decided. Do not treat silence as agreement.

1. **Where the hosted remote cache runs, and what it costs.** Does not
   matter until the runner reaches v0.4.
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
  commit. `Cargo.toml`'s `[lints.clippy]` allows `needless_return`: an
  explicit `return` is a chosen style here, not a mistake the gate should
  flag. Deny-by-default stays the rule; this is the one named exception.
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

**What exists.** The merge engine, complete: `json-deep`, `line-block`,
`toml-members`, `yaml-list`, `hook-script`, `yaml-steps`. The layer
format with categories, shape-dependent cardinality, manifest parsing,
and selection validation. The scaffold module: interpolation, copy plan,
disk writing, and the layout system (`{{app_dir}}`, `{{test_dir}}`,
`{{script_suffix}}`, `{{config_suffix}}`, `{{to_root}}`) that lets a
layer never hardcode its own depth. Thirty four layers across base,
shape, frontend, backend, data, toolchain, quality, runner, infra, and
editor, compiled into the binary by `include_dir` with `Catalog::load`
kept as the `--layers-dir` escape hatch. `culm new`, non-interactive, in
`src/cli/`. The install step in `src/install/`, running `pnpm add` and
`cargo add` in the generated workspace, with `--no-install` printing the
plan. Presets in `src/preset/`, TOML, compiled in the same way. The
startup banner in `src/banner.rs`, printed after argument parsing and
suppressed when stdout is not a terminal, when `NO_COLOR` is set, or when
`CULM_NO_BANNER` is set. Two GitHub Actions workflows: `templates.yml`
(weekly, proves every shipped layer that emits an application still
installs and builds against a live registry) and `ci.yml` (every push and
PR: build, `cargo install`, clippy).

**Proven, not just parsed, as of 2026-09-07.** Every one of the thirty
four layers has been scaffolded, installed with real registry packages,
and built by hand, in both `single-app` and `monorepo` shape. This
surfaced and fixed a systemic bug class: several layers hardcoded a
relative-path depth (a `../` count, a fixed `src/<id>` path) that was
correct for single-app and silently wrong the moment a second app layer
forced namespacing. `{{to_root}}` exists because of this. It also
surfaced and fixed a run of smaller, real defects the merge engine and a
reading could not catch on their own: a `.gitignore` contribution that
stopped matching its own build output once namespaced, `svelte-check`
leaking into a sibling frontend's files under monorepo, `astro check`
never type-checking the shipped test file at all, three frontend
component parsers not being told about TypeScript, missing Node globals
failing `lint` before any user code was written, a missing HTTP platform
adapter that would have crashed NestJS at boot, and an npm dist-tag trap
where `prisma@latest` currently resolves to a broken pre-release (see
"Traps already identified" for the version caps this forced).

**The scaffolder (v0.1) is complete as of 2026-09-08.** Every item that
was on this list is done:

- **The wizard**, `src/wizard/` (`ratatui` + `crossterm`). Triggers on
  `culm new <name>` with no `--layer`/`--preset` given, only when both
  stdin and stdout are real terminals; falls through to the same error a
  script has always gotten otherwise, so nothing about scripted or CI use
  changed. Walks shape, frontend, backend, (toolchain, only under
  `tooling-only`), quality, infra, editor, runner, using the real
  `Category::cardinality`/`shape_of` logic rather than a second copy of
  either. Data layers (`sqlx`, `prisma`, `drizzle`) are deliberately not
  in the wizard flow: they are not in `01-scaffolder.md`'s documented
  order either, so none was invented for them. Verified live via a real
  pty: the cancel path, the "accept every default" path (which correctly
  hits the existing cross-category validation error rather than the
  wizard trying to re-implement that rule), and a real successful
  scaffold.
- **Git URL presets**, `src/preset/remote.rs`. `--preset github:user/repo`,
  `gitlab:user/repo`, an optional `/path/to/file.toml` and `@ref`, or a raw
  git URL, fetched via a real `git clone` into a local cache keyed by the
  clone URL. Tested against real local git repositories standing in for
  "remote" (`git clone` treats a filesystem path exactly like a URL), so
  `cargo test` stays fully offline. `--preset ./file.toml` also works, no
  git involved, added because the feature was already half-built once the
  local-file case was considered.
- **Saving a run as a preset**, `--save-preset NAME` on `culm new`. Writes
  the fully resolved layer list (every default included, not just what
  was typed) to `<NAME>.toml` beside the generated workspace, in the same
  shape a first-party preset already parses, so `--preset ./NAME.toml`
  reads it straight back.
- **Telemetry**, `src/telemetry/`. The Rust side only: UUID persistence
  (`dirs`), the exact decision-9 payload and nothing else, opt-out via
  `CULM_TELEMETRY=0`, a 500ms best-effort send on a detached thread so a
  dead network never adds visible latency. Wired into `main.rs` only,
  never into `cli::new` itself, specifically because `tests/cli.rs` calls
  `run_new` directly to avoid spawning a process, and that must never
  touch the network or this machine's real telemetry id as a side effect
  of `cargo test` — the same reasoning that already keeps `banner::print`
  out of `cli::new`. The receiver is written but not deployed:
  `telemetry/worker/` is a complete Cloudflare Worker + D1 schema with a
  step-by-step `README.md`; `DEFAULT_ENDPOINT` in `src/telemetry/mod.rs`
  still points at a domain that does not resolve to anything yet.

**Decision 22 settled 2026-09-07**: the config file is `culm.toml`,
closing open question 2.

**The runner (v0.2) has started, milestone r0.1 done.** `src/runner/`:
`culm.toml` parsing, a `petgraph` task graph, named-cycle detection,
topological execution of one task's dependency closure, sequential, no
caching, per `docs/02-runner.md` section 6's own scope for this
milestone. Dogfooded for real 2026-09-08 against a genuine scaffolded
`monorepo` (Nuxt + Axum): `culm run build`/`culm run test` correctly
ordered and ran the real `cargo`/`pnpm` commands, and a real injected
compile error in the backend crate stopped the run at the right task with
the right exit code, without ever reaching the tasks that depended on it.
No bugs found. Per the doc's own methodology this is the evidence r0.2
(content hashing, local caching via `cas`) wants before it starts, not a
substitute for the two-week real-use check r0.2 itself requires once it
exists.

**Go exists as of 2026-09-08**: `backend/go-chi` (the `go-chi/chi` router)
and `toolchain/go-mod`, decision 7's "then Go." No `go.work`: Go modules
are directory scoped, so `{{app_dir}}/go.mod` is a complete module on its
own and a second Go backend needs no shared root file to collide with —
`backend/go-chi` carries no `[[merge]]` entry at all. `src/install/mod.rs`
gained a `"go"` manager (`go get`, pointed at the right module via a
leading `-C {{app_dir}}` the same way `cargo add` uses `--manifest-path`
for a virtual workspace root) rather than leaving the dependency to be
added by hand, which is what every other backend already gets for free.
Building this exposed a real, previously-latent bug: `install::plan` was
reading the **uninterpolated** `CatalogEntry` list, so a `[[files]] to`
path containing a placeholder (`{{app_dir}}/go.mod`) reached it as literal
text instead of the resolved path. This never surfaced before because
every Rust layer's `Cargo.toml` sits at a literal `crates/<id>/` rather
than a templated location — `backend/go-chi` was the first layer whose
install-relevant path was ever templated. Fixed in `src/cli/new.rs`: the
already-interpolated manifests are what `install::plan` reads now.
Verified for real: `go get` resolving and writing a real version into
`go.mod`, `go build`/`go test` passing, in both single-app and monorepo
shape (the latter proving `{{app_dir}}` namespacing reaches the install
step correctly too, not only the copy plan).

**What does not exist yet.** r0.2 onward (caching, `--affected`, parallel
execution, remote cache) — not started, and not to be started until r0.1
has seen more real use per the point above it. Real per-app JS packages
for a monorepo (decision 23, open question 4) finished 2026-09-16, which
is what was blocking `runner/turborepo` and `runner/moon` from ever
being written; neither has been started yet, only unblocked. `runner/nx`
is still an honest stub for the reason it always was (it installs real
`nx`, emits no `nx.json`, and says so). The Cloudflare telemetry
receiver is written but not deployed. Klassivox's setup published as a
preset (decision 17's second half) is deliberately not being pursued
right now.

All six frontends reached the decision 19 bar on 2026-08-18. Each ships a
runtime-configured entry page, an error boundary, a scoped tsconfig, a
vitest config named for the layer, and one component test.

The order actually followed so far was an interleave rather than a
sequence: layers to the quality bar, then the CLI, then the installs,
then one frontend, then presets, then every layer to the proof-by-build
bar, then the infra/editor gap, then the wizard and remote presets and
telemetry and runner r0.1 together in one session. Doing the CLI before
every layer was finished meant no CLI test was written against a surface
that then had to change.

**A process trap, not a design one.** 2026-09-08: a background agent
working on the runner ran a bare `git stash` to isolate its own changes
while debugging, which stashed every tracked-file change in the working
tree indiscriminately, including unrelated concurrent work from another
session. Recovered without loss because `git stash` stashes rather than
discards, but the lesson generalises: **never run `git stash`, `reset`,
`checkout`, or `clean` against a shared working tree without checking
`git status` first**, and scope a stash to specific paths
(`git stash push -- <paths>`) rather than a bare `git stash` when there
might be work outside the files actually being edited.

**Two rules that shape every layer authored from here.**

- **An app layer never hardcodes its own layout.** It names
  `{{app_dir}}`, `{{test_dir}}`, `{{script_suffix}}` and
  `{{config_suffix}}`, and `src/scaffold/layout.rs` fills them in from
  the selection: a workspace holding exactly one application gets `src`,
  `test`, `dev` and `vitest.config.ts`, and a workspace holding two or
  more gets `src/<id>`, `test/<id>`, `dev:<id>` and
  `vitest.<id>.config.ts` each. Two layers writing different values to
  the same `package.json` key is a hard merge error, so the namespaced
  form is not optional once there is company; hardcoding it when there is
  none makes every single-app workspace look like a monorepo, which is
  the common case paying for the rare one. **The count of application
  layers decides, never the shape**: a `single-app` workspace still takes
  one frontend beside one backend, which is two layers wanting `dev`.
  Quality layers take no suffix at all, since at most one of each is
  selected and `format` has a single owner.
- **The ESLint fragment is the one filename that stays namespaced
  always.** `quality/eslint` owns the root `eslint.config.mjs` and
  discovers fragments by the pattern `eslint.<something>.config.mjs`, so
  a fragment collapsing to `eslint.config.mjs` in a one-app workspace
  would both collide with the root config and stop being found.
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

- **A layer's `tsconfig.json` goes inside that layer's own source
  directory, and is never renamed.** Two frontends cannot both own the
  root `tsconfig.json`, so the obvious escape is a distinct name at the
  root, such as `tsconfig.next.json`. That is worse than the collision it
  avoids: an editor's TypeScript server finds a project only by walking
  up from the open file to something called exactly `tsconfig.json`, so a
  renamed root file leaves every source in that layer with no
  configuration at all, and `strict` and the path aliases silently do not
  apply. Nothing fails, which is what makes it expensive to find. Where a
  framework insists on being told, point it at the scoped file:
  `frontend/next` sets `typescript.tsconfigPath` to
  `src/app/tsconfig.json`.
- **Never author a combination of layers.** The moment a special case
  exists for a specific pair, the combinatorial death has started. A kill
  criterion, not a style note.
- **A layer that contributes to another layer's file declares it in
  `requires`.** Established 2026-08-19. Contributing to `package.json`
  already meant requiring `toolchain/node-pnpm`; the same now holds for
  ESLint. `close_over_requirements` adds a requirement on a layer's
  behalf when the required layer is a toolchain or is already a default,
  because neither is a guess, and refuses when it was explicitly opted
  out rather than overriding the opt-out. Anything else stays an error,
  so requiring a database still cannot pick one for somebody.
- **A tool's ignore list is inherited from `.gitignore`, never
  restated.** Every framework layer already declares its own build output
  there. ESLint reads it through `includeIgnoreFile`, Prettier through
  `--ignore-path`. Restating `.next/` in a quality layer would make that
  layer aware of a framework, and forgetting to restate it is invisible:
  `lint` passes on a clean checkout and fails forever after the first
  build.
- **Flat config lints `.js`, `.mjs` and `.cjs` and nothing else.** A
  workspace of TypeScript with a stack-agnostic `eslint.config.mjs`
  reports success having examined one file, its own config. Each layer
  whose file types ESLint would skip ships an
  `eslint.<layer-id>.config.mjs` that claims them with `files` globs; the
  root config discovers those by filename rather than importing them, so
  no layer names another. Globs must stay scoped to the layer's own
  directories, which is what keeps fragments disjoint and their load
  order irrelevant.
- **Never emit configuration only culm's runner can consume.** Decision
  3.
- **Never put a version number in a layer template file.** Decision 5.
- **Never modify Klassivox unless explicitly told to**, naming it.
  Decision 13.
- **The runner is expected to possibly die at r0.2.** Planned, not
  failure. [02-runner.md](02-runner.md), section 10.
- **`typescript` is capped `<6` in every layer that adds it.** Discovered
  2026-09-05 proving `monorepo` builds end to end: unranged `typescript`
  resolved to `7.0.2`, and `typescript-eslint@8.69.0` refuses to run
  against TS 7. TypeScript jumped straight from 5.x to 7.x (the native
  compiler rewrite), so `<6` is not a stale cap, it names the whole 5.x
  line on purpose. Lift it once `typescript-eslint` supports TS 7.
- **`vitest` is capped `<5` in `frontend/nuxt`.** Found 2026-09-05 proving
  the `nuxt` script `test:nuxt`: `@nuxt/test-utils` only declares a peer
  on `^4.0.2`, and vitest 5 changed the `vitest/environments` export
  shape the `"nuxt"` test environment it registers depends on, so an
  unranged `vitest` breaks that environment outright. Lift once
  `@nuxt/test-utils` supports vitest 5.
- **`prisma` is capped `<8` in `data/prisma`.** Found 2026-09-07 proving
  `data/prisma` end to end: the `prisma` CLI's `latest` dist-tag is a
  `8.0.0-rc` pre-release of an entirely restructured CLI with no
  `generate`/`migrate`/`studio` commands, while `@prisma/client`'s `latest`
  is still on the 7.x line, so an unranged `prisma` pulls a CLI that cannot
  run this layer's own scripts. Prisma 7 also dropped `datasource.url` from
  `schema.prisma` in favour of `prisma.config.ts`, which the layer now ships.
  Lift the cap once a stable (non-`rc`) 8.x CLI ships.
- **A stock pnpm 11 install blocks `esbuild`'s postinstall script by
  default** (`ERR_PNPM_IGNORED_BUILDS`), which aborts culm's own
  `install::run` partway through on any layer selection that pulls it in
  transitively (Nuxt/Vite do). Not a culm bug, a pnpm default; the escape
  hatch is `pnpm approve-builds --all` or an `allowBuilds` entry in
  `pnpm-workspace.yaml`, run by hand in the generated workspace.
- **`infra/dockerfile`'s Nuxt container has a slow first start**, found
  verifying it against a real Docker daemon 2026-09-07: `corepack enable`
  at build time enables corepack but does not cache the `pnpm` binary
  itself into the image, so the first `docker compose up` spends 10-15
  seconds downloading pnpm fresh before `nuxt preview` runs. Not a
  correctness bug — every subsequent start is fast, and the build itself
  succeeds — just a cold-start cost nobody has decided is worth fixing
  yet. The fix, if it becomes worth doing: `RUN corepack prepare
  pnpm@<version> --activate` as its own build step, which decision 5 does
  not reach the same way it reaches a generated workspace's own
  dependencies, since this is a version of the tool the Dockerfile
  installs, not a package a generated project depends on.

---

## Context a fresh session will not have

- **Merged files are interpolated after the merge, not before it.**
  `Variables::render_files` runs over what `FileSet::render()` produces,
  which is what lets `toolchain/node-pnpm` own `name` in the root
  `package.json`: that value is only knowable at scaffold time, so no
  layer could have owned the key while merged content was written
  verbatim. Substituting after the merge rather than before it means two
  layers contributing the same placeholder still dedupe as identical
  text, so order independence is untouched. Until 2026-08-18 no merged
  file was interpolated at all and every generated Node workspace had an
  unnamed `package.json`.
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
