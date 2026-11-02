// === yaml-list

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::merge::{LayerId, MergeError};

/* Rendered rather than parsed, exactly as toml-members is. A YAML parser would
 * be a new dependency for a file culm only ever writes and never reads back,
 * and every value here is a string in a named sequence, which is the one corner
 * of YAML that needs no parser to emit correctly.
 *
 * Sequences, plus single integer or boolean settings. `pnpm-workspace.yaml`
 * also carries a `catalog` map, which does not belong here: a catalog is
 * version pins, which decision 5 forbids a layer from carrying at all. The
 * scalars exist for `minimumReleaseAge`, the release age guard decision 5
 * relies on; strings are left out because quoting one safely is a parser's
 * job. Two layers setting one scalar to different values is a conflict
 * naming both, the same rule `json-deep` applies. */

/// A single setting's value: an integer or a boolean, both of which render
/// unquoted in YAML exactly as they read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum YamlScalar {
    Bool(bool),
    Integer(i64),
}

impl std::fmt::Display for YamlScalar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(value) => write!(f, "{value}"),
            Self::Integer(value) => write!(f, "{value}"),
        }
    }
}

/// A YAML document that is a set of named sequences, accumulated from many layers.
/// The workspace globs in `pnpm-workspace.yaml` are the case it exists for: every layer that adds a project has to add its directory, and no layer can know what the others added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlListDoc {
    /* Sets inside a map, so union and dedupe are the same operation and both
     * levels iterate sorted. Order independence falls out rather than being
     * maintained, which is the same reason toml-members holds a set. */
    lists: BTreeMap<String, BTreeSet<String>>,
    /// Each setting with the layer that set it, so a conflict can name both.
    scalars: BTreeMap<String, (YamlScalar, LayerId)>,
    first_owner: LayerId,
}

impl YamlListDoc {
    /// Starts a document from the first layer to claim the target.
    pub(super) fn seed(
        layer: &LayerId,
        lists: &BTreeMap<String, Vec<String>>,
        scalars: &BTreeMap<String, YamlScalar>,
    ) -> Self {
        let mut doc = Self {
            lists: BTreeMap::new(),
            scalars: scalars
                .iter()
                .map(|(key, value)| (key.clone(), (*value, layer.clone())))
                .collect(),
            first_owner: layer.clone(),
        };
        doc.extend(lists);
        doc
    }

    /// Adds one more layer's sequences.
    /// Two layers naming the same glob is agreement rather than conflict, because a sequence here is a set: `apps/*` means the same thing however many layers ask for it.
    /// The same value from two layers is agreement; a different one is a
    /// conflict, and nothing is changed when it is refused.
    pub(super) fn add(
        &mut self,
        target: &Path,
        layer: &LayerId,
        lists: &BTreeMap<String, Vec<String>>,
        scalars: &BTreeMap<String, YamlScalar>,
    ) -> Result<(), MergeError> {
        for (key, value) in scalars {
            if let Some((existing, owner)) = self.scalars.get(key)
                && existing != value
            {
                /* Sorted, so which layer reads as `a` never depends on
                 * application order. */
                let (a, b) = if owner <= layer {
                    (owner.clone(), layer.clone())
                } else {
                    (layer.clone(), owner.clone())
                };
                return Err(MergeError::ScalarConflict {
                    target: target.to_path_buf(),
                    key: key.clone(),
                    a,
                    b,
                });
            }
        }
        for (key, value) in scalars {
            self.scalars
                .entry(key.clone())
                .or_insert_with(|| (*value, layer.clone()));
        }
        self.extend(lists);
        Ok(())
    }

    /* Two different kinds of "nothing" here, told apart by the incoming list
     * rather than the filtered one. A layer that names an empty sequence on
     * purpose still claims the key, so it still renders (see
     * `an_empty_sequence_still_renders_as_a_sequence`). A blank *entry*
     * inside a non-empty sequence is a different thing: a layer variable that
     * resolved to nothing for this selection (`{{workspace_glob}}` is empty
     * outside a monorepo, for instance), not a real glob. That one is
     * dropped, and if every entry offered for a name is blank this way, the
     * key is never created for it at all, which is what lets the whole
     * document stay genuinely empty and the file go unwritten rather than
     * being written with a stray empty sequence. */
    fn extend(&mut self, lists: &BTreeMap<String, Vec<String>>) {
        for (name, entries) in lists {
            if entries.is_empty() {
                self.lists.entry(name.clone()).or_default();
                continue;
            }
            let entries: Vec<String> = entries
                .iter()
                .filter(|entry| !entry.is_empty())
                .cloned()
                .collect();
            if !entries.is_empty() {
                self.lists.entry(name.clone()).or_default().extend(entries);
            }
        }
    }

    /// The layer that first claimed this target.
    pub(super) fn first_owner(&self) -> LayerId {
        self.first_owner.clone()
    }

    /// The entries of one sequence, sorted.
    pub fn entries(&self, name: &str) -> impl Iterator<Item = &str> {
        self.lists
            .get(name)
            .into_iter()
            .flatten()
            .map(String::as_str)
    }

    /// Every sequence name present, sorted.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.lists.keys().map(String::as_str)
    }

    /// Renders the document.
    /// Every entry is quoted whether it needs to be or not: a bare `*` opens an alias in YAML, and deciding per value which characters are safe is a parser's job that this deliberately does not do.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (key, (value, _)) in &self.scalars {
            out.push_str(&format!("{key}: {value}\n"));
        }
        for (name, entries) in &self.lists {
            out.push_str(name);
            if entries.is_empty() {
                out.push_str(": []\n");
                continue;
            }
            out.push_str(":\n");
            for entry in entries {
                out.push_str("  - \"");
                out.push_str(&escape(entry));
                out.push_str("\"\n");
            }
        }
        out
    }
}

/// Escapes the two characters a double-quoted YAML scalar cannot carry raw.
fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(id: &str) -> LayerId {
        LayerId::new(id)
    }

    fn lists(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        pairs
            .iter()
            .map(|(name, entries)| {
                (
                    (*name).to_owned(),
                    entries.iter().map(|e| (*e).to_owned()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn renders_a_sorted_sequence() {
        let doc = YamlListDoc::seed(
            &layer("shape/monorepo"),
            &lists(&[("packages", &["libs/*", "apps/*"])]),
            &BTreeMap::new(),
        );
        assert_eq!(doc.render(), "packages:\n  - \"apps/*\"\n  - \"libs/*\"\n");
    }

    #[test]
    fn unions_across_layers() {
        let mut doc = YamlListDoc::seed(
            &layer("frontend/nuxt"),
            &lists(&[("packages", &["apps/*"])]),
            &BTreeMap::new(),
        );
        doc.add(
            Path::new("x.yaml"),
            &layer("z"),
            &lists(&[("packages", &["libs/*"])]),
            &BTreeMap::new(),
        )
        .unwrap();
        let entries: Vec<&str> = doc.entries("packages").collect();
        assert_eq!(entries, vec!["apps/*", "libs/*"]);
    }

    /* Two layers naming the same glob is agreement, not a conflict: a sequence
     * here is a set, unlike a scalar where disagreement is unresolvable. */
    #[test]
    fn a_repeated_entry_appears_once() {
        let mut doc = YamlListDoc::seed(
            &layer("frontend/nuxt"),
            &lists(&[("packages", &["apps/*"])]),
            &BTreeMap::new(),
        );
        doc.add(
            Path::new("x.yaml"),
            &layer("z"),
            &lists(&[("packages", &["apps/*", "packages/*"])]),
            &BTreeMap::new(),
        )
        .unwrap();
        let entries: Vec<&str> = doc.entries("packages").collect();
        assert_eq!(entries, vec!["apps/*", "packages/*"]);
    }

    #[test]
    fn separate_sequences_do_not_merge_into_one() {
        let mut doc = YamlListDoc::seed(
            &layer("toolchain/node-pnpm"),
            &lists(&[("packages", &["apps/*"])]),
            &BTreeMap::new(),
        );
        doc.add(
            Path::new("x.yaml"),
            &layer("z"),
            &lists(&[("onlyBuiltDependencies", &["esbuild"])]),
            &BTreeMap::new(),
        )
        .unwrap();
        let names: Vec<&str> = doc.names().collect();
        assert_eq!(names, vec!["onlyBuiltDependencies", "packages"]);
        assert_eq!(doc.entries("packages").collect::<Vec<_>>(), vec!["apps/*"]);
    }

    #[test]
    fn addition_order_does_not_change_the_output() {
        let mut forward = YamlListDoc::seed(
            &layer("a"),
            &lists(&[("packages", &["b", "a"])]),
            &BTreeMap::new(),
        );
        forward
            .add(
                Path::new("x.yaml"),
                &layer("z"),
                &lists(&[("packages", &["c"]), ("built", &["x"])]),
                &BTreeMap::new(),
            )
            .unwrap();

        let mut backward =
            YamlListDoc::seed(&layer("a"), &lists(&[("built", &["x"])]), &BTreeMap::new());
        backward
            .add(
                Path::new("x.yaml"),
                &layer("z"),
                &lists(&[("packages", &["c", "a", "b"])]),
                &BTreeMap::new(),
            )
            .unwrap();

        assert_eq!(forward.render(), backward.render());
    }

    /* A bare `*` opens an alias and a bare `#` opens a comment, so an unquoted
     * glob is a document that means something else. */
    #[test]
    fn every_entry_is_quoted() {
        let doc = YamlListDoc::seed(
            &layer("a"),
            &lists(&[("packages", &["*", "apps/*"])]),
            &BTreeMap::new(),
        );
        assert_eq!(doc.render(), "packages:\n  - \"*\"\n  - \"apps/*\"\n");
    }

    /* The case `pnpm-workspace.yaml` exists for: a layer names an
     * interpolated variable rather than a literal glob, and that variable can
     * resolve to "" for a selection where it does not apply. */
    #[test]
    fn a_blank_entry_from_an_unresolved_variable_contributes_nothing() {
        let doc = YamlListDoc::seed(
            &layer("a"),
            &lists(&[("packages", &[""])]),
            &BTreeMap::new(),
        );
        assert_eq!(doc.render(), "");
    }

    #[test]
    fn a_blank_entry_does_not_hide_a_real_one_from_another_layer() {
        let mut doc = YamlListDoc::seed(
            &layer("a"),
            &lists(&[("packages", &[""])]),
            &BTreeMap::new(),
        );
        doc.add(
            Path::new("x.yaml"),
            &layer("z"),
            &lists(&[("packages", &["src/*"])]),
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(doc.render(), "packages:\n  - \"src/*\"\n");
    }

    #[test]
    fn an_empty_sequence_still_renders_as_a_sequence() {
        let doc = YamlListDoc::seed(&layer("a"), &lists(&[("packages", &[])]), &BTreeMap::new());
        assert_eq!(doc.render(), "packages: []\n");
    }

    #[test]
    fn a_quote_in_an_entry_is_escaped() {
        let doc = YamlListDoc::seed(
            &layer("a"),
            &lists(&[("packages", &["we\"ird"])]),
            &BTreeMap::new(),
        );
        assert_eq!(doc.render(), "packages:\n  - \"we\\\"ird\"\n");
    }

    fn scalars(pairs: &[(&str, YamlScalar)]) -> BTreeMap<String, YamlScalar> {
        pairs.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect()
    }

    #[test]
    fn a_scalar_renders_unquoted_before_the_sequences() {
        let doc = YamlListDoc::seed(
            &layer("toolchain/node-pnpm"),
            &lists(&[("packages", &["src/*"])]),
            &scalars(&[("minimumReleaseAge", YamlScalar::Integer(4320))]),
        );
        assert_eq!(
            doc.render(),
            "minimumReleaseAge: 4320\npackages:\n  - \"src/*\"\n"
        );
    }

    #[test]
    fn a_scalar_alone_still_writes_the_file() {
        let doc = YamlListDoc::seed(
            &layer("a"),
            &lists(&[("packages", &[""])]),
            &scalars(&[("saveExact", YamlScalar::Bool(true))]),
        );
        assert_eq!(doc.render(), "saveExact: true\n");
    }

    #[test]
    fn the_same_scalar_from_two_layers_is_agreement() {
        let setting = scalars(&[("minimumReleaseAge", YamlScalar::Integer(4320))]);
        let mut doc = YamlListDoc::seed(&layer("a"), &BTreeMap::new(), &setting);
        doc.add(
            Path::new("pnpm-workspace.yaml"),
            &layer("b"),
            &BTreeMap::new(),
            &setting,
        )
        .unwrap();
        assert_eq!(doc.render(), "minimumReleaseAge: 4320\n");
    }

    #[test]
    fn a_different_scalar_is_a_conflict_naming_both_layers_in_either_order() {
        let low = scalars(&[("minimumReleaseAge", YamlScalar::Integer(1440))]);
        let high = scalars(&[("minimumReleaseAge", YamlScalar::Integer(4320))]);
        let target = Path::new("pnpm-workspace.yaml");

        let mut forward = YamlListDoc::seed(&layer("a"), &BTreeMap::new(), &low);
        let first = forward
            .add(target, &layer("b"), &BTreeMap::new(), &high)
            .unwrap_err();

        let mut backward = YamlListDoc::seed(&layer("b"), &BTreeMap::new(), &high);
        let second = backward
            .add(target, &layer("a"), &BTreeMap::new(), &low)
            .unwrap_err();

        assert_eq!(first, second);
        assert!(matches!(first, MergeError::ScalarConflict { .. }));
        assert_eq!(
            forward.render(),
            "minimumReleaseAge: 1440\n",
            "a refused add changes nothing"
        );
    }
}
