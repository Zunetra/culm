# 02 - The Runner

A task graph and a content-addressed build cache for a repository that
mixes languages. Ships after the scaffolder. Higher risk than anything
else in this repository, deliberately documented as such.

Derived from `Klassivox/docs/Projects/05-task-runner.md`. The kill
criteria in section 10 are preserved from that document unchanged, per
[CONTEXT.md](CONTEXT.md#decisions) decision 13.

---

## 1. The problem

A polyglot monorepo needs to know what changed, what depends on what, and
what can be skipped.

The failure mode that matters most is **a wrong cache hit**. Skipping a
task that should have run produces a subtly broken build that ships, and
by the time anyone notices, the cache has served the same wrong artifact
to every other developer and every CI run. That is worse than no cache,
because it is *trusted* wrongness.

## 2. Competitive landscape, honestly

This section is more hedged than anything in
[01-scaffolder.md](01-scaffolder.md), on purpose. The honest read:
**this may be the half without a real gap**, and that possibility should
stay in view rather than be argued away.

- **Nx.** The daily point of comparison, since Klassivox runs it.
  Excellent plugins, excellent DX, genuinely good inference-based task
  detection, and a company with every incentive to keep investing. The
  gap, if any, is narrow: its non-JS support is real but visibly
  secondary to a TypeScript-first design centre.
- **Turborepo.** Same shape, smaller plugin surface, same JS-first centre
  of gravity.
- **Bazel.** Correct, enormous, and not a fair fight on correctness. The
  gap culm could occupy is "Bazel's guarantees without Bazel's
  operational weight," which is exactly the gap Moon and Pants already
  chase with more resources.
- **Moon.** Rust-based, explicitly polyglot, explicitly positioned as
  approachable Bazel-like correctness. The single closest existing
  project, built by people further along the same path. **Read Moon in
  full before writing a line of this.** If Moon already does this well,
  the honest move is contributing there.
- **Pants.** Similar positioning, Python roots, mature remote caching.

**The gap, stated as honestly as the evidence supports:** there may not
be a product-shaped one. What there is instead is firsthand pain with a
specific, well-understood workload, hit daily. That is not a market gap,
it is an unfair advantage at understanding the problem precisely.

If there is a real wedge, it is narrow: **cache-key correctness for a
genuinely mixed Rust-plus-JS toolchain**, getting hashing right across
`Cargo.lock`, `pnpm-lock.yaml`, source files, and environment inputs
simultaneously, in a way a JS-first plugin model makes awkward. That is a
real, narrow, defensible thing to be right about. It is not "a better
Nx."

## 3. Interface

```
culm run <task>               run a task, respecting the dependency graph
culm run <task> --affected    only tasks touched by changes since a ref
culm cache stats|clear        inspect or clear the local cache
culm graph                    print or visualize the task graph
```

Configuration is a plain file describing tasks and their inputs and
outputs, not a plugin system that has to understand every build tool in
existence.

```toml
# culm.toml, one per project in the workspace

[tasks.build]
command = "cargo build --release"
inputs = ["src/**/*.rs", "Cargo.toml", "Cargo.lock"]
outputs = ["target/release/backend"]
cache = true

[tasks.test]
command = "cargo nextest run"
inputs = ["src/**/*.rs", "tests/**/*.rs", "Cargo.toml"]
depends_on = ["build"]
cache = true
```

**culm does not know what `cargo` or `pnpm` or `vite` are.** It knows a
command, a set of input globs, and a set of output paths. Correctness
comes from that narrowness, not from plugins that have to keep up with
every build tool's own changes.

## 4. Architecture

```
┌────────────────────────────────────────────┐
│  culm.toml (per project) + workspace root │
│    parsed into a task graph                │
│  Task graph (petgraph)                     │
│    topological order + change detection    │
│  For each task:                            │
│    1. hash inputs (content, not mtime)     │
│    2. check local + remote cache           │
│    3. hit  -> restore outputs              │
│       miss -> run command, store result    │
└────────────────────────────────────────────┘
```

Two decisions carry the entire correctness story.

**Hash content, never mtime.** A cache keyed on modification time is
wrong the moment CI does a fresh clone, where every mtime becomes "now",
or a developer switches branches, where mtimes change and content may
not.

**The cache key includes everything, or it is not a cache, it is a bug
waiting to happen.** Source files, lockfiles, the command string itself,
explicitly declared environment variables (never all of `env`), and the
tool's own version. Too narrow and a real change produces a stale hit.
Too broad and the cache never hits, which is merely disappointing rather
than dangerous.

```rust
// === Cache key

/// Serialized deterministically, with sorted keys and fixed formatting,
/// before hashing, so two identical logical inputs always produce
/// byte-identical output regardless of hash-map iteration order.
pub struct CacheKey {
    pub command: String,
    pub input_hashes: BTreeMap<PathBuf, Hash>,  // sorted: order must be deterministic
    pub declared_env: BTreeMap<String, String>, // only vars the task explicitly lists
    pub tool_version: String,
}

impl CacheKey {
    /// A cache miss must always be safe. A cache hit must always be
    /// correct. This function only ever gains inputs over time, never
    /// loses them, and every addition is a reviewed decision rather than
    /// an inference.
    pub fn digest(&self) -> Hash {
        cas::hash(&self.canonical_bytes())
    }
}
```

## 5. Storage, and what `cas` is

A **content-addressable store**. It hashes a file's bytes, stores the
file under that hash as its name, and returns the hash. Ask for the hash
later and you get exactly those bytes back, or an error if they are
missing or corrupted. Two identical files stored twice occupy one slot,
because identical bytes hash identically.

A build cache is exactly that shape: "given these inputs, here are the
output bytes" is a lookup keyed by a hash of the inputs.

```rust
pub struct Task {
    pub name: String,
    pub project: String,
    pub command: String,
    pub inputs: Vec<GlobPattern>,
    pub outputs: Vec<PathBuf>,
    pub depends_on: Vec<TaskRef>,
    pub cacheable: bool,
    pub declared_env: Vec<String>,
}

pub struct CacheEntry {
    pub key: Hash,
    pub outputs: Vec<(PathBuf, Hash)>,  // each output stored content-addressed
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
    pub duration_ms: u64,
}
```

`cas` lives inside this repository as a module, unpublished. Per
[CONTEXT.md](CONTEXT.md#decisions) decision 11, it is extracted to its own
crate only if a second project genuinely needs it.

## 6. Milestones

Numbered from the runner's own start, which follows the scaffolder's
v1.0.

**r0.1 (2 weeks).** `culm run <task>`, sequential execution respecting
`depends_on`, no caching. Just the graph and correct ordering.

**r0.2 (2 weeks).** Content-based hashing, local cache via `cas`,
hit and miss reporting. **This milestone either proves or kills the
premise.** Run it against a real subset of the Klassivox workspace and
check every cache hit by hand against a fresh build for at least two
weeks of real use before writing another line.

**r0.3 (2 weeks).** `--affected` via `git diff` against a base ref, then
walking the graph for downstream tasks. Parallel execution respecting the
dependency graph.

**r0.4 (3 weeks).** Remote cache: a thin HTTP store or an S3-compatible
bucket, so CI and every developer share hits.

**r1.0 (3 weeks, only if r0.2's honest check passed).** Multi-platform
binaries, `culm graph --format dot`, and a real side-by-side benchmark
against Nx on the actual Klassivox workspace, published with methodology
rather than a claimed number.

There is a deliberate checkpoint inside the milestones, not only at the
end. **r0.2 is where this half earns the right to continue.**

## 7. Open-core line

**Free forever.** The runner, local caching, the full graph,
`--affected`, unlimited projects and tasks. A single team runs this fully
and pays nothing.

| Paid | Why fair |
| --- | --- |
| Hosted remote cache | Real storage and bandwidth cost |
| Distributed execution across machines | Real orchestration infrastructure |
| Team cache analytics, hit rate, time saved | Substantial extra product surface |
| SSO for the hosted cache | Only matters past roughly 50 engineers |

Never charge for a correct cache. Only for someone else hosting and
operating it.

## 8. Crates

```toml
[dependencies]
tokio       = { version = "1", features = ["full"] }
petgraph    = "0.7"          # task dependency graph
clap        = { version = "4", features = ["derive", "env"] }
serde       = { version = "1", features = ["derive"] }
toml        = "1"            # culm.toml, same crate the scaffolder already uses
globset     = "0.4"          # input glob matching
ignore      = "0.4"          # gitignore-aware walking, the ripgrep crate
blake3      = "1"            # fast content hashing, feeds the cas module
git2        = "0.19"         # --affected: diff against a base ref
thiserror   = "2"
anyhow      = "1"
tracing     = "0.1"
indicatif   = "0.17"         # live task progress, matters for daily-driver feel

[dev-dependencies]
tempfile    = "3"
insta       = "1"
proptest    = "1"            # property-test cache key determinism directly
```

## 9. Correctness testing

```yaml
# .github/workflows/ci.yml (excerpt)
- run: cargo fmt --all -- --check
- run: cargo clippy --all-targets -- -D warnings
- run: cargo nextest run
- run: cargo audit
- run: cargo deny check licenses advisories
# Runs culm twice against fixture workspaces and asserts the second
# run's outputs are byte-identical to the first, rather than trusting
# that it reported a cache hit.
- run: cargo test --test cache_correctness -- --test-threads=1
```

The `cache_correctness` job does not trust culm's own "cache hit" log
line. It re-runs the task for real on a miss path and diffs the actual
output bytes against what the cache served, because the failure mode
named throughout this document is exactly a tool that reports a hit while
quietly serving something stale. Single-threaded because these tests
exercise real subprocess execution and filesystem state and must not
interleave.

## 10. Kill criteria

Preserved from the original document. Do not soften these.

- **r0.2's honest two-week check finds even one incorrect cache hit** a
  human would not have caught without deliberately looking for it. A
  build cache that is wrong sometimes is worse than no cache, and this is
  where that principle gets tested against reality rather than left as a
  nice sentence.
- **Moon or Pants closes the gap** named in section 2 with real adoption
  before this reaches r0.3. They have more resources and more runway on
  exactly this thesis; the honest move is switching rather than
  competing.
- **Nx's non-JS support improves enough** that the daily pain motivating
  this stops being felt on the Klassivox workspace. If the incumbent
  stops hurting, the dogfooding premise evaporates, because there is no
  more pain to be an expert in.
- **The r1.0 benchmark shows no meaningful win over Nx** on the real
  Klassivox workspace, not a synthetic one. "Roughly the same speed, but
  self-built" is not a reason for anyone else to adopt it.
- **Any point before r0.2 where writing `culm.toml` feels like more work
  than the Nx configuration already in place.** If the replacement is not
  obviously less annoying than the incumbent on day one, on the one
  workspace with perfect information, it will not be less annoying
  anywhere else.

The default expectation going in should be that this half **might stop at
r0.2**, and that stopping there, having learned graph algorithms and
content hashing and confirmed the market is too crowded to win, is
success rather than failure.
