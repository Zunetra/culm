# Contributing to culm

culm scaffolds a polyglot workspace out of layers, and will later run its
tasks. This document covers how work happens here: what to read first,
branch and commit rules, how to add a layer, what the tests and CI prove,
and the review bar.

[docs/CONTEXT.md](docs/CONTEXT.md) is authoritative on conventions and
decisions. Where this file summarises it, CONTEXT.md wins. Nothing here
restates a rule in full, so the two cannot drift apart.

---

## Table of contents

- [Before your first change](#before-your-first-change)
- [Getting set up](#getting-set-up)
- [Branches](#branches)
- [Commits](#commits)
- [Pull requests](#pull-requests)
- [The two rules that outrank convenience](#the-two-rules-that-outrank-convenience)
- [Adding a layer](#adding-a-layer)
- [Adding a preset](#adding-a-preset)
- [Working on the merge engine](#working-on-the-merge-engine)
- [Code conventions](#code-conventions)
- [Tests](#tests)
- [CI](#ci)
- [What not to touch](#what-not-to-touch)
- [Review bar](#review-bar)

---

## Before your first change

Read these, in this order. They are short, and skipping them is how a
settled decision gets re-litigated in review.

1. **[docs/CONTEXT.md](docs/CONTEXT.md), the `Decisions` section.** Every
   entry is `LOCKED`. They are closed, and a change that works around one
   is rejected on those grounds alone rather than on its merits. The
   `Open` section is the only place a decision is genuinely still
   available.
2. **The rest of [docs/CONTEXT.md](docs/CONTEXT.md).** Coding
   conventions, the modularity rules, the total order invariant, and the
   traps section. That last one exists so the same bug is not found
   twice.
3. **[docs/01-scaffolder.md](docs/01-scaffolder.md)**, the v0.1 product,
   and **[docs/02-runner.md](docs/02-runner.md)**, the second half.

State lives in the code and in git history, not in a document. If a doc
and the code disagree, the code is what is true and the doc is a bug.

---

## Getting set up

culm is a single crate on edition 2024. There is no workspace, no
`crates/` directory, and no Node toolchain needed to build it.

```sh
git clone git@github.com:Zunetra/culm.git
cd culm
cargo build
cargo test
```

The four commands that have to pass before anything is committed:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

The release build is not redundant. The layer tree is compiled into the
binary with `include_dir`, so a template that exists on disk but was
never declared in a manifest is only caught by building the real
artefact.

Working on layers additionally wants `pnpm` and a Node toolchain, since
that is what the generated workspaces install with.

---

## Branches

Branch from `develop`. `main` is the released line.

Every branch is either `main`, `develop`, or `<type>/<description>`:

| Prefix      | For                                                       |
| ----------- | --------------------------------------------------------- |
| `feature/`  | A new capability                                           |
| `fix/`      | A bug fix                                                  |
| `refactor/` | A change with no behavioural difference                    |
| `chore/`    | Tooling, dependencies, repository plumbing                 |
| `hotfix/`   | An urgent fix, branched from `main` and merged back        |
| `release/`  | Release preparation                                        |
| `conflict/` | An isolated branch for resolving one merge conflict        |
| `docs/`     | Documentation only                                         |

Use the layer path in the description where a change is about one layer,
so the branch says which: `feature/frontend-svelte-layer`,
`fix/merge-json-deep-array-order`.

---

## Commits

Full rules in [docs/CONTEXT.md](docs/CONTEXT.md#git). The short version,
because these three are the ones that get missed:

- **One line.** `<type>: <description>`. No body, no bullets, no
  trailers, no co-authorship attribution.
- **Signed.** `git commit -S`, or a configured `commit.gpgsign`.
- **Types:** `feat`, `fix`, `refactor`, `chore`, `test`, `docs`, `perf`,
  `style`.

```
feat: add the svelte frontend layer
fix: break sort ties on a number's textual form
docs: record why templates carry no versions
```

Committing and pushing are each a separate explicit act. Finishing a
piece of work is not on its own a reason to commit it, and permission to
commit is not permission to push.

**No em dash** in a commit message, a comment, a document, or a pull
request description. Use a plain hyphen or rewrite the sentence.

---

## Pull requests

- Target `develop`, unless it is a `hotfix/`, which targets `main`.
- One concern per pull request. A layer addition and a merge engine fix
  are two.
- Say what changed and why. The why is the part review cannot reconstruct
  from the diff.
- If the change touches a `LOCKED` decision, say which one and why it
  should reopen. Do not work around it quietly.
- A new or re-harvested layer needs a green `templates.yml` run before it
  is called done. See [Adding a layer](#adding-a-layer).

---

## The two rules that outrank convenience

These are kill criteria, not style notes. A change that violates either
is wrong no matter how convenient it is.

**1. The scaffolder must be able to emit Nx, Turborepo, Moon, or
nothing.** The runner is a layer, and Nx is a valid answer. If the
scaffolder ever emits configuration only culm's runner can consume, the
tool has become a lock-in play. Decision 3.

**2. No version numbers in layer template files.** Package managers write
versions. `pnpm add` and `cargo add` resolve what is current on the day
the scaffold runs, which is what makes the weekly template build a
meaningful check rather than a diff against a pin. Decision 5.

A third, from the traps list: **never author a combination of layers.**
The moment a special case exists for one specific pair, the combinatorial
death has started.

---

## Adding a layer

A layer is a directory under `layers/<category>/<id>/`. The categories
are declared in `ALL_CATEGORIES` in
[`src/layer/category.rs`](src/layer/category.rs), which is the list to
check rather than this one:

`base`, `shape`, `frontend`, `backend`, `data`, `toolchain`, `quality`,
`runner`, `infra`, `editor`.

`infra` and `editor` are declared but have no layers on disk yet, so a
first layer in either is new ground rather than a pattern to follow.

A layer's identity is always `<category>/<id>`, never the bare id.
`runner/none` and `shape/single-app` are both real layers; `none` on its
own is not a thing.

### Anatomy

```
layers/quality/prettier/
├── layer.toml          # the manifest: identity, requires, merges, files, installs
└── files/              # verbatim template files, copied or interpolated
    ├── .prettierrc
    └── .prettierignore
```

`layer.toml` declares:

- `id` and `category`, which together form the identity
- `requires`, other layers this one depends on
- `default`, whether it arrives unless suppressed with `--without`
- `[[merge]]`, contributions into a shared file, by strategy and target
- `[[files]]`, template files and where they land
- `[[install]]`, packages for a manager to add, with no versions

### Rules that will fail review

- **A layer that contributes to another layer's file declares it in
  `requires`.** Contributing to `package.json` means requiring
  `toolchain/node-pnpm`. The same holds for ESLint.
- **A layer's `tsconfig.json` goes inside that layer's own source
  directory and is never renamed.** A root `tsconfig.next.json` looks
  like it avoids a collision and instead leaves every file in that layer
  with no TypeScript configuration at all, silently. Where a framework
  needs telling, point it at the scoped file.
- **Generated JavaScript is ESM only.** A file that genuinely must be
  CommonJS names itself `.cjs`. Decision 21.
- **The bar is comprehensive, not toy.** A generated backend has real
  structure, not a single route file. Comprehensiveness is composed from
  the framework's own generator output, never copied wholesale.
- **No hardcoded lists.** Anything discovered by walking a directory or
  iterating `ALL_CATEGORIES` scales. A new layer must not require editing
  an unrelated file to be seen.

### Authoring loop

Two flags make the loop fast.

`--layers-dir` reads from the working tree instead of the tree compiled
into the binary, so a layer can be iterated without rebuilding culm.
`--no-install` writes the workspace and prints the package manager
commands rather than running them, which is what you want while the file
list is still moving.

**Scaffold outside the checkout.** `culm new` writes into the current
working directory, so running `cargo run` from the repository root drops
a workspace into the repository, where it inherits culm's own Cargo
workspace and `.gitignore` and stops being the thing a user would get.
Build once, then run the binary from a temporary directory, the way
`templates.yml` does:

```sh
cargo build
CULM="$PWD/target/debug/culm"
LAYERS="$PWD/layers"

cd "$(mktemp -d)"
"$CULM" new demo --layer frontend/svelte --layers-dir "$LAYERS" --no-install --yes
```

Drop `--no-install` once the file list has settled, to check that what
the manifest installs is what the templates actually import.

### Verifying a layer

`cargo test` cannot reach a network, so **no test in this repository can
prove a template builds.** That is what
[`.github/workflows/templates.yml`](.github/workflows/templates.yml) is
for.

A layer that emits an application must be added to that workflow's matrix
and put through a `workflow_dispatch` run before it is called done. Give
it the scripts that prove it: a build, a test, a typecheck where the
framework has one, and always `lint` and `format:check`. Those last two
fail silently otherwise, because a flat ESLint config claims `.js`,
`.mjs` and `.cjs` and nothing else, so a workspace of TypeScript can be
entirely unlinted and still exit 0.

A manifest-only layer does not go in the matrix. It would fail for
reasons unrelated to upstream, which would make a red run mean nothing.

### When a template goes red

The weekly run catches shape changes, not version changes: a router
convention replaced, a config file renamed, a required file appearing.
Once or twice a year per framework.

Re-run that framework's own generator by hand, harvest the changed files
into `layers/<category>/<id>/files/`, strip whatever version the
generator wrote, and push.

**Nothing automated may rewrite a template.** Those files are the quality
bar, and a bot committing framework bytes nobody read means the first
person to see a regression is a user. Renovate and Dependabot have
nothing to act on here, and adding them would be a category error.

---

## Adding a preset

A preset is a named starting selection at `presets/<name>.toml`: a
`name`, a `description`, and a list of `layers`.

```toml
name = "monorepo"
description = "A Nuxt front end and an Axum service backed by SQLx, in one workspace."
layers = ["shape/monorepo", "frontend/nuxt", "backend/axum", "data/sqlx"]
```

TOML rather than YAML, per decision 20. Layers are named by full
identity. A description says what the user gets and is honest about what
is not included yet.

---

## Working on the merge engine

`src/merge/` is where layer contributions are combined: `json_deep.rs`,
`line_block.rs`, `toml_members.rs`.

**The total order invariant.** Any comparator the engine sorts with must
be a total order consistent with equality, reporting `Equal` only when
the values genuinely are equal.

This was violated once and it is worth knowing how, because the failure
mode is the point. Array entries are sorted so output does not depend on
which order layers were applied. The comparator ordered JSON numbers
through `f64`, so integers above 2^53 collapsed onto the same value and
compared `Equal` while differing. `sort_by` is stable, so it left them in
arrival order, and arrival order is layer order: the rendered bytes
changed depending on which layer went first, which is exactly the
property the engine exists to guarantee.

Two lessons that generalise:

- The property test missed it because its generators drew from small
  pools and could never produce two colliding numbers. A property test is
  only as strong as the inputs it can reach.
- The failure was silent. Silent wrong output is the worst category of
  bug this project can ship.

Any change to a comparator needs a test that would have caught that.

---

## Code conventions

Full rules in [docs/CONTEXT.md](docs/CONTEXT.md). The ones that come up
most:

**Layout.** One crate, edition 2024. A module is a directory with a
`mod.rs`, not one long file. `mod.rs` declares and re-exports and holds
no logic. Errors live in their own `error.rs` once a module has more than
a couple of variants. Split a file when it earns it, not preemptively.

**One concern per file.** The test is whether the file has one reason to
change.

**The second caller is when a function moves, and it moves immediately.**
Writing the same logic twice is two answers to one question, and they
drift the first time somebody corrects one.

**Shared code goes where its subject lives, never in a `util` bucket.** A
crate-wide `util/` has no reason to change, so it has no boundary and
accumulates whatever is homeless.

**No unsafe.** `#![forbid(unsafe_code)]` sits at the top of `src/lib.rs`
and `src/main.rs` so the compiler enforces it.

**Borrow by default.** `&str` over `String`, `&Path` over `PathBuf`,
iterators over eagerly collected `Vec`s. Prefer `impl IntoIterator<Item =
&T>` over a concrete container, so callers do not allocate to fit a
signature.

**Cloning is allowed where it buys something**, and is deliberately not
banned. A clone that upholds an invariant is a good trade in a tool that
runs once per project. Never clone merely to quiet the borrow checker:
that means the ownership design is wrong and the clone is hiding it.

**Errors** are `thiserror` in library code, `anyhow` at the binary
boundary.

**`rustfmt.toml` and `clippy.toml` stay at defaults.** Do not add either.

### Comments

- Comment on non-obvious WHY, never WHAT.
- `// === Section` is the top-level divider. `// == Subsection` for a
  section genuinely large enough, `// = Sub-subsection` rarely. Plain
  `//` is never a heading.
- **Ordinary comments use `/* ... */`, not `//`**, for single lines as
  well as paragraphs. Do not stack bare `//` lines.
- `///` doc comments on public items, **one sentence per line.** Break at
  the full stop and start a new `///` line, so a diff touching one
  sentence touches one line.
- No em dash, anywhere.

---

## Tests

**Unit tests sit beside the code they test**, in `#[cfg(test)] mod tests`
under a `// === Tests` heading.

**Cross-module behaviour goes in `tests/`**, where the crate is exercised
the way a caller would use it:

- `tests/cli.rs`, the command surface
- `tests/layers.rs`, the shipped catalog and its manifests
- `tests/scaffold.rs`, what lands on disk
- `tests/order_independence.rs`, the property tests behind the total
  order invariant

Commands are driven through the library rather than by spawning a
process. `run_new_to` takes a writer the caller owns, and `run_new` takes
the parent directory as an argument, precisely so tests never shell out.
A subprocess test is slower, flaky under load, and has to guess the
binary's path per profile.

Test names are sentences about behaviour, not about functions:
`a_rust_workspace_writes_nested_directories`,
`integers_beyond_f64_precision_still_sort_deterministically`.

A bug fix comes with the regression test that would have caught it.

---

## CI

Two workflows, and the split is deliberate.

**[`ci.yml`](.github/workflows/ci.yml)** runs on every push to `main` and
`develop` and on every pull request: `cargo fmt --check`, `cargo clippy
--all-targets -- -D warnings`, `cargo test --all-targets`, and a release
build. Each step reports rather than stopping the job, so one run gives
the whole picture instead of a formatting slip hiding a clippy finding.

It never reaches a network beyond the crate registry, so it proves only
that culm assembles what the manifests describe.

**[`templates.yml`](.github/workflows/templates.yml)** proves the
manifests still describe reality. It scaffolds each application-emitting
layer, installs, builds, and tests it. Weekly on a cron, plus on demand
as the manual verification gate for a new or re-harvested layer.

Nothing in `ci.yml` can stand in for `templates.yml`, and a red weekly
run opens a single `template-drift` issue rather than one per week.

---

## What not to touch

**Klassivox.** It lives in a separate repository and is the proving
fixture and first customer. It is **not modified by work in this
repository unless the owner explicitly asks for a Klassivox change,
naming it.** Keeping documentation in sync is not a reason, and a generic
instruction that merely sounds repository-wide does not authorise it.
Decision 13.

**`LOCKED` decisions.** Reopen one by proposing it explicitly, not by
writing code that routes around it.

**Template files, automatically.** Re-harvesting is a human act. See
[When a template goes red](#when-a-template-goes-red).

**Diacritics.** Plain ASCII everywhere a machine reads it: folder,
repository, binary, npm package, crate, domain, environment variables.
The project is `culm`. Decision 15.

---

## Review bar

A change is ready when:

- `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --all-targets`, and `cargo build --release` all pass
- New behaviour has a test, and a fix has the regression test that would
  have caught it
- A new or changed application layer has a green `templates.yml` run
- No `LOCKED` decision is worked around
- No layer combination is special-cased
- No version number appears in a template file
- Comments explain why, and there is no em dash anywhere

Execution is the only evidence that counts. Four bugs in this project
survived writing, review, and a clean clippy, and every one was found the
first time the code actually ran.
