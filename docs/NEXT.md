# Next

**The only document that tracks state.** Everything else describes intent.
If you are picking this up cold, read this after
[DECISIONS.md](DECISIONS.md) and before anything else.

**Last updated:** 2026-08-04

---

## Where this actually is

**Nothing is built.** This repository contains design documents and no
code. No `Cargo.toml`, no crates, no binary, no tests. That is the
correct state, because the layer format and the merge engine needed
settling on paper before either got written twice.

| Thing | State |
| --- | --- |
| Decisions | 17 locked, 4 open. See [DECISIONS.md](DECISIONS.md). |
| Scaffolder design | Complete enough to build from. [01-scaffolder.md](01-scaffolder.md). |
| Runner design | Complete, inherited from Klassivox's original doc. [02-runner.md](02-runner.md). |
| Telemetry spec | Complete. [TELEMETRY.md](TELEMETRY.md). |
| Naming | Settled. [NAMING.md](NAMING.md). |
| Code | None. |
| Name reservations | **None done.** See below. |
| Git | Not initialised. |

## Blocked on the owner

Nothing technical is blocked. These need a human with accounts and a card.

- [ ] **Reserve the names.** npm never releases a published name, so this
      is the only genuinely time-sensitive item. `ipile`, `create-ipile`,
      crates.io `ipile`, `ipile.dev`. Same four for `culm`. Checklist in
      [../../handbook/naming/RESERVED.md](../../handbook/naming/RESERVED.md).
- [ ] **Confirm the `zunetra` GitHub org is free** and create it. Its
      availability was never verified. This is the stated reopen
      condition on
      [../../handbook/decisions/0001-org-name.md](../../handbook/decisions/0001-org-name.md).
- [ ] **LICENSE file.** Apache-2.0 is decision 14, but the copyright line
      needs an entity name and Zunetra is not registered.
- [ ] **Delete the empty `Ìpìlẹ̀/` folder** at `Projects/Zunetra/`. It was
      created before decision 15 settled on ASCII. `ipile/` is the real
      one.
- [ ] **Decide what happens to `Projects/Zunetra/GrainPrompt.md`.** It is
      already copied to [prompt/GrainPrompt.md](prompt/GrainPrompt.md),
      so the root copy is redundant.

## The next work, in order

1. **`git init`**, first commit of these documents.
2. **Settle open decision 3, the config filename**, before any format is
   written. Renaming a config file after release is a migration.
3. **Build the merge engine first, not the CLI.** It is the only hard
   part and the only part worth being proud of. Everything else copies
   files. Start with `json-deep` and `line-block`, with the order
   independence property test written alongside rather than after.
4. **Then the layer format**, then three first-party presets, then
   non-interactive flags. That is v0.1 as scoped in
   [01-scaffolder.md](01-scaffolder.md) §9.
5. **The wizard comes second, not first.** Interactive is the part
   everyone builds first and it is the part that matters least. Flags
   have to work anyway.

## Traps already identified, do not rediscover

- **Never author a combination of layers.** The moment a special case
  exists for a specific pair of layers, the combinatorial death has
  started. This is a kill criterion, not a style note.
- **Never emit configuration only ipile's runner can consume.** Nx must
  stay a valid answer, or the entire reason for building the scaffolder
  first collapses. Decision 3.
- **Never put a version number in a layer template file.** Decision 5.
- **Never modify Klassivox**, including to keep documentation in sync.
  Decision 13.
- **The runner is expected to possibly die at r0.2.** That is planned,
  not failure. [02-runner.md](02-runner.md) §10.

## Context a fresh session will not have

- The name was Grain until it collided with the Grain programming
  language. Every rejected alternative and its reason is in
  [NAMING.md](NAMING.md), so the search does not get repeated.
- The org was almost Nomia. It was taken.
- The original plan put all five Rust projects in one `rust-infra`
  workspace sharing `crates/cas`. That is superseded by decision 11 and
  [../../handbook/decisions/0002-repo-per-project.md](../../handbook/decisions/0002-repo-per-project.md).
  `cas` is built inside ipile and stays unpublished.
- `Projects/Mine` was renamed to `Projects/Zunetra` on 2026-08-04, and
  the org repository is `handbook`, not `zunetra`.
