# Internals

How the merge engine and the scaffold pipeline work, and how to use culm as
a Rust library. You do not need any of this to use culm; it is for anyone
changing culm itself. Back to the [README](../README.md).

- [The merge engine](#-the-merge-engine)
- [Scaffolding](#-scaffolding)
- [Using the library](#-using-the-library)

## 🔀 The merge engine

This is the actual product. Everything else copies files.

The problem: N independent layers each contribute fragments to the same
shared files, in any selection order, and the result must be identical
every time and must be a valid file.

```
  layers/frontend/nuxt/layer.toml ─┐
  layers/quality/eslint/layer.toml ├─→  Layer  ─→  FileSet  ─→  render()  ─→  bytes
  layers/toolchain/node-pnpm/…     ─┘   (merges)    (in memory)              (on disk, once)
                                            │
                                            └─→  MergeError, naming both layers
```

Nothing reaches disk until the whole run has validated. A partial
scaffold is worse than a failed one: a failed one leaves nothing to clean
up.

### Strategies

| Strategy | Typical target | Behaviour |
| --- | --- | --- |
| `json-deep` | `package.json`, `tsconfig.base.json` | Objects merge recursively, arrays union then sort then dedupe, matching scalars agree, differing scalars are an error |
| `line-block` | `.gitignore`, `.env.example` | One `# layer:<id>` delimited block per layer, blocks sorted by layer id, lines sorted within a block |
| `toml-members` | `Cargo.toml` | Crate paths unioned into `[workspace].members`, sorted, deduped |
| `yaml-list` | `pnpm-workspace.yaml` | Named string lists unioned and sorted; a blank entry is dropped rather than written |
| `yaml-steps` | `.github/workflows/ci.yml` | A fixed workflow skeleton; each layer contributes its own steps |
| `hook-script` | `.husky/*` | One shell hook assembled from every layer's body, written executable (`0755`) |

A target is claimed by exactly one strategy. A second layer claiming it
under another is a `StrategyMismatch` error rather than a guess.

### Rendered output

`json-deep` renders pretty-printed JSON with keys sorted, ending in a
newline:

```json
{
  "name": "app",
  "version": "0.1.0"
}
```

`line-block` renders one labelled block per layer, blank line separated:

```
# layer:toolchain/node-pnpm
.npm/
.pnpm-store/
node_modules/

# layer:toolchain/rust-cargo
*.pdb
**/*.rs.bk
/target
```

`toml-members` renders the workspace table with a fixed resolver:

```toml
[workspace]
resolver = "3"
members = [
    "app",
    "common",
]
```

`resolver` is fixed rather than contributed, because it follows from the
edition, and a layer choosing it would be a layer deciding something for
the whole workspace.

### Why collisions are errors

If two layers set `scripts.build` to different strings, the scaffold
stops and names both. It does not pick one.

Last-write-wins would make the output depend on selection order, which
destroys the one property this engine exists to guarantee. Worse, it is
how a generated workspace ends up subtly wrong in a way nobody traces
back for a week. The same reasoning governs `FileCollision` in the
scaffold module: two layers writing different content to one destination
is an error, while two layers writing identical content is not, because a
shared file written twice is the same file either way.

Set-shaped merges are the exception, and only where the merge genuinely
is a set. Two layers naming the same Cargo member is agreement, because a
member list is a set. Two layers naming the same array element is
agreement, because arrays are unioned. A scalar has no such reading.

### Merge errors

| Error | Meaning |
| --- | --- |
| `ScalarConflict` | Two layers set the same key to different values. Names both layers and the dotted key |
| `TypeConflict` | Two layers put incompatible kinds at the same path, such as an object against a string |
| `StrategyMismatch` | Two layers claim one target under different strategies |
| `NonObjectRoot` | A `json-deep` contribution whose root is not an object |
| `MissingRequirement` | A selected layer needs a layer that was not selected |
| `Conflicting` | Two selected layers declare each other mutually exclusive |

Every conflict variant names both culprits, because "conflict in
package.json" without the culprits is useless to whoever hits it. The
conflict report is also order independent: the reported pair is sorted,
so the message does not change with selection order.

### Provenance

`json-deep` records which layer set each node, keyed by dotted path, and
records every node of a subtree inserted in one go. Without that, a deep
conflict names whichever layer owns the ancestor rather than the layer
that actually set the offending leaf.

The regression test for this is
`deep_conflict_names_the_layer_that_set_the_leaf`.

### Order independence

**Applying layers `{a, b, c}` in any permutation produces byte-identical
output.** This is a property test, not a hope.

`tests/order_independence.rs` generates two to five layers, builds every
permutation of them, renders each, and asserts the bytes match. A second
property asserts that every layer actually reaches the output, so the
first property cannot be satisfied by rendering nothing.

Sorted output everywhere is what makes this testable by byte comparison,
which is why `FileSet` uses a `BTreeMap` and `LineBlockDoc` keys its
blocks by layer id.

### The total order invariant, learned the hard way

Any comparator the engine sorts with must be a **total order consistent
with equality**. It may report `Equal` only when the values genuinely are
equal.

This was violated once. Array entries are sorted so output does not
depend on layer order, and the comparator ordered JSON numbers by
converting both to `f64`. An `f64` carries 53 bits of precision, so
integers above 2^53 no longer map one to one, and distinct integers
collapsed onto the same `f64`. The comparator then reported `Equal` for
numbers that were not equal, Rust's `sort_by` is stable, so it left them
in arrival order, and arrival order is layer order. The rendered bytes
changed depending on which layer was applied first.

The fix breaks the tie on the exact textual form of the number. The
regression test is
`integers_beyond_f64_precision_still_sort_deterministically`.

Two lessons worth carrying:

- The property test did not catch it. Its generators drew from small
  value pools and could never produce two numbers that collapse. **A
  property test is only as strong as the inputs it can reach.**
- The failure was silent, and silent wrong output is the same category as
  the runner's worst failure mode.

## 🏗️ Scaffolding

The merge engine decides what shared files contain. The `scaffold` module
decides what private files a layer copies and where they land. They are
deliberately separate, because conflating them would put path safety and
template interpolation inside the one module whose correctness property
is byte-for-byte determinism.

### The pipeline

```
  Manifest ─→ CopyPlan::build ─→ CopyPlan ─┐
                (reads files/,             ├─→ scaffold::write(root, plan, merged)
                 interpolates,             │        (creates dirs, writes, chmods)
                 normalises paths)         │
  Layer ────→ FileSet::build ──→ render() ─┘
```

### Interpolation

Templates use `{{name}}`. Whitespace inside the braces is tolerated, so
`{{ project_name }}` and `{{project_name}}` are the same placeholder.

| Input | Variables | Output |
| --- | --- | --- |
| `name = "{{project_name}}"` | `project_name=klassivox` | `name = "klassivox"` |
| `{{ project_name }}` | `project_name=klassivox` | `klassivox` |
| `{{typo}}` | `project_name=klassivox` | `{{typo}}`, left literal |
| `fn main() {{ let x = 1;` | any | unchanged, an unterminated `{{` is content |

An unknown placeholder is left exactly as written rather than blanked. A
silently empty string in a generated config is far harder to notice than
a literal `{{typo}}`.

The variable set is closed rather than arbitrary user data, so a template
cannot come to depend on something the wizard does not ask.

### Path containment

A layer is data, and a scaffolder that lets data write outside the
directory it was pointed at is a scaffolder that can overwrite anything
the user can. Destinations are normalised before use:

| Destination | Result |
| --- | --- |
| `src/main.rs` | `src/main.rs` |
| `./bacon.toml` | `bacon.toml` |
| `../../.ssh/authorized_keys` | refused, `EscapesRoot` |
| `src/../../etc/passwd` | refused, `EscapesRoot` |
| `/etc/passwd` | refused, `EscapesRoot` |
| `` or `./` | refused, `EscapesRoot` |

### Scaffold errors

| Error | Meaning |
| --- | --- |
| `TemplateMissing` | A `[[files]]` entry names a file the layer directory does not hold |
| `FileCollision` | Two layers write different content to one destination |
| `EscapesRoot` | A destination that is absolute, rooted, or climbs out |
| `Io` | The write itself failed, reported with the path |

## 🧪 Using the library

Everything the CLI does is also available as a library. The core public
surface is `culm::layer`, `culm::merge`, and `culm::scaffold`.

Merging a selection:

```rust
use culm::merge::{Contribution, FileSet, Layer};
use serde_json::json;

let node = Layer::new("toolchain/node-pnpm")
    .with_merge("package.json", Contribution::JsonDeep(json!({ "private": true })))
    .with_merge(
        ".gitignore",
        Contribution::LineBlock(vec!["node_modules/".to_owned()]),
    );

let nuxt = Layer::new("frontend/nuxt")
    .requiring("toolchain/node-pnpm")
    .with_merge(
        "package.json",
        Contribution::JsonDeep(json!({ "scripts": { "dev:nuxt": "nuxt dev" } })),
    );

let set = FileSet::build([&node, &nuxt])?;
for (path, body) in set.render() {
    println!("--- {}", path.display());
    print!("{body}");
}
```

Parsing and validating a selection of real manifests:

```rust
use culm::layer::{Manifest, validate_selection};

let manifest = Manifest::parse("layers/frontend/nuxt/layer.toml", source)?;
assert_eq!(manifest.qualified_id().as_str(), "frontend/nuxt");

validate_selection(&selection)?;
```

`FileSet::build` validates requirements and conflicts across the whole
selection before anything merges, so a doomed run cannot produce half a
file set.
