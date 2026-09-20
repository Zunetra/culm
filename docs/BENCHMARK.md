# Benchmark methodology

r1.0's own kill criterion, per [02-runner.md](02-runner.md) section 10: "the
r1.0 benchmark shows no meaningful win over Nx on the real Klassivox
workspace, not a synthetic one." This document is the methodology that
benchmark will follow. It is written before culm's own binary has been
built or installed in this session, per that session's own constraint, so
every number below is a placeholder, marked as such, not a claim.

**No number in this document has been measured yet. Every figure is a
`TODO`.** Do not read a placeholder as an estimate.

---

## Why Klassivox, not a synthetic workspace

Decision 13 and decision 17 already settle this: Klassivox is culm's proving
fixture and the workspace the first-party `monorepo` preset is derived from.
Section 2 of [02-runner.md](02-runner.md) is explicit that the honest
comparison is against Nx specifically, since Klassivox already runs it, not
against a workspace built to make culm look good. A synthetic benchmark
answers "is culm's graph algorithm fast," which nobody doubts; the question
that matters is whether a real polyglot workspace with a real Nx
configuration, real caches, and real developer habits already built around
it, is actually better served by culm.

Klassivox's current shape, read directly from the repository (never
modified to produce these figures, per decision 13):

| | Count |
| --- | --- |
| `apps/` | 4 (`klassivox-backend`, `klassivox-mobile`, `klassivox-web`, `landing-page`) |
| `libs/` | 1 (`shared`) |
| `packages/` | 1 (`klassivox-ui`) |
| `pnpm-lock.yaml` | 30,390 lines |
| `Cargo.lock` | 2,081 lines |

Nx is configured there through `@nx/js/typescript`, `@nx/eslint`, and
`@nx/vite` inferred plugins (`nx.json`), not hand-written `project.json`
targets, which matters for the comparison: culm is being measured against
Nx's own inference-based task detection at its most convenient, not a
worst-case hand-rolled Nx config a competitor could point to as unfair.

---

## What is measured

Three workloads, each run against both Nx (as Klassivox already has it
configured) and culm (once `runner/nx`'s config is superseded by culm's own
backend reaching parity, per decision 3 nothing about this benchmark
requires removing Nx as an option, only comparing against it):

1. **Cold run.** No cache anywhere, a fresh clone. `nx run-many --target=build`
   versus `culm run build --parallel` across every project. This is the
   floor: a tool cannot win on caching what it also loses on scheduling.
2. **Warm run, no changes.** Every task should be a full cache hit. This is
   where docs/02-runner.md section 1's own stated risk lives: a cache that
   is *fast* and *wrong* is worse than one that is merely slow, so a warm
   run's outputs are diffed byte-for-byte against the cold run's outputs
   before its timing counts for anything. A fast wrong answer is not a
   result, it is a bug this document refuses to report as a win.
3. **Warm run, one file changed.** A single source file touched in one leaf
   package (`packages/klassivox-ui`, chosen because it is a real dependency
   of more than one app, so this also exercises "downstream tasks reran,
   unrelated ones did not," `--affected`'s whole reason to exist).

Each workload is run five times per tool, discarding the first as a cache
and filesystem warm-up artifact for the *other* four, and the median of the
remaining four is reported, not the mean, so one slow run caused by an
unrelated background process on the machine does not skew the number either
tool is compared on.

## What is reported

For each of the three workloads, per tool:

- Wall-clock time, median of four runs (methodology above).
- Cache hit rate (tasks skipped over tasks eligible to be skipped).
- Whether the warm run's output bytes matched the cold run's output bytes
  exactly, task by task. Any mismatch is reported as a correctness failure,
  not folded into the timing number, and is disqualifying on its own per
  section 10's kill criterion regardless of how fast it was.
- Peak memory of the orchestrating process itself, since a runner that wins
  on wall clock by spawning every task at once and starving the machine is
  not actually the better daily-driver experience docs/02-runner.md section
  2 is honest about competing on.

## What is deliberately not claimed

- **Not a general "culm vs Nx" claim.** One workspace, one point in time,
  with the specific Rust-plus-JS mix Klassivox happens to have. Section 2 of
  [02-runner.md](02-runner.md) already says the honest wedge, if any, is
  narrow: cache-key correctness for a genuinely mixed toolchain, not general
  superiority.
- **Not a claim about Nx's own ceiling.** Nx improving its own non-JS
  inference is one of section 10's named kill criteria for the runner half
  of this project entirely; this benchmark is a snapshot, not a claim that
  the gap it measures is permanent.
- **No number is published without the methodology above run in full**,
  including the byte-for-byte output check on every warm run. A number
  without that check passing is not evidence the cache is correct, only that
  it was fast, and speed without correctness is exactly the failure mode
  this whole document exists to guard against reporting.

---

## Status

`TODO`, not yet measured. This document exists so the methodology is fixed
and reviewable before the first number is collected, rather than being
written to fit whatever number came out first. Per this session's own
constraint, no `cargo build` was run and culm was not installed, so no
measurement in this document has been attempted yet.
