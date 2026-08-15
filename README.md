# culm

**culm** is the stalk of a grain plant, the structure the grain grows on.
Four characters, plain ASCII, no diacritics: the folder, the repository,
the binary, the npm package, the crate, and the domain are all `culm`.

Workspace tooling for polyglot repositories. One binary that scaffolds a
workspace and then runs its tasks.

![Rust](https://img.shields.io/badge/rust-edition%202024-orange)
![Version](https://img.shields.io/badge/version-0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)

> **Status: pre-alpha, library only.** The layer format, the merge
> engine, and the scaffold writer exist and are tested. `src/main.rs` is
> still a hello-world stub, so **there is no working `culm` command
> yet**. Nothing in this README describes a CLI you can run today. See
> [Roadmap](#-roadmap) for what is built and what is not.

---

## 📖 Table of contents

- [What it is](#-what-it-is)
- [The gap it exists to fill](#-the-gap-it-exists-to-fill)
- [Features](#-features)
- [Project structure](#-project-structure)
- [The layer model](#-the-layer-model)
- [Shipped layers](#-shipped-layers)
- [The merge engine](#-the-merge-engine)
- [Scaffolding](#-scaffolding)
- [Authoring a layer](#-authoring-a-layer)
- [Using the library](#-using-the-library)
- [Development](#-development)
- [Distribution](#-distribution)
- [Roadmap](#-roadmap)
- [Troubleshooting](#-troubleshooting)
- [Contributing](#-contributing)
- [Documents](#-documents)
- [Licence](#-licence)

## 🌱 What it is

culm is two halves of one tool, shipped as one binary rather than two
products. That split is [decision 1](docs/CONTEXT.md#decisions).

**The scaffolder** stands up a workspace so a developer never again
hand-wires prettier, eslint, rustfmt, clippy, tsconfig paths, ignore
files, and a workspace manifest at 11pm on day one.

**The runner** knows what changed, skips what it can, and caches the rest
correctly across a repository that is genuinely more than one language.

They are different in every commercial dimension. The scaffolder runs
once per project and has no catastrophic failure mode. The runner runs
hundreds of times a day and its worst failure is a silent wrong cache hit
that ships broken code. They ship as one tool because the scaffolder is
the acquisition channel and the runner is what retains.

**The scaffolder ships first**, per [decision 2](docs/CONTEXT.md#decisions). It
is cheaper, lower risk, and it is how anyone finds the runner. The runner
is designed in [docs/02-runner.md](docs/02-runner.md) and none of it is
built.

## 🕳️ The gap it exists to fill

`create-nx-workspace` will not give you a Cargo workspace, a
`rustfmt.toml`, a `clippy.toml`, and a pre-commit hook that runs
`cargo fmt --check` on staged `.rs` files *and* `lint-staged` on staged
`.ts` files. `cargo-generate` will not give you the JavaScript half.
Neither will, because the JS tools are JS-first and the Rust tools are
Rust-first.

The same split shows up in the runner. A repository with a root
`Cargo.toml` next to a `pnpm-workspace.yaml` has two dependency graphs
and two lockfiles, and every existing task runner treats one of them as
the real one and the other as a plugin.

That is the whole thesis: **be correct about the polyglot case, which is
the case everyone else bolted on afterwards.**

## ✨ Features

Everything in this section is implemented in `src/` and covered by tests.

### 🧩 The layer format

- **A layer is `<category>/<id>`**, a directory holding a `layer.toml`
  plus the files that layer alone owns.
- **Ten categories**, declared once in `src/layer/category.rs`, not
  spread across manifests.
- **Cardinality is a property of the category and depends on the
  workspace shape**, so `single-app`, `monorepo`, and `tooling-only` get
  different rules from one table.
- **Strict manifest parsing.** `deny_unknown_fields` is on, so a typo
  such as `defualt = true` is a parse error rather than a silently
  ignored key.
- **Strategy-tagged merges**, so a `[[merge]]` entry cannot declare one
  strategy and carry another's payload.

### 🔀 The merge engine

- **Three strategies**: `json-deep`, `line-block`, and `toml-members`.
- **Order independence**, proven by a proptest over every permutation of
  a generated layer set, not asserted in prose.
- **Collisions are errors, never last-write-wins**, and every error names
  both culprit layers and the dotted key.
- **Provenance tracking**, so a deep conflict names the layer that set
  the offending leaf rather than the layer that happened to be applied
  first.
- **Failed merges leave the document untouched**, matching the
  all-or-nothing rule for the run as a whole.
- **A total order over JSON values** that never reports `Equal` for
  values that differ, including integers beyond `f64` precision.

### 🏗️ Scaffolding

- **A copy plan built entirely in memory** and validated before anything
  touches disk.
- **Path containment.** A destination that is absolute, rooted, or climbs
  out with `..` is refused, because a layer is data and data must not be
  able to write outside the directory it was pointed at.
- **`{{name}}` interpolation** over a closed set of variables, where an
  unknown placeholder is left literal rather than blanked.
- **Identical content from two layers is not a conflict**; differing
  content at the same destination is.
- **Unix file modes** are applied where declared, and ignored on
  platforms with no POSIX permission bits.

### 🔒 Safety posture

- `#![forbid(unsafe_code)]` in both `src/lib.rs` and `src/main.rs`, so it
  is enforced by the compiler rather than by a document asking nicely.
- Library errors are `thiserror` types. `anyhow` is reserved for the
  binary boundary that does not exist yet.
- No version numbers anywhere in a layer template file. Package managers
  write versions, per [decision 5](docs/CONTEXT.md#decisions), and a test fails
  if a shipped `[[install]]` entry carries one.

## 📁 Project structure

```
culm/
├── src/
│   ├── lib.rs                     # Crate root, forbids unsafe, declares the three modules
│   ├── main.rs                    # Binary entry point, still a hello-world stub
│   ├── layer/                     # The layer format
│   │   ├── mod.rs                 # Declares and re-exports, holds no logic
│   │   ├── category.rs            # The ten categories, Shape, and shape-dependent cardinality
│   │   ├── manifest.rs            # layer.toml parsing, and conversion to the merge engine's Layer
│   │   ├── catalog.rs             # Walks a directory tree and loads every layer.toml it finds
│   │   ├── resolve.rs             # Expands what the user chose into the complete legal selection
│   │   ├── selection.rs           # The questions both validate and resolve ask about a selection
│   │   ├── validate.rs            # Checks a selection against every category's cardinality
│   │   └── error.rs               # LayerError, the module's public contract for failure
│   ├── merge/                     # The merge engine
│   │   ├── mod.rs                 # LayerId, Strategy, Contribution, FileSet, MergeError
│   │   ├── json_deep.rs           # Recursive object merge, array union, provenance, total ordering
│   │   ├── line_block.rs          # One `# layer:<id>` delimited block per layer
│   │   └── toml_members.rs        # A Cargo workspace members array, unioned and sorted
│   └── scaffold/                  # Copying a layer's own files
│       ├── mod.rs                 # Declares and re-exports
│       ├── plan.rs                # CopyPlan, PlannedFile, and destination normalisation
│       ├── interpolate.rs         # {{name}} substitution over a closed variable set
│       ├── write.rs               # The only function that touches disk
│       └── error.rs               # ScaffoldError
├── layers/                        # The shipped layer catalog, one directory per layer
│   ├── base/base/                 # Ignores that belong to no framework, plus .env.example
│   ├── shape/                     # monorepo, single-app, tooling-only
│   ├── frontend/                  # astro, next, nuxt, react-vite, svelte, vue-vite
│   ├── backend/                   # actix, axum, express, fastify, nestjs
│   ├── data/                      # drizzle, prisma, sqlx
│   ├── toolchain/                 # node-pnpm, rust-cargo
│   ├── quality/                   # clippy, eslint, prettier, rustfmt
│   └── runner/                    # none, nx
├── tests/
│   ├── layers.rs                  # Exercises every shipped layer, discovered by walking layers/
│   ├── scaffold.rs                # End-to-end scaffolds into a temporary directory
│   ├── order_independence.rs      # The proptest: any permutation renders identical bytes
│   └── order_independence.proptest-regressions  # Seeds for failures already found
├── docs/
│   ├── CONTEXT.md                 # Decisions, conventions, and context worth keeping. Read first.
│   ├── 01-scaffolder.md           # The v0.1 product design
│   ├── 02-runner.md               # The second half, higher risk, not built
│   └── TELEMETRY.md               # The exact payload, published verbatim
├── Cargo.toml                     # Single package, edition 2024
└── LICENSE                        # MIT
```

Two layout rules, both from [docs/CONTEXT.md](docs/CONTEXT.md):

- **culm is a single crate**, not a Cargo workspace, and has no `crates/`
  directory. Splitting a module into a crate later is mechanical.
  Un-splitting a premature workspace is not.
- **A module is a directory with a `mod.rs`**, and that `mod.rs` declares
  and re-exports and holds no logic of its own.

## 🧱 The layer model

A **layer** is a directory of files plus a manifest describing how it
merges into files other layers also touch. **No combination of layers is
ever authored by hand.** That is the entire design, and abandoning it is
how every scaffolder before this one died.

### Identity

A layer's identity is **`<category>/<id>`**, never the bare id.

- `none` is a legitimate answer in more than one category, and the merge
  engine keys line blocks by identity, so two layers called `none` would
  silently share a block.
- Presets travel by git URL, so a third party will eventually ship a
  `nuxt` layer that is not ours.

`requires` and `conflicts` name qualified ids for the same reason.

### Categories and cardinality

Cardinality is declared on the category and depends on the workspace
shape, because "which framework does this app use" and "how many apps are
there" are two different questions a flat per-category rule cannot tell
apart.

| Category | `single-app` | `monorepo` | `tooling-only` |
| --- | --- | --- | --- |
| `base` | exactly one | exactly one | exactly one |
| `shape` | exactly one | exactly one | exactly one |
| `frontend` | zero or one | zero or more | **none** |
| `backend` | zero or one | zero or more | **none** |
| `data` | zero or one | zero or more | **none** |
| `toolchain` | zero or more | zero or more | **one or more** |
| `quality` | zero or more | zero or more | zero or more |
| `runner` | exactly one | exactly one | exactly one |
| `infra` | zero or more | zero or more | zero or more |
| `editor` | zero or more | zero or more | zero or more |

One extra rule spans two categories and therefore lives in validation
rather than in a cardinality: **a `single-app` selection needs at least
one frontend or backend.** A single app with neither is not a workspace,
it is a bare toolchain.

### Why the shapes differ

- **`single-app`** holds one app, and one app runs on one framework.
- **`monorepo`** holds many, and a Nuxt marketing site beside a React
  admin panel, or a Rust service beside a Node BFF, is the reason someone
  reaches for a monorepo at all.
- **`tooling-only`** takes no framework layers, because a framework layer
  scaffolds an app and an app implies a shape. Naming both is a
  contradiction, and it is rejected rather than half-scaffolded. The
  toolchain flips from implied to required here, since with no framework
  there is nothing left to imply it.

`tooling-only` is named for what it gives you rather than for what it
withholds. A shape called `none` tells the user nothing.

### Why role, not language

The user never answers "which language". Picking `backend/axum` implies
Rust, which implies `toolchain/rust-cargo`, which implies `rustfmt` and
`clippy`. The framework choice already contains the language, so asking
separately is asking the user to state the same fact twice and then
giving them a way to contradict themselves.

### Selection errors

`validate_selection` returns a named `LayerError` rather than a bare
boolean. The variants that a selection can produce:

| Error | When |
| --- | --- |
| `ShapeMissing` | No `shape` layer in the selection |
| `UnknownShape` | A `shape` layer whose id is none of the three |
| `CategoryEmpty` | A category needing at least one layer has none |
| `CategoryForbidden` | A layer in a category the shape forbids |
| `CategoryOverfull` | More than one layer in an at-most-one category |
| `SingleAppHasNoApp` | `single-app` with no frontend and no backend |
| `DuplicateId` | The same qualified id twice in one selection |
| `MissingRequirement` | A `requires` entry nothing in the selection provides |
| `Conflicting` | Two selected layers declare each other exclusive |

### Resolution

`resolve` turns what the user asked for into the complete legal
selection, then runs it through `validate_selection` so a caller cannot
forget to check it. The rules, in order:

1. **Requirements are closed over, but only a toolchain is added on a
   layer's behalf.** Node and pnpm follow any JS app and Rust and cargo
   follow any Rust one, and both answers are forced, so asking would be
   theatre. Any other unmet requirement stays an error, because quietly
   choosing somebody's database for them is a guess dressed up as an
   implication.
2. **The shape is settled next**, falling back to the layer marked
   `default` in the `shape` category. Cardinality depends on the shape,
   so nothing else can be tested until it is known.
3. **Defaults are added**, but only where the category has a free slot,
   the layer's own requirements are already met, and it conflicts with
   nothing selected. This is what makes quality tooling follow the
   language: `quality/prettier` requires `toolchain/node-pnpm`, so it
   appears in a JS workspace and stays away from a Rust-only one without
   a single rule naming either.
4. **A runner is filled in** if none was chosen. No runner layer is a
   default, because the right answer is not a property of the runner:
   `runner/nx` requires `toolchain/node-pnpm`, so a Rust-only workspace
   has exactly one legal runner and it is `runner/none`.
5. **Conflicts are checked symmetrically**, so one layer declaring the
   pair is enough and the two manifests cannot drift.

Explicit choices outrank opt-outs. Opting out only suppresses a layer
that would have arrived as a default.

## 📚 Shipped layers

Twenty-six layers live under `layers/`. Every one of them is loaded,
parsed, and merge-tested by `tests/layers.rs`, which discovers them by
walking the tree rather than reading a list, so a layer added tomorrow is
covered without anybody remembering to register it.

### base (1)

| Layer | Default | What it contributes |
| --- | --- | --- |
| `base/base` | ✅ | The ignores that belong to no framework, plus a committed `.env.example` |

Deliberately short. It carries OS and editor noise, `.env`, and `*.log`,
and no framework's build output, because each framework layer ignores its
own. A generated ignore file full of rules for tools the project does not
use teaches people to stop reading it.

### shape (3)

| Layer | Default | What it contributes |
| --- | --- | --- |
| `shape/single-app` | ✅ | Nothing. Its job is to select the cardinality rules |
| `shape/monorepo` | | Nothing, same reason |
| `shape/tooling-only` | | Nothing, same reason |

### frontend (6)

Each contributes namespaced `package.json` scripts plus its own ignore
lines, and requires `toolchain/node-pnpm`.

| Layer | Scripts | Ignores |
| --- | --- | --- |
| `frontend/astro` | `dev:astro`, `build:astro`, `preview:astro`, `check:astro` | `dist/`, `.astro/` |
| `frontend/next` | `dev:next`, `build:next`, `start:next`, `lint:next` | `.next/`, `out/`, `next-env.d.ts`, `.vercel/` |
| `frontend/nuxt` | `dev:nuxt`, `build:nuxt`, `preview:nuxt`, `generate:nuxt` | `.nuxt/`, `.output/`, `.data/`, `dist/` |
| `frontend/react-vite` | `dev:react-vite`, `build:react-vite`, `preview:react-vite` | `dist/`, `vite.config.ts.timestamp-*` |
| `frontend/svelte` | `dev:svelte`, `build:svelte`, `preview:svelte`, `check:svelte` | `.svelte-kit/`, `build/`, `dist/` |
| `frontend/vue-vite` | `dev:vue-vite`, `build:vue-vite`, `preview:vue-vite` | `dist/`, `vite.config.ts.timestamp-*` |

Every script is suffixed with the layer id. A monorepo may hold several
frontends, and two layers writing different values to `scripts.dev` is a
hard merge error rather than something to resolve silently.

`frontend/nuxt` deliberately does **not** contribute `postinstall`. That
key has to keep its literal name to work, so it cannot be namespaced, and
a second framework needing its own postinstall step would collide on it.

### backend (5)

| Layer | Requires | Notes |
| --- | --- | --- |
| `backend/actix` | `toolchain/rust-cargo` | Binary at `src/bin/actix.rs` |
| `backend/axum` | `toolchain/rust-cargo` | Binary at `src/bin/axum.rs` |
| `backend/express` | `toolchain/node-pnpm` | Server at `src/express/server.js` |
| `backend/fastify` | `toolchain/node-pnpm` | Server at `src/fastify/server.js` |
| `backend/nestjs` | `toolchain/node-pnpm` | Sources under `src/nestjs/`, plus `nest-cli.json` |

The two Rust backends put their entry point under `src/bin/` rather than
at `src/main.rs`, so a second Rust service in the same workspace has a
target of its own instead of a collision. Neither touches `package.json`,
because a Rust-only workspace has none and inventing one would leave a
JavaScript project behind for nobody to run.

The Node backends use CommonJS and contribute no root `type` key. That
key is a whole-workspace switch, and one app's module system is not a
decision a single layer gets to make for every other app beside it.

`backend/nestjs` installs no HTTP platform adapter. Which server sits
underneath belongs to whoever selects the layer.

### data (3)

Data is its own category because it is chosen *alongside* a backend
rather than instead of one. Nest with Prisma is an ordinary pairing.

| Layer | Requires | Contributes |
| --- | --- | --- |
| `data/drizzle` | `toolchain/node-pnpm` | `generate:drizzle`, `migrate:drizzle`, `studio:drizzle`, config and schema files |
| `data/prisma` | `toolchain/node-pnpm` | `generate:prisma`, `migrate:prisma`, `studio:prisma`, `prisma/schema.prisma` |
| `data/sqlx` | `toolchain/rust-cargo` | `migrations/0001_init.sql` |

All three add a `DATABASE_URL` line to `.env.example` rather than
defaulting one. A committed file holding a working credential is how a
real one ends up in a repository.

### toolchain (2)

| Layer | Contributes |
| --- | --- |
| `toolchain/node-pnpm` | `private: true` in `package.json`, plus `node_modules/`, `.pnpm-store/`, `.npm/` |
| `toolchain/rust-cargo` | `/target`, `**/*.rs.bk`, `*.pdb` |

`private` is the one root key every JS layer can agree on, and they all
agree on the same value, so it merges rather than conflicts.

`toolchain/rust-cargo` contributes no `Cargo.toml`. Whether the
repository is one crate or many is the shape layer's business, and
guessing wrong would clobber a real manifest in an existing repository.

### quality (4)

| Layer | Default | Requires | Contributes |
| --- | --- | --- | --- |
| `quality/prettier` | ✅ | `toolchain/node-pnpm` | `format`, `format:check`, `.prettierrc`, `.prettierignore` |
| `quality/eslint` | ✅ | `toolchain/node-pnpm` | `lint`, `lint:fix`, `eslint.config.mjs`, `.eslintcache` ignore |
| `quality/rustfmt` | ✅ | `toolchain/rust-cargo` | `rustfmt.toml` |
| `quality/clippy` | ✅ | `toolchain/rust-cargo` | `clippy.toml` |

Quality scripts are **not** namespaced, unlike the frontend ones. At most
one of each quality layer is selected, so `format` has a single owner and
cannot collide the way `dev` would across several frontends.

`quality/eslint` ships flat config only. `.eslintignore` was removed in
flat config, so ignores live inside `eslint.config.mjs`.

`quality/rustfmt` and `quality/clippy` contribute a config file and no
script, because Cargo already ships the command and there is no
`package.json` in a Rust-only repository to hang one on.

### runner (2)

| Layer | Requires | Contributes |
| --- | --- | --- |
| `runner/nx` | `toolchain/node-pnpm` | `.nx/cache` and `.nx/workspace-data` ignores, installs `nx` |
| `runner/none` | | Nothing, and that is the point |

`runner/none` has to be a real selectable answer rather than an absence
the validator rejects, because [decision 3](docs/CONTEXT.md#decisions) says the
scaffolder must be able to emit somebody else's runner or nothing at all.
If the scaffolder ever emits configuration only culm's runner can
consume, the entire argument for building the scaffolder first collapses.

### Not yet shipped

`infra` and `editor` are declared categories with no members yet. Both
are `zero or more` under every shape, so a selection is legal without
them.

## 🔀 The merge engine

This is the actual product. Everything else copies files.

The problem: N independent layers each contribute fragments to the same
shared files, in any selection order, and the result must be identical
every time and must be a valid file.

```
  layers/frontend/nuxt/layer.toml ─┐
  layers/quality/eslint/layer.toml ├─→  Layer  ─→  FileSet  ─→  render()  ─→  bytes
  layers/toolchain/node-pnpm/…     ─┘   (merges)    (in memory)              (on disk, once)
                                            │
                                            └─→  MergeError, naming both layers
```

Nothing reaches disk until the whole run has validated. A partial
scaffold is worse than a failed one: a failed one leaves nothing to clean
up.

### Strategies

| Strategy | Typical target | Behaviour |
| --- | --- | --- |
| `json-deep` | `package.json`, `tsconfig.base.json` | Objects merge recursively, arrays union then sort then dedupe, matching scalars agree, differing scalars are an error |
| `line-block` | `.gitignore`, `.env.example` | One `# layer:<id>` delimited block per layer, blocks sorted by layer id, lines sorted within a block |
| `toml-members` | `Cargo.toml` | Crate paths unioned into `[workspace].members`, sorted, deduped |

A target is claimed by exactly one strategy. A second layer claiming it
under another is a `StrategyMismatch` error rather than a guess.

### Rendered output

`json-deep` renders pretty-printed JSON with keys sorted, ending in a
newline:

```json
{
  "name": "app",
  "version": "0.1.0"
}
```

`line-block` renders one labelled block per layer, blank line separated:

```
# layer:toolchain/node-pnpm
.npm/
.pnpm-store/
node_modules/

# layer:toolchain/rust-cargo
*.pdb
**/*.rs.bk
/target
```

`toml-members` renders the workspace table with a fixed resolver:

```toml
[workspace]
resolver = "3"
members = [
    "app",
    "common",
]
```

`resolver` is fixed rather than contributed, because it follows from the
edition, and a layer choosing it would be a layer deciding something for
the whole workspace.

### Why collisions are errors

If two layers set `scripts.build` to different strings, the scaffold
stops and names both. It does not pick one.

Last-write-wins would make the output depend on selection order, which
destroys the one property this engine exists to guarantee. Worse, it is
how a generated workspace ends up subtly wrong in a way nobody traces
back for a week. The same reasoning governs `FileCollision` in the
scaffold module: two layers writing different content to one destination
is an error, while two layers writing identical content is not, because a
shared file written twice is the same file either way.

Set-shaped merges are the exception, and only where the merge genuinely
is a set. Two layers naming the same Cargo member is agreement, because a
member list is a set. Two layers naming the same array element is
agreement, because arrays are unioned. A scalar has no such reading.

### Merge errors

| Error | Meaning |
| --- | --- |
| `ScalarConflict` | Two layers set the same key to different values. Names both layers and the dotted key |
| `TypeConflict` | Two layers put incompatible kinds at the same path, such as an object against a string |
| `StrategyMismatch` | Two layers claim one target under different strategies |
| `NonObjectRoot` | A `json-deep` contribution whose root is not an object |
| `MissingRequirement` | A selected layer needs a layer that was not selected |
| `Conflicting` | Two selected layers declare each other mutually exclusive |

Every conflict variant names both culprits, because "conflict in
package.json" without the culprits is useless to whoever hits it. The
conflict report is also order independent: the reported pair is sorted,
so the message does not change with selection order.

### Provenance

`json-deep` records which layer set each node, keyed by dotted path, and
records every node of a subtree inserted in one go. Without that, a deep
conflict names whichever layer owns the ancestor rather than the layer
that actually set the offending leaf.

The regression test for this is
`deep_conflict_names_the_layer_that_set_the_leaf`.

### Order independence

**Applying layers `{a, b, c}` in any permutation produces byte-identical
output.** This is a property test, not a hope.

`tests/order_independence.rs` generates two to five layers, builds every
permutation of them, renders each, and asserts the bytes match. A second
property asserts that every layer actually reaches the output, so the
first property cannot be satisfied by rendering nothing.

Sorted output everywhere is what makes this testable by byte comparison,
which is why `FileSet` uses a `BTreeMap` and `LineBlockDoc` keys its
blocks by layer id.

### The total order invariant, learned the hard way

Any comparator the engine sorts with must be a **total order consistent
with equality**. It may report `Equal` only when the values genuinely are
equal.

This was violated once. Array entries are sorted so output does not
depend on layer order, and the comparator ordered JSON numbers by
converting both to `f64`. An `f64` carries 53 bits of precision, so
integers above 2^53 no longer map one to one, and distinct integers
collapsed onto the same `f64`. The comparator then reported `Equal` for
numbers that were not equal, Rust's `sort_by` is stable, so it left them
in arrival order, and arrival order is layer order. The rendered bytes
changed depending on which layer was applied first.

The fix breaks the tie on the exact textual form of the number. The
regression test is
`integers_beyond_f64_precision_still_sort_deterministically`.

Two lessons worth carrying:

- The property test did not catch it. Its generators drew from small
  value pools and could never produce two numbers that collapse. **A
  property test is only as strong as the inputs it can reach.**
- The failure was silent, and silent wrong output is the same category as
  the runner's worst failure mode.

## 🏗️ Scaffolding

The merge engine decides what shared files contain. The `scaffold` module
decides what private files a layer copies and where they land. They are
deliberately separate, because conflating them would put path safety and
template interpolation inside the one module whose correctness property
is byte-for-byte determinism.

### The pipeline

```
  Manifest ─→ CopyPlan::build ─→ CopyPlan ─┐
                (reads files/,             ├─→ scaffold::write(root, plan, merged)
                 interpolates,             │        (creates dirs, writes, chmods)
                 normalises paths)         │
  Layer ────→ FileSet::build ──→ render() ─┘
```

### Interpolation

Templates use `{{name}}`. Whitespace inside the braces is tolerated, so
`{{ project_name }}` and `{{project_name}}` are the same placeholder.

| Input | Variables | Output |
| --- | --- | --- |
| `name = "{{project_name}}"` | `project_name=klassivox` | `name = "klassivox"` |
| `{{ project_name }}` | `project_name=klassivox` | `klassivox` |
| `{{typo}}` | `project_name=klassivox` | `{{typo}}`, left literal |
| `fn main() {{ let x = 1;` | any | unchanged, an unterminated `{{` is content |

An unknown placeholder is left exactly as written rather than blanked. A
silently empty string in a generated config is far harder to notice than
a literal `{{typo}}`.

The variable set is closed rather than arbitrary user data, so a template
cannot come to depend on something the wizard does not ask.

### Path containment

A layer is data, and a scaffolder that lets data write outside the
directory it was pointed at is a scaffolder that can overwrite anything
the user can. Destinations are normalised before use:

| Destination | Result |
| --- | --- |
| `src/main.rs` | `src/main.rs` |
| `./bacon.toml` | `bacon.toml` |
| `../../.ssh/authorized_keys` | refused, `EscapesRoot` |
| `src/../../etc/passwd` | refused, `EscapesRoot` |
| `/etc/passwd` | refused, `EscapesRoot` |
| `` or `./` | refused, `EscapesRoot` |

### Scaffold errors

| Error | Meaning |
| --- | --- |
| `TemplateMissing` | A `[[files]]` entry names a file the layer directory does not hold |
| `FileCollision` | Two layers write different content to one destination |
| `EscapesRoot` | A destination that is absolute, rooted, or climbs out |
| `Io` | The write itself failed, reported with the path |

## 📝 Authoring a layer

A layer is a directory under `layers/<category>/<id>/` holding a
`layer.toml` and, optionally, a `files/` directory.

### Full `layer.toml` reference

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `id` | string | ✅ | The bare id. Must match the directory name |
| `category` | string | ✅ | One of the ten categories, kebab-case |
| `requires` | array of string | | Qualified ids this layer cannot apply without |
| `conflicts` | array of string | | Qualified ids this layer is mutually exclusive with |
| `default` | bool | | Selected unless opted out, subject to the resolution rules |
| `[[files]]` | table array | | Files copied verbatim into the workspace |
| `[[merge]]` | table array | | Contributions to files other layers may also own |
| `[[install]]` | table array | | Packages to install after scaffolding |

`[[files]]` entries:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `from` | path | ✅ | Relative to the layer's own directory |
| `to` | path | ✅ | Relative to the workspace root. Cannot escape it |
| `mode` | integer | | POSIX permission bits, ignored on non-unix |

`[[merge]]` entries are tagged on `strategy`, and each strategy takes a
different payload key:

| `strategy` | Payload key | Type |
| --- | --- | --- |
| `json-deep` | `value` | table, whose root must be a table |
| `line-block` | `lines` | array of string |
| `toml-members` | `members` | array of string |

Every entry also takes `target`, the path relative to the workspace root.

`[[install]]` entries:

| Field | Type | Required | Meaning |
| --- | --- | --- | --- |
| `manager` | string | ✅ | `pnpm`, `cargo`, and so on |
| `dev` | bool | | Whether it is a development dependency |
| `packages` | array of string | ✅ | Package names, **never with a version** |

Unknown fields are rejected. `deny_unknown_fields` is on for every one of
these structures, so `defualt = true` is a parse error rather than a
silently ignored key.

### A worked example

```toml
# layers/frontend/nuxt/layer.toml

# Nuxt, the Vue meta framework.

id = "nuxt"
category = "frontend"
requires = ["toolchain/node-pnpm"]
default = false

# Scripts are suffixed with the layer id because a monorepo may hold
# several frontends, and two layers writing different values to
# `scripts.dev` is a hard merge error rather than a last-one-wins.
[[merge]]
strategy = "json-deep"
target = "package.json"
[merge.value.scripts]
"dev:nuxt" = "nuxt dev"
"build:nuxt" = "nuxt build"
"preview:nuxt" = "nuxt preview"
"generate:nuxt" = "nuxt generate"

# Only this framework's own build output. The OS and editor noise is the
# base layer's job, and duplicating it here would emit two blocks.
[[merge]]
strategy = "line-block"
target = ".gitignore"
lines = [".nuxt/", ".output/", ".data/", "dist/"]

# No versions. Package managers write versions, per decision 5.
[[install]]
manager = "pnpm"
dev = false
packages = ["vue", "vue-router"]

[[install]]
manager = "pnpm"
dev = true
packages = ["nuxt", "typescript"]
```

### Rules a new layer has to follow

These are enforced by `tests/layers.rs`, so breaking one fails the build
rather than surviving to a release.

1. **`id` matches the directory name**, and the layer sits in the
   directory named for its category.
2. **No version numbers in any `[[install]]` package.** The check has its
   own test proving the check can actually fail, so it cannot rot into a
   no-op.
3. **A layer never names another layer in its content.** Not in a package
   name, not in a script, not in an ignore rule. The moment a special
   case exists for a specific pair of layers, the combinatorial death has
   started. This is a kill criterion, not a style note.
4. **Every declared template file exists** in the layer directory.
5. **Scripts in a many-per-workspace category are namespaced by layer
   id.** Quality layers are not, because at most one of each is selected.
6. **Every quality, backend, and data layer requires a toolchain**, which
   is what makes tooling follow the language without a rule naming
   either.
7. **Every pair of frontends, and every pair of backends, merges
   cleanly**, since a monorepo may select several.
8. **A file only one layer touches is copied. A file several might touch
   is merged.** Copying a shared file would let one layer clobber
   another.

### Adding a layer

```bash
mkdir -p layers/frontend/solid/files
$EDITOR layers/frontend/solid/layer.toml

# The tests discover it by walking the tree. Nothing to register.
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

## 🧪 Using the library

Until the CLI exists, culm is consumed as a library. The public surface
is `culm::layer`, `culm::merge`, and `culm::scaffold`.

Merging a selection:

```rust
use culm::merge::{Contribution, FileSet, Layer};
use serde_json::json;

let node = Layer::new("toolchain/node-pnpm")
    .with_merge("package.json", Contribution::JsonDeep(json!({ "private": true })))
    .with_merge(
        ".gitignore",
        Contribution::LineBlock(vec!["node_modules/".to_owned()]),
    );

let nuxt = Layer::new("frontend/nuxt")
    .requiring("toolchain/node-pnpm")
    .with_merge(
        "package.json",
        Contribution::JsonDeep(json!({ "scripts": { "dev:nuxt": "nuxt dev" } })),
    );

let set = FileSet::build([&node, &nuxt])?;
for (path, body) in set.render() {
    println!("--- {}", path.display());
    print!("{body}");
}
```

Parsing and validating a selection of real manifests:

```rust
use culm::layer::{Manifest, validate_selection};

let manifest = Manifest::parse("layers/frontend/nuxt/layer.toml", source)?;
assert_eq!(manifest.qualified_id().as_str(), "frontend/nuxt");

validate_selection(&selection)?;
```

`FileSet::build` validates requirements and conflicts across the whole
selection before anything merges, so a doomed run cannot produce half a
file set.

## 🛠️ Development

### Prerequisites

- **Rust** with edition 2024 support, installed via
  [rustup](https://rustup.rs/)
- Nothing else. There are no system dependencies, no database, and no
  container

### Commands

| Command | What it does |
| --- | --- |
| `cargo build` | Builds the library and the stub binary |
| `cargo test` | Runs unit tests beside each module plus the integration tests |
| `cargo test --test layers` | Only the shipped-layer suite |
| `cargo test --test order_independence` | Only the merge proptest |
| `cargo test --test scaffold` | Only the end-to-end scaffold tests |
| `cargo clippy --all-targets -- -D warnings` | **The gate. Must pass before any commit** |
| `cargo fmt` | Formats. `rustfmt.toml` stays at defaults |
| `cargo fmt --check` | Fails instead of rewriting, for CI |

### Before every commit

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

`cargo clippy --all-targets -- -D warnings` is not advisory. It is the
gate, and `--all-targets` matters because test code is where warnings
otherwise accumulate unseen.

### Test layout

- **Unit tests sit beside the code they test**, in a `#[cfg(test)] mod
  tests` under a `// === Tests` heading.
- **Cross-module behaviour goes in `tests/`**, because that is where the
  crate is exercised the way a caller would use it.
- `tests/order_independence.proptest-regressions` holds seeds for
  failures proptest has already found. It is committed on purpose: a
  shrunk counterexample is worth more than the run that produced it.

### Conventions

Full detail is in [docs/CONTEXT.md](docs/CONTEXT.md). The ones that bite
first:

- **Ordinary comments use `/* ... */`, not `//`.** Bare `//` lines are
  never stacked.
- **`// === Section` is the top-level divider**, `// == Subsection` below
  it, and `// = ` below that and rare.
- **Doc comments are one sentence per line**, broken at the full stop, so
  a diff touching one sentence touches one line.
- **No em dash** in any comment, commit message, document, or PR
  description.
- **Commits are signed, one line, `<type>: <description>`**, with no
  body, no trailers, and no co-authorship attribution.
- **Committing is an explicit instruction, and pushing is a separate
  one.** Finish the work and leave it uncommitted.

## 📦 Distribution

> **None of this is published.** Every row below is a plan. There is no
> `culm` on npm, no `culm` on crates.io, no `culm.dev`, and no Homebrew
> tap. Do not paste these commands expecting them to work.

| Channel | Planned command | Audience |
| --- | --- | --- |
| npm | `pnpm create culm` / `npx culm@latest new` | Primary funnel, zero install |
| crates.io | `cargo install culm` | Rust-native developers |
| Shell | `curl -fsSL https://culm.dev/install.sh \| sh` | CI and Docker |
| Homebrew | `brew install zunetra/tap/culm` | Only if v1.0 is reached |

The Rust binary is the product. The npm package is planned as a launcher
shipping prebuilt binaries as platform-specific `optionalDependencies`
with `os` and `cpu` fields, the pattern esbuild, Biome, `@swc/core`, and
Rspack use. The package manager installs exactly one.

**Not a postinstall downloader.** Those break under `--ignore-scripts`,
offline CI, corporate proxies, and sandboxed environments. See
[decision 8](docs/CONTEXT.md#decisions).

Both `culm` and `create-culm` were checked clean on npm, crates.io, and
the `.dev` domain on 2026-08-04, and are recorded in the handbook's
[reserved names](../handbook/naming/RESERVED.md). Availability changes,
so re-verify before relying on it.

## 🗺️ Roadmap

### Built

- The layer format: ten categories, shape-dependent cardinality, manifest
  parsing, selection validation, and selection resolution.
- The layer catalog, which walks a directory tree and loads every
  `layer.toml` it finds.
- The merge engine with `json-deep`, `line-block`, and `toml-members`.
- Order independence, covered by a proptest over every permutation.
- The scaffold module: the copy plan, `{{name}}` interpolation, path
  containment, and the disk writer.
- Twenty-six layers across base, shape, frontend, backend, data,
  toolchain, quality, and runner.
- Unit tests beside every module plus three integration suites.

### Not built

- **The CLI.** `src/main.rs` prints hello and nothing else. There is no
  `culm new`, no argument parsing, and no wizard.
- **Presets.** Designed in [decision 6](docs/CONTEXT.md#decisions) and
  [docs/01-scaffolder.md](docs/01-scaffolder.md) §6, implemented nowhere.
- **Install execution.** `[[install]]` entries are parsed and validated,
  and nothing runs a package manager.
- **The `yaml-list`, `hook-script`, and `yaml-steps` strategies.** Only
  the three above exist. Note that the `[[hook]]` block shown in
  `docs/01-scaffolder.md` §3 is design, not format: the parser rejects it
  as an unknown field today.
- **Version resolution**, including the `compat` range and the
  minimum-release-age guard.
- **Nightly CI** over the preset matrix.
- **Telemetry.** The payload is specified in
  [docs/TELEMETRY.md](docs/TELEMETRY.md) and nothing sends it.
- **`infra` and `editor` layers.** The categories exist with no members.
- **The entire runner.** Task graph, `cas`, and build cache. See
  [docs/02-runner.md](docs/02-runner.md).

### The order from here

1. Bring each layer up to the quality bar, composed by category per
   [decision 19](docs/CONTEXT.md#decisions). Start with `backend/actix`, since
   the reference exists for it, then `backend/axum`, then the Node
   backends, then the frontends.
2. Non-interactive flags, so a selection can be made without a wizard.
3. The install step, running `pnpm add` and `cargo add` after
   scaffolding.
4. Presets, then the wizard. **The wizard is deliberately last.** It is
   the part everyone builds first and the part that matters least, and
   flags have to work anyway.

## 🔧 Troubleshooting

### `scalar conflict in package.json: layers 'a' and 'b' both set 'scripts.dev' to different values`

Two layers in a many-per-workspace category contributed the same script
key. Namespace them by layer id: `dev:nuxt`, not `dev`. Only quality
layers may use a plain name, because at most one of each is selected.

### `strategy conflict in .gitignore: layer 'a' merges it as json-deep, layer 'b' as line-block`

One of the two manifests names the wrong `strategy` for that target.
Guessing which would be a silent resolution, so it is reported instead.
Check which strategy the target is supposed to use in
[Strategies](#strategies).

### `layer 'x' requires layer 'y', which is not selected`

Only toolchain requirements are filled in automatically. Everything else
has to be selected explicitly, because quietly choosing somebody's
database for them is a guess dressed up as an implication.

### `category 'runner' requires at least one layer, but the selection has none`

Every workspace needs a runner, whatever the shape. If you do not want
one, select `runner/none`, which is a real answer rather than an absence.

### `a single-app workspace needs at least one frontend or backend layer`

A single app with neither is a bare toolchain, not a workspace. Either
add an app layer, or switch the shape to `tooling-only`, which is the
shape that exists for exactly that case.

### `shape 'tooling-only' allows no frontend layers, but the selection has nuxt`

A framework layer scaffolds an app, and an app implies a shape. Asking
for tooling only and then naming a framework is a contradiction, so it is
caught rather than half-scaffolded.

### `layer 'x' writes '../y', which escapes the workspace root`

A `[[files]]` destination climbed out of the root, or was absolute. Fix
the `to` path. This is refused by design and there is no flag to allow
it.

### `layer 'x' declares 'files/y' which is not in the layer directory`

A `[[files]]` entry's `from` path is relative to the layer's own
directory, not the repository root, and the file has to exist.

### The output changed between two runs with the same layers

That is the one bug this engine exists to prevent. Check whether a
comparator was changed: any comparator the engine sorts with must be a
total order consistent with equality, and reporting `Equal` for values
that differ leaves a stable sort holding arrival order, which is layer
order. See
[the total order invariant](#the-total-order-invariant-learned-the-hard-way).

### `cargo test` passes but `cargo clippy` fails

Run it with `--all-targets`. Test code is not linted otherwise, and it is
where warnings accumulate unseen.

## 🤝 Contributing

1. Read [docs/CONTEXT.md](docs/CONTEXT.md#decisions) first. Its `LOCKED`
   entries are settled and closed, and are not to be re-litigated or
   worked around. The `Open` section at the bottom is the only place
   where a decision is genuinely still available.
2. Read [docs/CONTEXT.md](docs/CONTEXT.md) second. It is authoritative on
   how code here is written, and it outranks anything stated elsewhere,
   including `CLAUDE.md` and this file.
3. Branch, change, and run the gate:
   ```bash
   cargo test
   cargo clippy --all-targets -- -D warnings
   cargo fmt
   ```
4. One line commit message, `<type>: <description>`, signed.
5. Open a pull request.

### The rules that outrank convenience

- **The scaffolder must be able to emit Nx, Turborepo, Moon, or nothing.**
  Never emit configuration only culm's runner can consume.
- **No version numbers in a layer template file.** Ever.
- **Never author a combination of layers.** The moment a special case
  exists for a specific pair, the combinatorial death has started.
- **Klassivox is not modified by work in this repository** unless the
  owner explicitly asks for a Klassivox change, naming it. Keeping
  documentation in sync is not a reason.

## 📄 Documents

| Doc | What it covers |
| --- | --- |
| [docs/CONTEXT.md](docs/CONTEXT.md) | Every settled decision, the coding conventions, and the traps. Read first. |
| [docs/01-scaffolder.md](docs/01-scaffolder.md) | The v0.1 product: layers, the merge engine, the wizard, presets. |
| [docs/02-runner.md](docs/02-runner.md) | The task graph and build cache. Higher risk, ships second. |
| [docs/TELEMETRY.md](docs/TELEMETRY.md) | The exact payload that would be sent, published verbatim. |
| [handbook/naming/NAMING.md](../handbook/naming/NAMING.md) | Why culm, what was eliminated, what is reserved. |

The quality bar in [decision 19](docs/CONTEXT.md#decisions) was a clone
of `dev-davexoyinbo/actix-web-starter-template`, kept in `docs/` until
the backends reached it and deleted on 2026-08-15. Its copyright is
`2025 Oyinbo David Bayode`, MIT, so anything still derived from it owes
attribution. `backend/actix` is now the reference.

## 🏛️ Organisation

culm is a [Zunetra](../handbook) project. Org-level decisions, including
naming and reserved names, live in that repository, not this one.
[Klassivox](../Klassivox) is the proving fixture and the first customer.
[ipile](../ipile) is the machine bootstrap tool.

## 📄 Licence

MIT. See [LICENSE](LICENSE).
