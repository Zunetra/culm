// === yaml-list

use std::collections::{BTreeMap, BTreeSet};

use crate::merge::LayerId;

/* Rendered rather than parsed, exactly as toml-members is. A YAML parser would
 * be a new dependency for a file culm only ever writes and never reads back,
 * and every value here is a string in a named sequence, which is the one corner
 * of YAML that needs no parser to emit correctly.
 *
 * Scoped to sequences on purpose. `pnpm-workspace.yaml` also carries scalar
 * settings and a `catalog` map, and neither belongs here: a catalog is version
 * pins, which decision 5 forbids a layer from carrying at all, and the scalars
 * have no second consumer yet, so a strategy shaped around them today would be
 * designed against one example. */

/// A YAML document that is a set of named sequences, accumulated from many layers.
/// The workspace globs in `pnpm-workspace.yaml` are the case it exists for: every layer that adds a project has to add its directory, and no layer can know what the others added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlListDoc {
    /* Sets inside a map, so union and dedupe are the same operation and both
     * levels iterate sorted. Order independence falls out rather than being
     * maintained, which is the same reason toml-members holds a set. */
    lists: BTreeMap<String, BTreeSet<String>>,
    first_owner: LayerId,
}

impl YamlListDoc {
    /// Starts a document from the first layer to claim the target.
    pub(super) fn seed(layer: &LayerId, lists: &BTreeMap<String, Vec<String>>) -> Self {
        let mut doc = Self {
            lists: BTreeMap::new(),
            first_owner: layer.clone(),
        };
        doc.extend(lists);
        doc
    }

    /// Adds one more layer's sequences.
    /// Two layers naming the same glob is agreement rather than conflict, because a sequence here is a set: `apps/*` means the same thing however many layers ask for it.
    pub(super) fn add(&mut self, lists: &BTreeMap<String, Vec<String>>) {
        self.extend(lists);
    }

    fn extend(&mut self, lists: &BTreeMap<String, Vec<String>>) {
        for (name, entries) in lists {
            self.lists
                .entry(name.clone())
                .or_default()
                .extend(entries.iter().cloned());
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
        );
        assert_eq!(doc.render(), "packages:\n  - \"apps/*\"\n  - \"libs/*\"\n");
    }

    #[test]
    fn unions_across_layers() {
        let mut doc = YamlListDoc::seed(
            &layer("frontend/nuxt"),
            &lists(&[("packages", &["apps/*"])]),
        );
        doc.add(&lists(&[("packages", &["libs/*"])]));
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
        );
        doc.add(&lists(&[("packages", &["apps/*", "packages/*"])]));
        let entries: Vec<&str> = doc.entries("packages").collect();
        assert_eq!(entries, vec!["apps/*", "packages/*"]);
    }

    #[test]
    fn separate_sequences_do_not_merge_into_one() {
        let mut doc = YamlListDoc::seed(
            &layer("toolchain/node-pnpm"),
            &lists(&[("packages", &["apps/*"])]),
        );
        doc.add(&lists(&[("onlyBuiltDependencies", &["esbuild"])]));
        let names: Vec<&str> = doc.names().collect();
        assert_eq!(names, vec!["onlyBuiltDependencies", "packages"]);
        assert_eq!(doc.entries("packages").collect::<Vec<_>>(), vec!["apps/*"]);
    }

    #[test]
    fn addition_order_does_not_change_the_output() {
        let mut forward = YamlListDoc::seed(&layer("a"), &lists(&[("packages", &["b", "a"])]));
        forward.add(&lists(&[("packages", &["c"]), ("built", &["x"])]));

        let mut backward = YamlListDoc::seed(&layer("a"), &lists(&[("built", &["x"])]));
        backward.add(&lists(&[("packages", &["c", "a", "b"])]));

        assert_eq!(forward.render(), backward.render());
    }

    /* A bare `*` opens an alias and a bare `#` opens a comment, so an unquoted
     * glob is a document that means something else. */
    #[test]
    fn every_entry_is_quoted() {
        let doc = YamlListDoc::seed(&layer("a"), &lists(&[("packages", &["*", "apps/*"])]));
        assert_eq!(doc.render(), "packages:\n  - \"*\"\n  - \"apps/*\"\n");
    }

    #[test]
    fn an_empty_sequence_still_renders_as_a_sequence() {
        let doc = YamlListDoc::seed(&layer("a"), &lists(&[("packages", &[])]));
        assert_eq!(doc.render(), "packages: []\n");
    }

    #[test]
    fn a_quote_in_an_entry_is_escaped() {
        let doc = YamlListDoc::seed(&layer("a"), &lists(&[("packages", &["we\"ird"])]));
        assert_eq!(doc.render(), "packages:\n  - \"we\\\"ird\"\n");
    }
}
