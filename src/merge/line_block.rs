// === line-block

use std::collections::{BTreeMap, BTreeSet};

use super::LayerId;

/// A line oriented file built from one delimited block per layer.
/// Targets `.gitignore` and `.env.example`, where lines are independent and a layer only ever adds its own.
#[derive(Debug, Clone, PartialEq)]
pub struct LineBlockDoc {
    /* Keyed by layer id so block order is sorted by construction rather than by
     * the order layers happened to be applied. */
    blocks: BTreeMap<LayerId, BTreeSet<String>>,
    first_owner: LayerId,
}

impl LineBlockDoc {
    /// Starts a document from the first layer to claim the target.
    pub(super) fn seed(layer: &LayerId, lines: &[String]) -> Self {
        let mut doc = Self {
            blocks: BTreeMap::new(),
            first_owner: layer.clone(),
        };
        doc.add(layer, lines);
        doc
    }

    /// Adds one layer's lines.
    /// A layer contributing twice extends its own block rather than opening a second one.
    pub(super) fn add(&mut self, layer: &LayerId, lines: &[String]) {
        let block = self.blocks.entry(layer.clone()).or_default();
        block.extend(lines.iter().cloned());
    }

    /// The layer that first claimed this target.
    pub(super) fn first_owner(&self) -> LayerId {
        self.first_owner.clone()
    }

    /// Renders the file, blocks sorted by layer id and lines sorted within each block.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (index, (layer, lines)) in self.blocks.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            out.push_str("# layer:");
            out.push_str(layer.as_str());
            out.push('\n');
            for line in lines {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use crate::merge::{Contribution, FileSet, Layer};
    use std::path::PathBuf;

    fn target() -> PathBuf {
        PathBuf::from(".gitignore")
    }

    fn layer(id: &str, lines: &[&str]) -> Layer {
        Layer::new(id).with_merge(
            ".gitignore",
            Contribution::LineBlock(lines.iter().map(|l| (*l).to_owned()).collect()),
        )
    }

    fn rendered(layers: &[Layer]) -> String {
        FileSet::build(layers.iter())
            .unwrap()
            .get(&target())
            .unwrap()
            .render()
    }

    #[test]
    fn blocks_are_sorted_by_layer_id() {
        let rust = layer("rust-cargo", &["target/"]);
        let node = layer("node-pnpm", &["node_modules/", ".pnpm-store/"]);
        assert_eq!(
            rendered(&[rust, node]),
            "# layer:node-pnpm\n.pnpm-store/\nnode_modules/\n\n# layer:rust-cargo\ntarget/\n"
        );
    }

    #[test]
    fn application_order_does_not_change_the_bytes() {
        let a = layer("a", &["dist/"]);
        let b = layer("b", &["coverage/"]);
        assert_eq!(
            rendered(&[a.clone(), b.clone()]),
            rendered(&[b.clone(), a.clone()])
        );
    }

    #[test]
    fn a_layer_contributing_twice_keeps_one_block() {
        let mut set = FileSet::new();
        let first = layer("node-pnpm", &["node_modules/"]);
        let second = layer("node-pnpm", &["dist/", "node_modules/"]);
        set.apply(&first).unwrap();
        set.apply(&second).unwrap();
        assert_eq!(
            set.get(&target()).unwrap().render(),
            "# layer:node-pnpm\ndist/\nnode_modules/\n"
        );
    }
}
