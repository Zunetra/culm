# ipile - Claude Instructions

Project-specific additions to the global instructions.

---

## Before doing anything

Read [docs/DECISIONS.md](docs/DECISIONS.md) first. Its `LOCKED` entries
are settled and closed. Do not re-litigate or work around any of them.
Its `Open` section is the only place where a decision is genuinely still
available.

Then [docs/CONTEXT.md](docs/CONTEXT.md), which holds the coding
conventions and the context that has to survive between sessions. It is
authoritative on how code here is written, and it is where a correction
from the owner gets recorded. State lives in the code and in git history,
not in a document.

The two design documents that follow from it:

- [docs/01-scaffolder.md](docs/01-scaffolder.md), the v0.1 product
- [docs/02-runner.md](docs/02-runner.md), the second half, higher risk

## The name

The written brand is **Ìpìlẹ̀**. Everything a machine reads is plain
ASCII `ipile`: folder, repository, binary, npm package, crate, domain,
environment variables. Never introduce a diacritic into a path, an
identifier, a package name, or a command.

## Git commits

[docs/CONTEXT.md](docs/CONTEXT.md) is the single source of truth here
too: signing, the one line message format, and the rule that committing
and pushing are each a separate explicit instruction. Not restated here,
so the two files cannot drift apart.

## Comment style

[docs/CONTEXT.md](docs/CONTEXT.md) is the single source of truth for
comment conventions, section headings, doc comments, and the no em dash
rule. Read it there. The rules are deliberately not restated here, so the
two files cannot drift apart.

## Rust conventions

Also in [docs/CONTEXT.md](docs/CONTEXT.md): the single crate layout, the
default rustfmt and clippy configuration, the clippy gate before every
commit, and the thiserror and anyhow split.

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
separate repository and **is not modified by work in this one unless the
owner explicitly asks for a Klassivox change**, naming it. Keeping
documentation in sync is not a reason. A generic instruction that merely
sounds repository-wide does not authorise it. See
[docs/DECISIONS.md](docs/DECISIONS.md) decision 13.
