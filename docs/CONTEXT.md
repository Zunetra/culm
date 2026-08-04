# Context

**Conventions and hard-won context.** This is the running log of how code
in this repository is written and of the things a fresh session would
otherwise have to rediscover the hard way.

State is not tracked here. The code and the git history track state.
Settled product decisions live in [DECISIONS.md](DECISIONS.md), which
this file does not repeat.

**How to use this file:** when the owner corrects a coding decision, the
correction gets appended here as a new entry under the section it
belongs to, or as a new `##` section if it fits none of them. Corrections
are authoritative and outrank anything stated elsewhere, including
[../CLAUDE.md](../CLAUDE.md).

**Last updated:** 2026-08-04

---

## Comment conventions

Comment on non-obvious WHY, never WHAT. Comments are warranted where the
reasoning is not visible in the code, not as decoration every few lines.

### Section headings

Use only the depth a file actually needs.

- `// === Section` is the top-level divider.
- `// == Subsection` exists, for a section genuinely large enough.
- `// = Sub-subsection` exists and is rare. If a file needs this depth,
  consider splitting it.
- Plain `//` is never a heading.

### Doc comments

`///` doc comments are correct and encouraged on public items.

**One sentence per line.** Break the line at the full stop and start a
new `///` line for the next sentence. Never wrap a single sentence across
multiple `///` lines.

Correct:

```rust
/// Identifies the layer a contribution came from.
/// Ordered, because rendered block order is defined as sorted by layer id.
pub struct LayerId(String);
```

Incorrect:

```rust
/// Identifies the layer a contribution came from. Ordered,
/// because rendered block order is defined as sorted by
/// layer id.
pub struct LayerId(String);
```

The point is that a diff touching one sentence touches one line.

### Ordinary comments

Ordinary non-doc comments use `/* ... */`, not `//`. This holds for
single-line comments as well as paragraphs. Do not stack bare `//` lines.

### No em dash

No em dash in any comment, commit message, document, or PR description.
Use a plain hyphen or rewrite the sentence.

---

## Rust and layout

**ipile is a single crate.** One package named `ipile`, one `src/`,
edition 2024. It is not a Cargo workspace and has no `crates/`
directory.

Why that is the right shape:

- Decision 11 says abstract later. Shared code gets extracted when a
  second real consumer exists, not before.
- ipile is a standalone project. If infrastructure is ever needed, it
  wraps ipile rather than containing it.
- The merge engine is a module under `src/merge/`, not a separate crate.
- If `cas` needs to become its own crate at the runner's v0.2, split it
  then, with a real reason.
- Splitting a module into a crate later is mechanical. Un-splitting a
  premature workspace is not.

Other Rust conventions:

- `rustfmt.toml` and `clippy.toml` stay at defaults. Nothing here needs a
  non-default lint configuration.
- `cargo clippy --all-targets -- -D warnings` must pass before any
  commit.
- Errors are `thiserror` types in library code, `anyhow` at the binary
  boundary.

---

## Modularity and separation of concerns

**A module is a directory with a `mod.rs`, not one long file.** Single
crate does not mean single file. `src/merge/` and `src/layer/` are both
directories, and anything of comparable size that comes later is too.

**One concern per file.** The test is whether the file has one reason to
change. `src/layer/` splits into `category.rs`, `manifest.rs`,
`validate.rs`, and `error.rs` because what a category is, what a manifest
holds, whether a selection is legal, and how failure is reported are four
separate questions. Only the third depends on the other two.

**`mod.rs` declares and re-exports, and holds no logic of its own.**
Callers import from the module root, so internal reorganisation does not
break them. A `mod.rs` that has grown its own functions is a file that
should have been a submodule.

**Errors live in their own `error.rs`** once a module has more than a
couple of variants. The error type is the module's public contract for
failure and reads badly buried under the code that happens to raise it.

**Tests sit beside the code they test**, in a `#[cfg(test)] mod tests`
under a `// === Tests` heading. Cross-module behaviour goes in `tests/`
instead, because that is where the crate is exercised the way a caller
would use it.

**Split when a file earns it, not preemptively.** A 40 line module does
not need a directory. The same abstract-later reasoning that keeps ipile
a single crate applies one level down.

---

## Safety and allocation

**No unsafe code.** `#![forbid(unsafe_code)]` sits at the top of both
`src/lib.rs` and `src/main.rs`, so this is enforced by the compiler
rather than by a document asking nicely. Nothing ipile does needs
unsafe. A workspace scaffolder and a task runner are IO and data
shuffling, and neither is bottlenecked on anything unsafe would buy.

**Borrow by default.** Prefer `&str` over `String`, `&Path` over
`PathBuf`, and iterators over eagerly collected `Vec`s. Take references
in function signatures unless ownership is genuinely being transferred.

**Cloning is allowed where it buys something.** This is deliberately not
a ban, because a blanket ban would cost more than it saves:

- Decision 1 splits the tool in two. The scaffolder runs **once per
  project, ever**. The runner runs **every day, hundreds of times**.
- In the scaffolder, a clone that upholds an invariant or keeps the code
  legible is a good trade. `JsonDoc::merge` clones the document and its
  provenance map so a conflict halfway through a contribution leaves the
  document untouched. Hand-rolling rollback to avoid that clone would be
  more code, more branches, and more ways to be wrong, to save
  microseconds in a program that runs once in a repository's lifetime.
- In the runner, a clone on a hot path needs a comment saying why.
- **Never clone merely to quiet the borrow checker.** That one is not a
  performance question. It means the ownership design is wrong, and the
  clone is hiding it.

---

## The total order invariant

Any comparator the merge engine sorts with must be a **total order
consistent with equality**. It may report `Equal` only when the values
genuinely are equal. Every strategy that sorts has to honour this.

This is not abstract. It was violated once already, and the failure is
worth keeping as the worked example.

Array entries are sorted so that output does not depend on the order
layers were applied. The comparator ordered JSON numbers by converting
both to `f64`. An `f64` carries 53 bits of precision, so integers above
2^53 no longer map one to one, and several distinct integers collapse
onto the same `f64`. The comparator then reported `Equal` for numbers
that were not equal. Rust's `sort_by` is stable, so it left those
entries in arrival order, and arrival order is layer order. The rendered
bytes changed depending on which layer was applied first, which is
exactly the property the engine exists to guarantee.

The fix was to break the tie on the exact textual form of the number, so
two different numbers can never compare `Equal`. The regression test is
`integers_beyond_f64_precision_still_sort_deterministically`.

Two lessons worth carrying:

- The property test did not catch it, because its generators drew from
  small value pools and could never produce two numbers that collapse.
  A property test is only as strong as the inputs it can reach.
- The failure was silent. Nobody traces a mysterious byte difference
  back to floating point precision. Silent wrong output is the same
  category as the runner's worst failure mode, a wrong cache hit.

---

## Git

**Committing is an explicit instruction, never an assumption.** Finish the
work and leave it uncommitted. Do not commit because the change looks
complete, because a milestone was reached, or because a task was
described as done.

**Pushing is a separate explicit instruction.** Permission to commit is
not permission to push. Never push unless told to push, in those words.

**Every commit is signed.** Use `git commit -S`, or rely on a configured
`commit.gpgsign`. The only exception is an explicit request for an
unsigned commit.

**Every commit message is one line**, `<type>: <description>`, using the
conventional commit types: `feat`, `fix`, `refactor`, `chore`, `test`,
`docs`, `perf`, `style`. No body, no bullet points, no trailers, no
co-authorship attribution.

---

## The name

The written brand is **Ìpìlẹ̀**. Everything a machine reads is plain ASCII
`ipile`: folder, repository, binary, npm package, crate, domain,
environment variables. Never a diacritic in a path, an identifier, a
package name, or a command.

Why ipile, and what was eliminated on the way, is in
[NAMING.md](NAMING.md).

---

## What is being built, and in what order

Written 2026-08-04 so a session picking this up cold does not have to
reconstruct the plan. The reasoning behind each item is in
[DECISIONS.md](DECISIONS.md); this is the shape of the work.

**The goal.** A user picks a stack and gets a workspace they would have
been happy to write themselves. Not a hello-world. The quality bar is the
actix reference in decision 19: configuration loading, error types, a
health check, an integration test, real tooling config.

**What exists.** The merge engine with `json-deep`, `line-block`, and
`toml-members`. The layer format with categories, shape-dependent
cardinality, manifest parsing, and selection validation. The scaffold
module: interpolation, the copy plan, and disk writing. Seventeen plus
layers across base, shape, frontend, backend, data, toolchain, quality,
and runner.

**What does not exist yet.** The CLI. The wizard. Presets. The
`hook-script` and `yaml-steps` strategies. Nightly CI. Installs are
parsed and validated but nothing runs a package manager.

**The order from here.**

1. Bring each layer up to the quality bar, composed by category per
   decision 19. Start with `backend/actix`, since the reference exists
   for it, then `backend/axum`, then the Node backends, then the
   frontends.
2. Non-interactive flags, so a selection can be made without a wizard.
3. The install step, running `pnpm add` and `cargo add` after scaffolding.
4. Presets, then the wizard. The wizard is deliberately last: it is the
   part everyone builds first and the part that matters least, and flags
   have to work anyway.

**Two rules that shape every layer authored from here.**

- Scripts in a many-per-workspace category are namespaced by layer id
  (`dev:nuxt`, `dev:nestjs`). A monorepo can select several frontends and
  several backends, and two layers writing different values to the same
  `package.json` key is a hard merge error. Quality layers are not
  namespaced, because at most one of each is selected and `format` has a
  single owner.
- A layer never names another layer in its content. Not in a package
  name, not in a script, not in an ignore rule. There is a test that
  fails if one does.

## Traps already identified, do not rediscover

- **Never author a combination of layers.** The moment a special case
  exists for a specific pair of layers, the combinatorial death has
  started. This is a kill criterion, not a style note.
- **Never emit configuration only ipile's runner can consume.** Nx must
  stay a valid answer, or the entire reason for building the scaffolder
  first collapses. Decision 3.
- **Never put a version number in a layer template file.** Decision 5.
- **Never modify Klassivox unless explicitly told to**, naming it.
  Keeping documentation in sync is not a reason. Decision 13.
- **The runner is expected to possibly die at r0.2.** That is planned,
  not failure. [02-runner.md](02-runner.md) §10.

---

## Context a fresh session will not have

- The name was Grain until it collided with the Grain programming
  language. Every rejected alternative and its reason is in
  [NAMING.md](NAMING.md), so the search does not get repeated.
- The org was almost Nomia. It was taken. The org is Zunetra, and
  github.com/Zunetra now exists and hosts `ipile` and `handbook`.
- The original plan put all five Rust projects in one `rust-infra`
  workspace sharing `crates/cas`. That is superseded by decision 11 and
  [../../handbook/decisions/0002-repo-per-project.md](../../handbook/decisions/0002-repo-per-project.md).
  `cas` is built inside ipile and stays unpublished.
- `Projects/Mine` was renamed to `Projects/Zunetra` on 2026-08-04, and
  the org repository is `handbook`, not `zunetra`.
- `docs/prompt/` held the original working notes and was a locked
  directory. The owner deleted it on 2026-08-04 and asked for the lock to
  be removed with it. Do not restore it.
- `docs/actix-web-starter-template/` is a third-party MIT clone, kept
  only as the quality bar described in decision 19. Its copyright is
  `2025 Oyinbo David Bayode`. Anything derived from it owes attribution,
  and the clone itself is to be deleted once the layers reach its
  standard.
- **Unverified as of 2026-08-04:** the `toml-members` wiring through
  `Strategy`, `Contribution`, `FileContent`, and `MergeSpec` was written
  but never compiled, at the owner's request to stop running tests. Run
  `cargo test`, `cargo clippy --all-targets -- -D warnings`, and
  `cargo fmt --check` before building on it. Everything before it was
  green at 74 tests.
