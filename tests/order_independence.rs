// === Merge order independence

use std::collections::BTreeMap;

use culm::merge::{Contribution, FileSet, Layer};
use proptest::prelude::*;
use serde_json::{Map, Value};

/* Sections and keys are drawn from small fixed pools so that generated layers
 * genuinely collide on shared objects and shared arrays. A generator that gave
 * every layer its own private subtree would pass this property without ever
 * exercising the merge. */
const SECTIONS: [&str; 3] = ["scripts", "dependencies", "compilerOptions"];
const ARRAY_ENTRIES: [&str; 5] = ["dist", "bin", "README.md", "src", "LICENSE"];
const IGNORE_LINES: [&str; 5] = ["node_modules/", "target/", "dist/", ".env", "coverage/"];
/* Two sequence names, so the property covers layers colliding inside one
 * sequence and layers contributing to different sequences of the same file. */
const YAML_KEYS: [&str; 2] = ["packages", "onlyBuiltDependencies"];
const YAML_ENTRIES: [&str; 5] = ["apps/*", "libs/*", "packages/*", "esbuild", "sharp"];

/* Numbers straddling 2^53, the point where f64 stops being able to tell adjacent
 * integers apart. The middle three collapse onto the same f64, so a comparator
 * that ordered numbers by f64 alone called them equal, and a stable sort then
 * leaked layer order into the rendered bytes. The original pools held only small
 * strings, so the property could never reach this and the bug survived a passing
 * test suite. See the total order invariant in docs/CONTEXT.md. */
const NUMBERS: [u64; 6] = [
    0,
    1,
    9_007_199_254_740_992,
    9_007_199_254_740_993,
    9_007_199_254_740_994,
    u64::MAX,
];

/// One entry a layer contributes to the shared `files` array.
/// Mixing kinds matters, because the comparator has to order across types as well as within them.
#[derive(Debug, Clone)]
enum Entry {
    Text(usize),
    Number(usize),
}

impl Entry {
    fn value(&self) -> Value {
        match self {
            Self::Text(slot) => Value::String(ARRAY_ENTRIES[*slot].to_owned()),
            Self::Number(slot) => Value::Number(NUMBERS[*slot].into()),
        }
    }
}

fn entry_strategy() -> impl Strategy<Value = Entry> {
    prop_oneof![
        (0..ARRAY_ENTRIES.len()).prop_map(Entry::Text),
        (0..NUMBERS.len()).prop_map(Entry::Number),
    ]
}

/// One layer's generated contributions, before they are turned into a `Layer`.
#[derive(Debug, Clone)]
struct Plan {
    section: usize,
    /* Scalar leaves are namespaced by the layer's index, because the property
     * under test is order independence of successful merges. A generator that
     * could emit a genuine scalar conflict would be testing error reporting
     * instead, which the unit tests cover. */
    scalars: Vec<(usize, String)>,
    nested: Vec<(usize, String)>,
    array: Vec<Entry>,
    lines: Vec<usize>,
    yaml: Vec<(usize, usize)>,
}

fn plan_strategy() -> impl Strategy<Value = Plan> {
    (
        0..SECTIONS.len(),
        prop::collection::vec((0usize..4, "[a-z]{1,4}"), 0..4),
        prop::collection::vec((0usize..4, "[a-z]{1,4}"), 0..4),
        prop::collection::vec(entry_strategy(), 0..4),
        prop::collection::vec(0..IGNORE_LINES.len(), 0..4),
        prop::collection::vec((0..YAML_KEYS.len(), 0..YAML_ENTRIES.len()), 0..4),
    )
        .prop_map(|(section, scalars, nested, array, lines, yaml)| Plan {
            section,
            scalars,
            nested,
            array,
            lines,
            yaml,
        })
}

fn build_layer(index: usize, plan: &Plan) -> Layer {
    let id = format!("layer-{index:02}");
    let section = SECTIONS[plan.section];

    let mut section_body = Map::new();
    for (slot, value) in &plan.scalars {
        section_body.insert(format!("k{index}_{slot}"), Value::String(value.clone()));
    }

    let mut nested_body = Map::new();
    for (slot, value) in &plan.nested {
        nested_body.insert(format!("n{index}_{slot}"), Value::String(value.clone()));
    }
    if !nested_body.is_empty() {
        /* A shared nested object under a shared section, so deep recursion and
         * subtree provenance are both on the path. */
        section_body.insert("paths".to_owned(), Value::Object(nested_body));
    }

    let mut root = Map::new();
    if !section_body.is_empty() {
        root.insert(section.to_owned(), Value::Object(section_body));
    }
    if !plan.array.is_empty() {
        let entries: Vec<Value> = plan.array.iter().map(Entry::value).collect();
        root.insert("files".to_owned(), Value::Array(entries));
    }
    /* Every layer agrees on this scalar, which exercises the identical-value
     * agreement path rather than leaving it untouched by the property. */
    root.insert("license".to_owned(), Value::String("MIT".to_owned()));

    let lines: Vec<String> = plan
        .lines
        .iter()
        .map(|slot| IGNORE_LINES[*slot].to_owned())
        .collect();

    let mut yaml: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (key, entry) in &plan.yaml {
        yaml.entry(YAML_KEYS[*key].to_owned())
            .or_default()
            .push(YAML_ENTRIES[*entry].to_owned());
    }

    Layer::new(id)
        .with_merge("package.json", Contribution::JsonDeep(Value::Object(root)))
        .with_merge(".gitignore", Contribution::LineBlock(lines))
        .with_merge(
            "pnpm-workspace.yaml",
            Contribution::YamlList {
                lists: yaml,
                scalars: BTreeMap::new(),
            },
        )
}

/* Every permutation, not a sample: with at most five layers the full set is 120
 * orderings, which is cheap enough to be exhaustive per case. */
fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    if items.is_empty() {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let mut rest: Vec<T> = items.to_vec();
        rest.remove(index);
        for mut tail in permutations(&rest) {
            let mut one = vec![item.clone()];
            one.append(&mut tail);
            out.push(one);
        }
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Applying the same layers in any order produces byte-identical output.
    #[test]
    fn rendering_is_identical_under_every_permutation(
        plans in prop::collection::vec(plan_strategy(), 2..=5)
    ) {
        let layers: Vec<Layer> = plans
            .iter()
            .enumerate()
            .map(|(index, plan)| build_layer(index, plan))
            .collect();

        let orders = permutations(&layers);
        prop_assert!(orders.len() >= 2);

        let baseline = FileSet::build(layers.iter())
            .expect("generated layers never conflict")
            .render_all();

        for order in &orders {
            let rendered = FileSet::build(order.iter())
                .expect("generated layers never conflict")
                .render_all();
            prop_assert_eq!(&rendered, &baseline);
        }
    }

    /// The merged JSON is not merely stable, it actually contains every layer's contribution.
    #[test]
    fn every_layer_reaches_the_output(
        plans in prop::collection::vec(plan_strategy(), 2..=5)
    ) {
        let layers: Vec<Layer> = plans
            .iter()
            .enumerate()
            .map(|(index, plan)| build_layer(index, plan))
            .collect();

        let set = FileSet::build(layers.iter()).expect("generated layers never conflict");
        let rendered: BTreeMap<_, _> = set.render();
        let gitignore = rendered
            .get(std::path::Path::new(".gitignore"))
            .expect("every generated layer contributes a line block");

        for index in 0..layers.len() {
            let marker = format!("# layer:layer-{index:02}");
            prop_assert!(gitignore.contains(&marker));
        }
    }
}
