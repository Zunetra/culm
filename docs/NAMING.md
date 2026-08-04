# Naming

Why this project is called ipile, what was rejected, and what is held for
later. Recorded so the search is never repeated.

**Checked on 2026-08-04.** Availability changes. Anything not yet
reserved should be re-verified before it is relied on.

---

## The decision

**ipile**, pronounced *ìpìlẹ̀*, Yoruba for *foundation*.

| Surface | Status |
| --- | --- |
| npm `ipile` | available |
| npm `create-ipile` | available |
| crates.io `ipile` | available |
| `ipile.dev` | available |
| Existing product collision | none found |
| `github.com/ipile` | held by an unrelated active developer |

The GitHub namespace does not matter. Nx lives at `nrwl/nx`, Turborepo at
`vercel/turborepo`, Moon at `moonrepo/moon`. None own the bare name. This
repository lives under the Zunetra org.

**ASCII everywhere a machine reads it.** The folder, repository, binary,
npm package, crate, and domain are all `ipile`. The written brand is
**Ìpìlẹ̀**. Diacritics break shell paths, git remotes, URLs, and package
names, and nobody types accents at a terminal.

The tradeoff accepted: pronunciation is not self-evident to a global
audience. It is stated in the first line of the README, which is what Nx
and pnpm do with their own names.

## Why not Grain

The original working name. Abandoned for two independent reasons:

- **Grain is an existing programming language** at grain-lang.org,
  targeting WebAssembly. A permanent search collision.
- **`create-grain` is live on npm**, published recently, with keywords
  including `ai-agents` and `claude-code`. Adjacent enough to confuse.
- `grain` and `grain-cli` on npm are held by abandoned packages from 2016
  and earlier, which blocks the names without providing competition.

## Rejected candidates

Eliminated on collision, which is the Grain failure mode repeating:

| Name | Reason |
| --- | --- |
| quern | `quern-dev/quern`, an active AI-agent debug tool. `quern.dev` gone. |
| farro | npm `farro` is a live agent-native scaffolder CLI. Direct competitor in this exact category. |
| tilth | Taken on npm *and* crates.io by an active agent-tooling product, plus tilthapps.com. |
| millet | Millet, the Standard ML language server. |
| spelt | Spelt, a Swift static site generator. |
| bran | An EVM bytecode static-analysis framework. `bran.sh` registered. |
| granary | `raster-foundry/granary`. |
| stook | npm `stook`, a React state library. |
| swath | npm name reserved with zero published versions, so permanently unreclaimable. |

Clean but rejected on meaning:

| Name | Reason |
| --- | --- |
| chaffer | Flawless availability. Chaff is the worthless husk you discard, and "chaffer" archaically means to haggle. A build tool named after the discarded part is a joke waiting to be made. |
| groat | A hulled grain kernel, clean on npm and crates.io, but `groat.dev` is registered. |

## Reserved for later

**culm** is held for a future product. It is the stalk of a grain plant,
the structure the grain grows on. Free on npm, `create-culm`,
crates.io, and `culm.dev`, with no product collision found. Four
characters, which matters for a daily command.

It was explicitly **not** used as the organisation name, because spending
a good product name on an org gains nothing.

Reserved names are tracked in
[../../handbook/naming/RESERVED.md](../../handbook/naming/RESERVED.md).

## Reservation checklist

Names are first come, first served, and npm never releases a published
name. Do this before anything else:

- [ ] Publish `ipile` 0.0.0 placeholder to npm
- [ ] Publish `create-ipile` 0.0.0 placeholder to npm
- [ ] Publish `ipile` 0.0.0 placeholder to crates.io
- [ ] Register `ipile.dev`
- [ ] Same four for `culm`
- [ ] Create the `zunetra` GitHub org
