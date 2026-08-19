// === toml-members

use std::collections::BTreeSet;

use crate::merge::LayerId;

/// A Cargo workspace manifest accumulated from many layers.
/// Only `[workspace].members` is merged, because that is the one key several layers legitimately contribute to: every crate a layer adds has to appear in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TomlMembersDoc {
    /* A set, so union and dedupe are the same operation and iteration is
     * already sorted. Order independence falls out rather than being
     * maintained. */
    members: BTreeSet<String>,
    first_owner: LayerId,
}

impl TomlMembersDoc {
    /// Starts a manifest from the first layer to claim the target.
    pub(super) fn seed(layer: &LayerId, members: &[String]) -> Self {
        Self {
            members: members.iter().cloned().collect(),
            first_owner: layer.clone(),
        }
    }

    /// Adds one more layer's members.
    /// Two layers naming the same crate is agreement rather than conflict, unlike a scalar, because a member list is a set.
    pub(super) fn add(&mut self, members: &[String]) {
        self.members.extend(members.iter().cloned());
    }

    /// The layer that first claimed this target.
    pub(super) fn first_owner(&self) -> LayerId {
        self.first_owner.clone()
    }

    pub fn members(&self) -> impl Iterator<Item = &str> {
        self.members.iter().map(String::as_str)
    }

    /// Renders the workspace manifest.
    /// `resolver` is fixed rather than contributed, because it follows from the edition and a layer choosing it would be a layer deciding something for the whole workspace.
    pub fn render(&self) -> String {
        let mut out = String::from("[workspace]\nresolver = \"3\"\nmembers = [\n");
        for member in &self.members {
            out.push_str("    \"");
            out.push_str(member);
            out.push_str("\",\n");
        }
        out.push_str("]\n");
        out
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(id: &str) -> LayerId {
        LayerId::new(id)
    }

    fn members(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| (*v).to_owned()).collect()
    }

    #[test]
    fn renders_a_sorted_member_list() {
        let doc = TomlMembersDoc::seed(&layer("backend/actix"), &members(&["common", "app"]));
        let rendered = doc.render();
        let app = rendered.find("\"app\"").expect("app is listed");
        let common = rendered.find("\"common\"").expect("common is listed");
        assert!(app < common, "members must be sorted");
    }

    #[test]
    fn unions_across_layers() {
        let mut doc = TomlMembersDoc::seed(&layer("backend/axum"), &members(&["app"]));
        doc.add(&members(&["entity", "migration"]));
        let listed: Vec<&str> = doc.members().collect();
        assert_eq!(listed, vec!["app", "entity", "migration"]);
    }

    /* Two layers naming the same crate is agreement, not a conflict: a member
     * list is a set, unlike a scalar where disagreement is unresolvable. */
    #[test]
    fn a_repeated_member_appears_once() {
        let mut doc = TomlMembersDoc::seed(&layer("backend/actix"), &members(&["app", "common"]));
        doc.add(&members(&["common", "errors"]));
        let listed: Vec<&str> = doc.members().collect();
        assert_eq!(listed, vec!["app", "common", "errors"]);
    }

    #[test]
    fn addition_order_does_not_change_the_output() {
        let mut forward = TomlMembersDoc::seed(&layer("a"), &members(&["b", "a"]));
        forward.add(&members(&["c"]));

        let mut backward = TomlMembersDoc::seed(&layer("a"), &members(&["c"]));
        backward.add(&members(&["a", "b"]));

        assert_eq!(forward.render(), backward.render());
    }
}
