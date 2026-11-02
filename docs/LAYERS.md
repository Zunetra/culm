# Layers

How culm's layers work, every layer that ships, and how to write a new one.
Back to the [README](../README.md).

- [The layer model](#-the-layer-model)
- [Shipped layers](#-shipped-layers)
- [Authoring a layer](#-authoring-a-layer)

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
   can only take `runner/none` or `runner/culm`, which requires nothing.
5. **Conflicts are checked symmetrically**, so one layer declaring the
   pair is enough and the two manifests cannot drift.

Explicit choices outrank opt-outs. Opting out only suppresses a layer
that would have arrived as a default.

## 📚 Shipped layers

Thirty-eight layers live under `layers/`. Every one of them is loaded,
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

### backend (6)

| Layer | Requires | Notes |
| --- | --- | --- |
| `backend/actix` | `toolchain/rust-cargo` | Binary at `src/bin/actix.rs` |
| `backend/axum` | `toolchain/rust-cargo` | Binary at `src/bin/axum.rs` |
| `backend/express` | `toolchain/node-pnpm` | Server at `src/express/server.js` |
| `backend/fastify` | `toolchain/node-pnpm` | Server at `src/fastify/server.js` |
| `backend/nestjs` | `toolchain/node-pnpm` | Sources under `src/nestjs/`, plus `nest-cli.json` |
| `backend/go-chi` | `toolchain/go-mod` | `{{app_dir}}/go.mod` is a complete module on its own, no `[[merge]]` entry |

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

### toolchain (3)

| Layer | Contributes |
| --- | --- |
| `toolchain/node-pnpm` | `private: true` in `package.json`, plus `node_modules/`, `.pnpm-store/`, `.npm/` |
| `toolchain/rust-cargo` | `/target`, `**/*.rs.bk`, `*.pdb` |
| `toolchain/go-mod` | `*.exe`, `*.test`, `*.out`, `vendor/`. No root manifest: every Go app owns a complete module in `{{app_dir}}` |

`private` is the one root key every JS layer can agree on, and they all
agree on the same value, so it merges rather than conflicts.

`toolchain/rust-cargo` contributes no `Cargo.toml`. Whether the
repository is one crate or many is the shape layer's business, and
guessing wrong would clobber a real manifest in an existing repository.

### quality (7)

| Layer | Default | Requires | Contributes |
| --- | --- | --- | --- |
| `quality/prettier` | ✅ | `toolchain/node-pnpm` | `format`, `format:check`, `.prettierrc`, `.prettierignore` |
| `quality/eslint` | ✅ | `toolchain/node-pnpm` | `lint`, `lint:fix`, `eslint.config.mjs`, `.eslintcache` ignore |
| `quality/rustfmt` | ✅ | `toolchain/rust-cargo` | `rustfmt.toml` |
| `quality/clippy` | ✅ | `toolchain/rust-cargo` | `clippy.toml` |
| `quality/husky` | ✅ | `toolchain/node-pnpm` | `.husky/pre-commit`, `.lintstagedrc.json`, installs `husky` and `lint-staged` |
| `quality/commitlint` | ✅ | `toolchain/node-pnpm`, `quality/husky` | `.husky/commit-msg`, `commitlint.config.mjs`, installs `@commitlint/cli` |
| `quality/editorconfig` | ✅ | `toolchain/node-pnpm` | `.editorconfig` |

Quality scripts are **not** namespaced, unlike the frontend ones. At most
one of each quality layer is selected, so `format` has a single owner and
cannot collide the way `dev` would across several frontends.

`quality/eslint` ships flat config only. `.eslintignore` was removed in
flat config, so ignores live inside `eslint.config.mjs`.

`quality/rustfmt` and `quality/clippy` contribute a config file and no
script, because Cargo already ships the command and there is no
`package.json` in a Rust-only repository to hang one on.

### runner (4)

| Layer | Requires | Contributes |
| --- | --- | --- |
| `runner/nx` | `toolchain/node-pnpm` | `.nx/cache` and `.nx/workspace-data` ignores, installs `nx` |
| `runner/turborepo` | `toolchain/node-pnpm` | `.turbo/` ignore, installs `turbo`. `turbo.json` is written by `culm new` from the final app layout |
| `runner/moon` | `toolchain/node-pnpm` | `.moon/cache/` ignore, installs `@moonrepo/cli`. The workspace and per-project files are written by `culm new` |
| `runner/culm` | | Nothing to install. `culm new` writes a root `culm.toml` with `[workspace]` and one per app; `test` is cached, `build` and `dev` are not |
| `runner/none` | | Nothing, and that is the point |

`runner/none` has to be a real selectable answer rather than an absence
the validator rejects, because [decision 3](CONTEXT.md#decisions) says the
scaffolder must be able to emit somebody else's runner or nothing at all.
If the scaffolder ever emits configuration only culm's runner can
consume, the entire argument for building the scaffolder first collapses.

### infra (3)

| Layer | Default | Requires | Contributes |
| --- | --- | --- | --- |
| `infra/dockerfile` | | | A `Dockerfile` per app |
| `infra/docker-compose` | | `infra/dockerfile` | `compose.yaml`, merged `.env.example` |
| `infra/github-actions` | | | `.github/workflows/ci.yml` |

### editor (2)

| Layer | Default | Requires | Contributes |
| --- | --- | --- | --- |
| `editor/vscode` | | `quality/prettier` | `.vscode/settings.json`, `.vscode/extensions.json` |
| `editor/zed` | | | `.zed/settings.json` |

All five are `zero or more` under every shape, so any combination, or
none, is a legal selection.

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
