# ipile - Claude Instructions

Project-specific additions to the global instructions.

---

## Before doing anything

Read [docs/DECISIONS.md](docs/DECISIONS.md) first. Its `LOCKED` entries
are settled and closed. Do not re-litigate or work around any of them.
Its `Open` section is the only place where a decision is genuinely still
available.

Then [docs/NEXT.md](docs/NEXT.md), which is the only document that tracks
state: what exists, what is next, and what is blocked on the owner.
Update it when that changes. Every other document describes intent.

The two design documents that follow from it:

- [docs/01-scaffolder.md](docs/01-scaffolder.md), the v0.1 product
- [docs/02-runner.md](docs/02-runner.md), the second half, higher risk

## The name

The written brand is **Ìpìlẹ̀**. Everything a machine reads is plain
ASCII `ipile`: folder, repository, binary, npm package, crate, domain,
environment variables. Never introduce a diacritic into a path, an
identifier, a package name, or a command.

## Git commits

- Always sign commits (`git commit -S`, or rely on configured
  `commit.gpgsign`) unless explicitly asked for an unsigned commit.
- One line per commit, `<type>: <description>`, using conventional commit
  types: `feat`, `fix`, `refactor`, `chore`, `test`, `docs`, `perf`,
  `style`. No body, no bullet points.

## Comment style

Comment on non-obvious WHY, never WHAT. Comments are warranted where
reasoning is not visible in the code, not as decoration every few lines.

**Single-line:** `//` for Rust and TypeScript.

**Section headings**, using only the depth a file actually needs:

- `// === Section` for a top-level divider
- `// == Subsection`, only when a section is genuinely large enough
- `// = Sub-subsection`, rare. If a file needs this depth, consider
  splitting it.
- Plain `//` is never a heading.

**Multi-line** comments use `/** ... */` with continuation lines prefixed
`* `. Do not stack bare `//` lines to fake a paragraph.

**No em dash** in any comment, commit message, document, or PR
description. Use a plain hyphen or rewrite the sentence.

## Rust conventions

- `rustfmt.toml` and `clippy.toml` stay at defaults. Nothing here needs a
  non-default lint configuration.
- `cargo clippy --all-targets -- -D warnings` must pass before any
  commit.
- Errors are `thiserror` types in libraries, `anyhow` at the binary
  boundary.

## The two rules that outrank convenience

1. **The scaffolder must be able to emit Nx, Turborepo, Moon, or
   nothing.** Never emit configuration only ipile's runner can consume.
   [docs/DECISIONS.md](docs/DECISIONS.md) decision 3.
2. **No version numbers in layer template files.** Package managers write
   versions. [docs/DECISIONS.md](docs/DECISIONS.md) decision 5.

## Locked files

`docs/prompt/` holds original working notes, preserved unedited. Read
them freely, cite them freely, change them never. This holds even for
instructions that sound repository-wide, such as "update the docs" or
"fix the broken links".

## Klassivox

Klassivox is the proving fixture and the first customer. It lives in a
separate repository and **is not modified by any work in this one**, for
any reason, including keeping documentation in sync.
