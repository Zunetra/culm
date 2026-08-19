// === json-deep

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde_json::{Map, Value};

use super::{LayerId, MergeError};

/// The kind of a JSON value, as a type conflict message names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueKind {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Null => "null",
            Self::Boolean => "a boolean",
            Self::Number => "a number",
            Self::String => "a string",
            Self::Array => "an array",
            Self::Object => "an object",
        })
    }
}

/// A JSON file accumulated from many layers under the json-deep strategy.
/// Objects merge recursively, arrays union then sort then dedupe, and two layers setting the same scalar to different values is an error.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonDoc {
    value: Value,
    /* Which layer set each node, keyed by dotted path. Without this, a deep
     * conflict names whichever layer happened to be applied first rather than
     * the one that actually set the offending leaf. */
    provenance: BTreeMap<String, LayerId>,
    first_owner: LayerId,
}

impl JsonDoc {
    /// Starts a document from the first layer to claim the target.
    pub(super) fn seed(target: &Path, layer: &LayerId, value: &Value) -> Result<Self, MergeError> {
        let object = as_object(target, layer, value)?;
        let mut provenance = BTreeMap::new();
        record_object(&mut provenance, "", object, layer);
        Ok(Self {
            value: Value::Object(object.clone()),
            provenance,
            first_owner: layer.clone(),
        })
    }

    /// Merges one more layer's contribution into the document.
    pub(super) fn merge(
        &mut self,
        target: &Path,
        layer: &LayerId,
        value: &Value,
    ) -> Result<(), MergeError> {
        let incoming = as_object(target, layer, value)?.clone();
        /* Merged into a clone so a conflict halfway through leaves the document
         * untouched, matching the all-or-nothing rule for the run as a whole. */
        let mut merged = self.value.clone();
        let mut provenance = self.provenance.clone();
        {
            let destination = merged
                .as_object_mut()
                .expect("a JsonDoc always holds an object at its root");
            merge_object(target, "", destination, &incoming, layer, &mut provenance)?;
        }
        self.value = merged;
        self.provenance = provenance;
        Ok(())
    }

    /// The layer that first claimed this target.
    pub(super) fn first_owner(&self) -> LayerId {
        self.first_owner.clone()
    }

    /// Borrows the merged JSON value.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Renders the document.
    /// Keys come out sorted because serde_json's default `Value` backs objects with a BTreeMap, which is exactly the ordering guarantee wanted here.
    pub fn render(&self) -> String {
        let mut out = serde_json::to_string_pretty(&self.value)
            .expect("a serde_json::Value always serialises");
        out.push('\n');
        out
    }
}

// == Merging

fn as_object<'v>(
    target: &Path,
    layer: &LayerId,
    value: &'v Value,
) -> Result<&'v Map<String, Value>, MergeError> {
    value.as_object().ok_or_else(|| MergeError::NonObjectRoot {
        target: target.to_path_buf(),
        layer: layer.clone(),
    })
}

fn merge_object(
    target: &Path,
    prefix: &str,
    destination: &mut Map<String, Value>,
    incoming: &Map<String, Value>,
    layer: &LayerId,
    provenance: &mut BTreeMap<String, LayerId>,
) -> Result<(), MergeError> {
    for (key, source) in incoming {
        let path = join(prefix, key);
        match destination.get_mut(key) {
            Some(existing) => merge_value(target, &path, existing, source, layer, provenance)?,
            None => {
                destination.insert(key.clone(), source.clone());
                record(provenance, &path, source, layer);
            }
        }
    }
    Ok(())
}

fn merge_value(
    target: &Path,
    path: &str,
    destination: &mut Value,
    source: &Value,
    layer: &LayerId,
    provenance: &mut BTreeMap<String, LayerId>,
) -> Result<(), MergeError> {
    match (destination, source) {
        (Value::Object(destination), Value::Object(source)) => {
            merge_object(target, path, destination, source, layer, provenance)
        }
        (Value::Array(destination), Value::Array(source)) => {
            destination.extend(source.iter().cloned());
            destination.sort_by(compare);
            destination.dedup();
            Ok(())
        }
        (destination, source) if kind(destination) != kind(source) => {
            Err(MergeError::TypeConflict {
                target: target.to_path_buf(),
                key: path.to_owned(),
                a: owner(provenance, path),
                b: layer.clone(),
                a_type: kind(destination),
                b_type: kind(source),
            })
        }
        /* Same kind of scalar. Agreement is fine, disagreement is never resolved
         * silently: last-write-wins is how a generated workspace ends up subtly
         * wrong in a way nobody traces back for a week. */
        (destination, source) => {
            if destination == source {
                Ok(())
            } else {
                Err(MergeError::ScalarConflict {
                    target: target.to_path_buf(),
                    key: path.to_owned(),
                    a: owner(provenance, path),
                    b: layer.clone(),
                })
            }
        }
    }
}

// == Provenance

/* A whole subtree inserted at once still needs every node recorded, otherwise a
 * later deep conflict names the layer that owns the ancestor rather than the
 * layer that set the leaf. */
fn record(provenance: &mut BTreeMap<String, LayerId>, path: &str, value: &Value, layer: &LayerId) {
    provenance
        .entry(path.to_owned())
        .or_insert_with(|| layer.clone());
    if let Value::Object(object) = value {
        record_object(provenance, path, object, layer);
    }
}

fn record_object(
    provenance: &mut BTreeMap<String, LayerId>,
    prefix: &str,
    object: &Map<String, Value>,
    layer: &LayerId,
) {
    for (key, value) in object {
        record(provenance, &join(prefix, key), value, layer);
    }
}

fn owner(provenance: &BTreeMap<String, LayerId>, path: &str) -> LayerId {
    provenance
        .get(path)
        .cloned()
        .unwrap_or_else(|| LayerId::new("unknown"))
}

fn join(prefix: &str, key: &str) -> String {
    if prefix.is_empty() {
        key.to_owned()
    } else {
        format!("{prefix}.{key}")
    }
}

// == Ordering

fn kind(value: &Value) -> ValueKind {
    match value {
        Value::Null => ValueKind::Null,
        Value::Bool(_) => ValueKind::Boolean,
        Value::Number(_) => ValueKind::Number,
        Value::String(_) => ValueKind::String,
        Value::Array(_) => ValueKind::Array,
        Value::Object(_) => ValueKind::Object,
    }
}

fn rank(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(_) => 2,
        Value::String(_) => 3,
        Value::Array(_) => 4,
        Value::Object(_) => 5,
    }
}

/* A total order over JSON values, so array union can sort. serde_json::Value is
 * only PartialOrd-less because of floats, and NaN cannot reach here from parsed
 * or constructed layer manifests. */
fn compare(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        (Value::Number(a), Value::Number(b)) => {
            let numeric = match (a.as_f64(), b.as_f64()) {
                (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(Ordering::Equal),
                _ => Ordering::Equal,
            };
            /* Integers above 2^53 collapse onto the same f64, so a numeric-only
             * comparison reports Equal for values that are not equal. A stable
             * sort then leaves them in input order, and the rendered bytes depend
             * on the order layers were applied, which breaks the one property
             * this engine exists to guarantee. Tie-breaking on the exact
             * representation keeps distinct numbers distinct. */
            numeric.then_with(|| a.to_string().cmp(&b.to_string()))
        }
        (Value::String(a), Value::String(b)) => a.cmp(b),
        (Value::Array(a), Value::Array(b)) => compare_slices(a, b),
        (Value::Object(a), Value::Object(b)) => {
            let a: Vec<(&String, &Value)> = a.iter().collect();
            let b: Vec<(&String, &Value)> = b.iter().collect();
            let mut pairs = a.iter().zip(b.iter());
            let found = pairs.find_map(|((ak, av), (bk, bv))| {
                match ak.cmp(bk).then_with(|| compare(av, bv)) {
                    Ordering::Equal => None,
                    other => Some(other),
                }
            });
            found.unwrap_or_else(|| a.len().cmp(&b.len()))
        }
        (a, b) => rank(a).cmp(&rank(b)),
    }
}

fn compare_slices(a: &[Value], b: &[Value]) -> Ordering {
    let found = a
        .iter()
        .zip(b.iter())
        .find_map(|(a, b)| match compare(a, b) {
            Ordering::Equal => None,
            other => Some(other),
        });
    found.unwrap_or_else(|| a.len().cmp(&b.len()))
}

// === Tests

#[cfg(test)]
mod tests {
    use super::ValueKind;
    use crate::merge::{Contribution, FileSet, Layer, LayerId, MergeError};
    use serde_json::{Value, json};
    use std::path::PathBuf;

    fn target() -> PathBuf {
        PathBuf::from("package.json")
    }

    fn layer(id: &str, value: Value) -> Layer {
        Layer::new(id).with_merge("package.json", Contribution::JsonDeep(value))
    }

    fn merged(layers: &[Layer]) -> Value {
        let set = FileSet::build(layers.iter()).unwrap();
        match set.get(&target()).unwrap() {
            super::super::FileContent::JsonDeep(doc) => doc.value().clone(),
            other => panic!("expected a json-deep document, got {other:?}"),
        }
    }

    /* Regression: these two differ but collapse onto the same f64, so a
     * numeric-only comparator called them Equal and the stable sort left them
     * in whatever order the layers arrived in. */
    #[test]
    fn integers_beyond_f64_precision_still_sort_deterministically() {
        let a = layer("a", json!({ "files": [9007199254740993u64] }));
        let b = layer("b", json!({ "files": [9007199254740992u64] }));
        assert_eq!(merged(&[a.clone(), b.clone()]), merged(&[b, a]));
    }

    #[test]
    fn objects_merge_recursively() {
        let a = layer("a", json!({ "scripts": { "build": "nx build" } }));
        let b = layer(
            "b",
            json!({ "scripts": { "test": "vitest" }, "name": "app" }),
        );
        assert_eq!(
            merged(&[a, b]),
            json!({
                "name": "app",
                "scripts": { "build": "nx build", "test": "vitest" }
            })
        );
    }

    #[test]
    fn arrays_union_then_sort_then_dedupe() {
        let a = layer("a", json!({ "files": ["dist", "README.md", "dist"] }));
        let b = layer("b", json!({ "files": ["bin", "dist"] }));
        assert_eq!(
            merged(&[a, b]),
            json!({ "files": ["README.md", "bin", "dist"] })
        );
    }

    #[test]
    fn identical_scalars_agree() {
        let a = layer("a", json!({ "license": "MIT" }));
        let b = layer("b", json!({ "license": "MIT" }));
        assert_eq!(merged(&[a, b]), json!({ "license": "MIT" }));
    }

    #[test]
    fn differing_scalars_conflict_and_name_both_layers() {
        let a = layer("prettier", json!({ "scripts": { "build": "tsc" } }));
        let b = layer("nx", json!({ "scripts": { "build": "nx build" } }));
        let err = FileSet::build([&a, &b]).unwrap_err();
        assert_eq!(
            err,
            MergeError::ScalarConflict {
                target: target(),
                key: "scripts.build".to_owned(),
                a: LayerId::new("prettier"),
                b: LayerId::new("nx"),
            }
        );
        let message = err.to_string();
        assert!(message.contains("prettier"), "{message}");
        assert!(message.contains("nx"), "{message}");
        assert!(message.contains("scripts.build"), "{message}");
    }

    #[test]
    fn deep_conflict_names_the_layer_that_set_the_leaf() {
        /* 'husky' plants the whole subtree in one insert, so the leaf's owner is
         * only known if the insert walked it. */
        let husky = layer(
            "husky",
            json!({ "compilerOptions": { "paths": { "@app/*": ["apps/app/src/*"] , "strict": true } } }),
        );
        let eslint = layer(
            "eslint",
            json!({ "compilerOptions": { "target": "es2022" } }),
        );
        let vitest = layer(
            "vitest",
            json!({ "compilerOptions": { "paths": { "strict": false } } }),
        );
        let err = FileSet::build([&husky, &eslint, &vitest]).unwrap_err();
        assert_eq!(
            err,
            MergeError::ScalarConflict {
                target: target(),
                key: "compilerOptions.paths.strict".to_owned(),
                a: LayerId::new("husky"),
                b: LayerId::new("vitest"),
            }
        );
    }

    #[test]
    fn mismatched_types_conflict() {
        let a = layer("a", json!({ "scripts": { "build": "tsc" } }));
        let b = layer("b", json!({ "scripts": { "build": ["tsc"] } }));
        assert_eq!(
            FileSet::build([&a, &b]).unwrap_err(),
            MergeError::TypeConflict {
                target: target(),
                key: "scripts.build".to_owned(),
                a: LayerId::new("a"),
                b: LayerId::new("b"),
                a_type: ValueKind::String,
                b_type: ValueKind::Array,
            }
        );
    }

    #[test]
    fn object_against_scalar_names_the_subtree_owner() {
        let a = layer("a", json!({ "scripts": { "build": "tsc" } }));
        let b = layer("b", json!({ "scripts": "none" }));
        assert_eq!(
            FileSet::build([&a, &b]).unwrap_err(),
            MergeError::TypeConflict {
                target: target(),
                key: "scripts".to_owned(),
                a: LayerId::new("a"),
                b: LayerId::new("b"),
                a_type: ValueKind::Object,
                b_type: ValueKind::String,
            }
        );
    }

    #[test]
    fn a_failed_merge_leaves_the_document_untouched() {
        let a = layer("a", json!({ "scripts": { "build": "tsc" } }));
        let b = layer(
            "b",
            json!({ "name": "app", "scripts": { "build": "nx build" } }),
        );
        let mut set = FileSet::new();
        set.apply(&a).unwrap();
        set.apply(&b).unwrap_err();
        match set.get(&target()).unwrap() {
            super::super::FileContent::JsonDeep(doc) => {
                assert_eq!(doc.value(), &json!({ "scripts": { "build": "tsc" } }));
            }
            other => panic!("expected a json-deep document, got {other:?}"),
        }
    }

    #[test]
    fn render_sorts_keys_and_ends_with_a_newline() {
        let a = layer("a", json!({ "version": "0.1.0", "name": "app" }));
        let set = FileSet::build([&a]).unwrap();
        let rendered = set.get(&target()).unwrap().render();
        assert_eq!(
            rendered,
            "{\n  \"name\": \"app\",\n  \"version\": \"0.1.0\"\n}\n"
        );
    }
}
