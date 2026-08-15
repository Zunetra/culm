# 01 - The Scaffolder

The v0.1 product. Ships first. See
[CONTEXT.md](CONTEXT.md#decisions) decisions 2, 3, 4, 5, and 6 for why the
shape below is what it is.

---

## 1. The problem

Standing up Klassivox required deciding on and wiring, by hand:

- pnpm catalog, `saveExact`, `minimumReleaseAge`, `onlyBuiltDependencies`
- eslint flat config, prettier, `.lintstagedrc`
- husky pre-commit and pre-push hooks
- `.nvmrc`, `tsconfig.base.json` path mapping, `vitest.workspace.ts`
- `.vscode/settings.json`, `extensions.json`, `launch.json`
- docker-compose plus a postgres init script, `.env.example`
- five separate Nx plugin blocks

None of that is Klassivox-specific. All of it is decisions a developer
makes badly at 11pm on day one and then lives with for two years.

**Nobody covers the polyglot case.** `create-nx-workspace` will not
produce a Cargo workspace, `rustfmt.toml`, `clippy.toml`, `cargo-deny`,
`cargo-nextest` in CI, and a pre-commit hook that runs `cargo fmt --check`
on staged `.rs` files while `lint-staged` handles staged `.ts` files.
`cargo-generate` will not produce the JavaScript half. Neither will,
because the JS tools are JS-first and the Rust tools are Rust-first.

**Who has this problem:** anyone starting a repository with more than one
language in it, and anyone who has started more than one repository and
noticed they configured the same twelve things twice, slightly
differently.

## 2. The layer model

A **layer** is a directory of files plus a manifest describing how it
merges into files other layers also touch. **No combination of layers is
ever authored by hand.** That is the entire design, and abandoning it is
how every scaffolder before this one died.

### Categories

| Category | Cardinality | Members |
| --- | --- | --- |
| Base | exactly one | `base` |
| Shape | exactly one | `monorepo`, `single-app`, `tooling-only` |
| Frontend | shape dependent | `nuxt`, `next`, `react-vite`, `vue-vite`, `astro`, `svelte` |
| Backend | shape dependent | `axum`, `actix`, `express`, `nestjs`, `fastify`, `go-chi` |
| Toolchain | implied, never asked | `node-pnpm`, `rust-cargo`, `go-mod` |
| Quality | defaults on, toggleable | `prettier`, `eslint`, `rustfmt`, `clippy`, `husky`, `commitlint`, `editorconfig` |
| Runner | exactly one | `nx`, `turborepo`, `moon`, `culm`, `none` |
| Infra | zero or more | `docker-compose`, `dockerfile`, `github-actions` |
| Editor | zero or more | `vscode`, `zed` |

Cardinality is declared on the category, never as pairwise `conflicts`
entries between layers, so changing a category is one edit rather than
one per member.

**Shape dependent** means what it says. A `single-app` workspace takes at
most one frontend and one backend, because one app runs on one framework.
A `monorepo` takes any number of each, because a Nuxt site beside a React
admin panel is the reason someone reaches for a monorepo. `tooling-only` takes no
framework layers at all and requires a toolchain instead, since it exists
to drop tooling onto a repository that already exists. A single app
additionally needs at least one of the two, a rule that spans categories
and therefore lives in validation. See [CONTEXT.md](CONTEXT.md#decisions)
decision 18.

A layer's identity is `<category>/<id>`, not the bare id. `none` is a
legitimate answer in more than one category, and the merge engine keys
line blocks by identity, so two layers called `none` would silently share
a block. The stronger reason is decision 6: presets travel by git URL, so
a third party will eventually ship a `nuxt` layer of their own.
`requires` and `conflicts` name qualified ids for the same reason.

The **base** layer applies to every workspace whatever else is selected.
It owns the ignores that belong to no framework, plus a committed
`.env.example`. Framework layers still ignore only their own build
output, which is what keeps them independent of one another.

### Why role, not language

The user never answers "which language." Picking `axum` implies Rust,
which implies `Cargo.toml`, which implies `rustfmt`, which implies
`cargo fmt --check` in the pre-commit hook. The framework choice already
contains the language, so asking for it separately is asking the user to
state the same fact twice and then giving them a way to contradict
themselves.

### What this buys

- Frontend only: `shape=single-app + nuxt`
- Backend only: `shape=single-app + axum`
- Polyglot monorepo: `shape=monorepo + nuxt + axum + docker-compose + github-actions`

Same engine. No extra authoring. Roughly 35 to 40 layers covers a
combination space in the thousands.

## 3. The layer manifest

```toml
# layers/quality/husky/layer.toml
id       = "husky"
category = "quality"
requires = ["node-pnpm"]           # cannot apply without these
conflicts = []                     # mutually exclusive layers
default  = true                    # on unless toggled off

# Files copied verbatim. Handlebars-style {{project_name}} interpolation.
[[files]]
from = "files/.husky/pre-commit"
to   = ".husky/pre-commit"
mode = 0o755

# Merges into a file another layer may also own.
[[merge]]
target   = "package.json"
strategy = "json-deep"
[merge.value]
scripts = { prepare = "husky" }

# Packages to install after scaffolding. No versions here, ever.
[[install]]
manager = "pnpm"
dev     = true
packages = ["husky", "lint-staged"]

# Contributions to shared hook scripts other layers also extend.
[[hook]]
name  = "pre-commit"
order = 20
body  = "pnpm lint-staged"
```

Two rules make this workable:

- **A layer declares intent, never final content**, for any file another
  layer might touch. `package.json` is never copied, only merged into.
- **`order` is explicit wherever sequence is observable**, such as hook
  bodies and CI steps. Determinism is not left to directory iteration
  order.

## 4. The merge engine

This is the actual product. Everything else copies files.

The problem: N independent layers each contribute fragments to the same
shared files, in any selection order, and the result must be identical
every time and must be a valid file.

### Strategies

| Target | Strategy | Notes |
| --- | --- | --- |
| `package.json` | `json-deep` | objects merge, arrays union then sort, scalar collision is an error |
| `Cargo.toml` | `toml-members` | append to `[workspace].members`, sort, dedupe |
| `pnpm-workspace.yaml` | `yaml-list` | union of globs, sorted |
| `tsconfig.base.json` | `json-deep` | `compilerOptions.paths` merged key by key |
| `.gitignore` | `line-block` | one `# layer:<id>` delimited block per layer, sorted by layer id |
| `.husky/*` | `hook-script` | shebang, then bodies sorted by `order` |
| CI yaml | `yaml-steps` | steps sorted by `order` inside their job |
| `.env.example` | `line-block` | same as gitignore |

### The rules that make it correct

- **Order independence.** Applying layers `{a, b, c}` in any permutation
  produces byte-identical output. This is a property test, not a hope.
- **Collisions are errors, never silent last-write-wins.** If two layers
  set `scripts.build` to different strings, the scaffold stops and names
  both layers. Silent resolution is how a generated workspace ends up
  subtly wrong in a way nobody traces back for a week.
- **Every write is atomic across the whole run.** Build the full file set
  in memory, validate it, then write. A scaffold that fails halfway
  leaves nothing behind.
- **Sorted output everywhere.** Not cosmetic. It is what makes order
  independence testable by byte comparison.

### Sketch

```rust
// === Merge

/// Layers are applied to a virtual file set, never to disk, until the
/// whole run has validated. A partial scaffold is worse than a failed
/// one, because a failed one leaves nothing to clean up.
pub struct FileSet {
    files: BTreeMap<PathBuf, FileContent>, // sorted: output must be deterministic
}

pub enum MergeError {
    /// Two layers set the same scalar to different values. Both are
    /// named in the message, because "conflict in package.json" without
    /// the culprits is useless to whoever hits it.
    ScalarConflict { target: PathBuf, key: String, a: LayerId, b: LayerId },
    MissingRequirement { layer: LayerId, requires: LayerId },
    Conflicting { a: LayerId, b: LayerId },
}
```

## 5. The wizard

### Order

1. **Project name.** Every subsequent file needs it.
2. **Shape.** Monorepo or single app. It changes the meaning of every
   later question.
3. **Frontend apps**, if any.
4. **Backend apps**, if any.
5. **Quality tooling**, defaults pre-selected.
6. **Runner**, defaulting to `nx` until culm's own runner passes its
   v0.2 check.
7. **Infra and editor.**

### Two hard rules

- **Every question has a flag equivalent.**
  `culm new myapp --preset backend --yes` must work with zero prompts.
  Interactive-only scaffolders are unusable in CI, in documentation, and
  in exactly the places a tool spreads from.
- **Print the resolved plan and confirm before writing anything.** "Here
  are the 14 files I will create and the 6 commands I will run." This
  costs nothing and matches the honesty posture in
  [CONTEXT.md](CONTEXT.md#decisions) decision 9.

## 6. Presets

A preset is a name, a layer list, and pre-answered questions.

```yaml
# culm.preset.yaml
name: polyglot-monorepo
description: Nuxt frontend, Axum backend, one workspace, full quality gate.
shape: monorepo
layers:
  - nuxt
  - axum
  - prettier
  - eslint
  - rustfmt
  - clippy
  - husky
  - commitlint
  - docker-compose
  - github-actions
  - vscode
runner: nx
answers:
  database: postgres
```

- **Shipped in the binary:** `monorepo`, `frontend`, `backend`.
- **From a git URL:** `culm new myapp --preset github:user/repo`.
- **Saved locally after a run:** written to
  `~/.config/culm/presets/<name>.yaml`.
- No hosted registry before v2. See
  [CONTEXT.md](CONTEXT.md#decisions) decision 6.

The `monorepo` preset is derived from Klassivox's real configuration,
which is the point: it is the one workspace we have perfect information
about.

## 7. Version resolution

Restating [CONTEXT.md](CONTEXT.md#decisions) decision 5 as behaviour:

1. Layers contain no version numbers.
2. After files are written, run `pnpm add -D <packages>` and
   `cargo add <crates>`, letting each package manager write versions and
   lockfiles.
3. Where a layer's config shape is bound to a major, it declares
   `compat = ">=9 <10"` and the newest release inside that range is
   chosen.
4. "Newest" excludes releases published within the last N days, matching
   the `minimumReleaseAge` supply-chain guard, which is a scaffolder
   default.

## 8. Nightly CI, and why it is not optional

Untested templates are the default state of every dead scaffolder. The
nightly job is the only real defence against `latest` breaking a layer.

```yaml
# .github/workflows/nightly-scaffold.yml (excerpt)
strategy:
  matrix:
    preset: [monorepo, frontend, backend]
    os: [ubuntu-latest, macos-latest]
steps:
  - run: culm new fixture --preset ${{ matrix.preset }} --yes
  - run: cd fixture && pnpm install --frozen-lockfile=false
  - run: cd fixture && pnpm build
  - run: cd fixture && pnpm lint
  - run: cd fixture && pnpm test
  # A generated workspace that lints clean but does not build is the
  # failure users actually hit, so build runs before lint is trusted.
```

Plus a property test on the merge engine: for a random selection of
compatible layers, applying them in a shuffled order produces
byte-identical output.

## 9. Milestones

**v0.1 (2 weeks).** Layer format, merge engine, `json-deep` and
`line-block` strategies, all six frontend layers per decision 18, three
first-party presets, non-interactive flags only. No wizard yet, and no
backend layers yet beyond what the Klassivox shape needs. Scaffolds the
Klassivox shape and the result builds.

Built so far: the merge engine with both strategies and order
independence proven by property test, and the layer format with
categories, manifest parsing, and selection validation. Frontend layers
are in progress. Not started: file copying, interpolation, presets, the
CLI.

**v0.2 (1 week).** Interactive wizard, plan preview and confirmation,
`toml-members` and `hook-script` and `yaml-steps` strategies.

**v0.3 (1 week).** Presets from git URLs, save-run-as-preset, telemetry
with the payload printed on first run.

**v0.4 (1 week).** Nightly CI matrix, property tests on merge order
independence, npm launcher package with platform binaries, install
script.

**v1.0.** Go layers, documentation site, and the number that matters:
how many people ran `culm new` and what they selected.

## 10. Kill criteria

Separate from the runner's, which stay exactly as written in
[02-runner.md](02-runner.md).

- **A generated workspace fails to build and nightly CI did not catch it
  before a user did.** The layer matrix is too wide. Cut layers until CI
  is trustworthy again. This is the sharp one.
- **The merge engine needs a special case for a specific pair of
  layers.** That is the combination-authoring trap reappearing in
  disguise, and it is the beginning of the death every previous
  scaffolder died. Either the strategy set is wrong, or that layer should
  not exist.
- **Nobody uses it.** If telemetry after three months of the tool being
  public shows scaffolds only from machines that are ours, the runner has
  no audience either, and that was the entire reason for building this
  half first. Stop, and count the month well spent.
- **Writing a layer takes longer than writing the config by hand.** The
  layer format has become a programming language, which means it is
  competing with shell scripts and losing.

## 11. Crates

```toml
[dependencies]
clap        = { version = "4", features = ["derive", "env"] }
serde       = { version = "1", features = ["derive"] }
serde_json  = "1"
serde_yaml  = "0.9"
toml_edit   = "0.22"          # format-preserving TOML, required for Cargo.toml merges
handlebars  = "6"             # {{project_name}} interpolation in template files
inquire     = "0.7"           # the wizard
console     = "0.15"          # plan preview rendering
ignore      = "0.4"           # gitignore-aware walking, the ripgrep crate
thiserror   = "2"
anyhow      = "1"
tracing     = "0.1"

[dev-dependencies]
tempfile    = "3"
insta       = "1"             # snapshot the generated file set
proptest    = "1"             # merge order independence
```

`toml_edit` rather than `toml` because merging into a user's
`Cargo.toml` must preserve their comments and formatting. A scaffolder
that reformats a file it touches is a scaffolder people stop trusting
with existing repositories.
