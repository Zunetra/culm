// === hook-script

use std::collections::BTreeMap;

use super::LayerId;

/// The mode a rendered hook is written with.
/// A git hook that is not executable is skipped by git without a word, so the bit is part of what this strategy produces rather than something a layer remembers to ask for.
pub const HOOK_MODE: u32 = 0o755;

/// A shell hook assembled from the bodies several layers contribute.
/// Targets `.husky/*`, where a pre-commit hook legitimately runs a JavaScript linter and a Rust formatter that know nothing about each other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookScriptDoc {
    /* Keyed by order then layer id, so the sequence is decided by the declared
     * order and ties break on identity rather than on which layer happened to
     * be applied first. Sequence is observable here in a way it is not in
     * .gitignore: a formatter that runs after a linter sees different files. */
    bodies: BTreeMap<(u32, LayerId), Vec<String>>,
    first_owner: LayerId,
}

impl HookScriptDoc {
    /// Starts a hook from the first layer to claim the target.
    pub(super) fn seed(layer: &LayerId, order: u32, body: &[String]) -> Self {
        let mut doc = Self {
            bodies: BTreeMap::new(),
            first_owner: layer.clone(),
        };
        doc.add(layer, order, body);
        doc
    }

    /// Adds one layer's body at its declared order.
    /// Body lines keep the order they were written in, unlike every other strategy's payload: these are commands, and sorting them would change what the hook does.
    pub(super) fn add(&mut self, layer: &LayerId, order: u32, body: &[String]) {
        self.bodies
            .entry((order, layer.clone()))
            .or_default()
            .extend(body.iter().cloned());
    }

    /// The layer that first claimed this target.
    pub(super) fn first_owner(&self) -> LayerId {
        self.first_owner.clone()
    }

    /// The mode this file must be written with.
    pub fn mode(&self) -> u32 {
        HOOK_MODE
    }

    /// Renders the hook: a shebang, then each layer's body in order.
    /// `set -e` is not added, because a layer's body may legitimately test an exit status, and a strategy that silently changed the shell's failure semantics would be deciding something for the layers rather than assembling them.
    pub fn render(&self) -> String {
        let mut out = String::from("#!/bin/sh\n");
        for ((order, layer), body) in &self.bodies {
            out.push_str("\n# layer:");
            out.push_str(layer.as_str());
            out.push_str(" order:");
            out.push_str(&order.to_string());
            out.push('\n');
            for line in body {
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
    use super::*;

    fn layer(id: &str) -> LayerId {
        LayerId::new(id)
    }

    fn body(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|l| (*l).to_owned()).collect()
    }

    #[test]
    fn renders_a_shebang_first() {
        let doc = HookScriptDoc::seed(&layer("quality/husky"), 10, &body(["echo hi"].as_ref()));
        assert!(doc.render().starts_with("#!/bin/sh\n"));
    }

    /* The whole reason `order` exists: a formatter that rewrites files has to
     * run before a linter that reads them, and neither layer can know the
     * other exists. */
    #[test]
    fn bodies_run_in_declared_order_not_insertion_order() {
        let mut doc = HookScriptDoc::seed(&layer("quality/eslint"), 30, &body(&["lint"]));
        doc.add(&layer("quality/prettier"), 10, &body(&["format"]));

        let rendered = doc.render();
        let format = rendered.find("format").expect("format is present");
        let lint = rendered.find("lint").expect("lint is present");
        assert!(format < lint, "order 10 must precede order 30");
    }

    /* Two layers at the same order would otherwise render in whichever order
     * they were applied, which is the one thing the engine must never do. */
    #[test]
    fn a_tie_on_order_breaks_on_layer_id() {
        let mut forward = HookScriptDoc::seed(&layer("quality/rustfmt"), 20, &body(&["cargo fmt"]));
        forward.add(&layer("quality/prettier"), 20, &body(&["prettier"]));

        let mut backward =
            HookScriptDoc::seed(&layer("quality/prettier"), 20, &body(&["prettier"]));
        backward.add(&layer("quality/rustfmt"), 20, &body(&["cargo fmt"]));

        assert_eq!(forward.render(), backward.render());
        let rendered = forward.render();
        assert!(rendered.find("prettier").unwrap() < rendered.find("cargo fmt").unwrap());
    }

    /* Body lines are commands, so unlike every other strategy's payload they
     * are not sorted: `cd app` before `pnpm test` is not the same script as
     * the reverse. */
    #[test]
    fn body_lines_keep_the_order_they_were_written_in() {
        let doc = HookScriptDoc::seed(&layer("a"), 10, &body(&["cd app", "pnpm test", "cd .."]));
        let rendered = doc.render();
        let cd = rendered.find("cd app").unwrap();
        let test = rendered.find("pnpm test").unwrap();
        let back = rendered.find("cd ..").unwrap();
        assert!(cd < test && test < back);
    }

    #[test]
    fn a_layer_contributing_twice_extends_its_own_body() {
        let mut doc = HookScriptDoc::seed(&layer("a"), 10, &body(&["first"]));
        doc.add(&layer("a"), 10, &body(&["second"]));
        let rendered = doc.render();
        assert_eq!(rendered.matches("# layer:a").count(), 1);
        assert!(rendered.find("first").unwrap() < rendered.find("second").unwrap());
    }

    #[test]
    fn addition_order_does_not_change_the_output() {
        let mut forward = HookScriptDoc::seed(&layer("b"), 20, &body(&["two"]));
        forward.add(&layer("a"), 10, &body(&["one"]));

        let mut backward = HookScriptDoc::seed(&layer("a"), 10, &body(&["one"]));
        backward.add(&layer("b"), 20, &body(&["two"]));

        assert_eq!(forward.render(), backward.render());
    }

    /* A hook without the executable bit is skipped by git in silence, which is
     * the failure this strategy exists to make impossible. */
    #[test]
    fn a_hook_is_executable() {
        let doc = HookScriptDoc::seed(&layer("a"), 10, &body(&["x"]));
        assert_eq!(doc.mode(), 0o755);
    }
}
