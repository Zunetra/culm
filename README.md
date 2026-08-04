# ipile

Pronounced **ìpìlẹ̀**, Yoruba for *foundation*.

Workspace tooling for polyglot repositories. One binary that scaffolds a
workspace and then runs its tasks.

> **Status: pre-alpha.** The merge engine is the only code that exists,
> and there is no usable CLI yet. Read
> [docs/DECISIONS.md](docs/DECISIONS.md) first, since it records what is
> settled and what is still open, then
> [docs/CONTEXT.md](docs/CONTEXT.md), which holds the coding conventions
> and the context that has to survive between sessions.

The written brand is **Ìpìlẹ̀**. Everything a machine touches is plain
ASCII `ipile`: the folder, the repository, the binary, the npm package,
the crate, the domain. Diacritics break shell paths, git remotes, URLs,
and package names, and nobody types accents at a terminal.

---

## What it does

ipile is two halves of one tool.

**The scaffolder** stands up a workspace so a developer never again
hand-wires prettier, eslint, rustfmt, clippy, husky, lint-staged,
tsconfig paths, docker-compose, and CI at 11pm on day one.

**The runner** knows what changed, skips what it can, and caches the rest
correctly across a repository that is genuinely more than one language.

The scaffolder ships first. It is cheaper, lower risk, and it is how
anyone finds the runner. See
[docs/DECISIONS.md](docs/DECISIONS.md) decision 2 for why that order is
not negotiable.

## The gap

`create-nx-workspace` will not give you a Cargo workspace, `rustfmt.toml`,
`clippy.toml`, `cargo-deny`, and a pre-commit hook that runs
`cargo fmt --check` on staged `.rs` files *and* `lint-staged` on staged
`.ts` files. `cargo-generate` will not give you the JavaScript half.
Neither will, because the JS tools are JS-first and the Rust tools are
Rust-first.

The same split shows up in the runner. A repository with a root
`Cargo.toml` next to a `pnpm-workspace.yaml` has two dependency graphs and
two lockfiles, and every existing task runner treats one of them as the
real one and the other as a plugin.

That is the whole thesis: **be correct about the polyglot case, which is
the case everyone else bolted on afterwards.**

## Planned distribution

| Channel | Command | Audience |
| --- | --- | --- |
| npm | `pnpm create ipile` / `npx ipile@latest new` | primary funnel, zero install |
| crates.io | `cargo install ipile` | Rust-native developers |
| Shell | `curl -fsSL https://ipile.dev/install.sh \| sh` | CI and Docker |
| Homebrew | `brew install zunetra/tap/ipile` | only if v1.0 is reached |

The Rust binary is the product. The npm package is a launcher that ships
prebuilt binaries as platform-specific optional dependencies, the pattern
esbuild, Biome, and `@swc/core` use. It is not a postinstall downloader.
See [docs/DECISIONS.md](docs/DECISIONS.md) decision 8.

## Documents

| Doc | What it covers |
| --- | --- |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Every settled decision, with reasons. Read first. |
| [docs/CONTEXT.md](docs/CONTEXT.md) | Coding conventions, known traps, and context worth keeping. Read second. |
| [docs/01-scaffolder.md](docs/01-scaffolder.md) | The v0.1 product: layers, the merge engine, the wizard, presets. |
| [docs/02-runner.md](docs/02-runner.md) | The task graph and build cache. Higher risk, ships second. |
| [docs/TELEMETRY.md](docs/TELEMETRY.md) | The exact payload sent, published verbatim. |
| [docs/NAMING.md](docs/NAMING.md) | Why ipile, what was eliminated, what is reserved. |
| [docs/prompt/GrainPrompt.md](docs/prompt/GrainPrompt.md) | Original raw working notes, preserved unedited. |

## Organisation

ipile is a [Zunetra](../handbook) project. Org-level decisions, including
naming and reserved names, live in that repository, not this one.

## Licence

MIT.
