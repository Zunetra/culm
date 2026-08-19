// === yaml-steps

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use super::LayerId;

/* Rendered rather than parsed, for the same reason as yaml-list and
 * toml-members: culm writes this file and never reads it back.
 *
 * The skeleton is fixed rather than contributed. A workflow needs a name, a
 * trigger set, and a runner image before any step can exist, and none of the
 * three is a property of a step. Letting a layer own them would mean the first
 * layer to be applied decided the trigger for every other layer's steps, which
 * is exactly the kind of silent precedence the engine exists to prevent. It is
 * the same call toml-members makes by fixing `resolver` rather than accepting
 * it from whichever layer claimed Cargo.toml first. */

/// One step in a CI job.
/// `uses` and `run` are both optional and both allowed, because an action step and a command step are the two shapes every CI file is made of and a strategy that supported only one would push layers into shell workarounds.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub name: String,
    #[serde(default)]
    pub uses: Option<String>,
    /* A ref such as `actions/checkout@v4` is a version, and decision 5 does not
     * reach it: that rule exists because package managers write versions into
     * lockfiles, and nothing resolves an action ref on anybody's behalf. An
     * unpinned action is a supply chain hole rather than a stale pin. */
    #[serde(default)]
    pub with: BTreeMap<String, String>,
    #[serde(default)]
    pub run: Vec<String>,
}

/// A CI workflow assembled from steps several layers contribute.
/// Targets `.github/workflows/*.yml`, where a Rust layer wants `cargo test` and a JavaScript layer wants `pnpm test` in the same file and neither knows the other exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlStepsDoc {
    /* Jobs sorted by name, steps sorted by declared order then layer id. Both
     * levels are BTree so sequence is decided by declaration rather than by
     * which layer was applied first, and a tie still renders identically. */
    jobs: BTreeMap<String, BTreeMap<(u32, LayerId), Vec<Step>>>,
    /// Taken from the target's file stem, so a second workflow file is named after itself rather than after the first one written.
    name: String,
    first_owner: LayerId,
}

impl YamlStepsDoc {
    /// Starts a workflow from the first layer to claim the target.
    pub(super) fn seed(
        target: &Path,
        layer: &LayerId,
        job: &str,
        order: u32,
        steps: &[Step],
    ) -> Self {
        let name = target
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| "ci".to_owned());
        let mut doc = Self {
            jobs: BTreeMap::new(),
            name,
            first_owner: layer.clone(),
        };
        doc.add(layer, job, order, steps);
        doc
    }

    /// Adds one layer's steps to one job at its declared order.
    /// Steps within a single contribution keep the order they were written in, because they are commands in a sequence rather than a set.
    pub(super) fn add(&mut self, layer: &LayerId, job: &str, order: u32, steps: &[Step]) {
        self.jobs
            .entry(job.to_owned())
            .or_default()
            .entry((order, layer.clone()))
            .or_default()
            .extend(steps.iter().cloned());
    }

    /// The layer that first claimed this target.
    pub(super) fn first_owner(&self) -> LayerId {
        self.first_owner.clone()
    }

    /// Every job name present, sorted.
    pub fn jobs(&self) -> impl Iterator<Item = &str> {
        self.jobs.keys().map(String::as_str)
    }

    /// Renders the workflow.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("name: ");
        out.push_str(&self.name);
        out.push_str("\n\non:\n  push:\n  pull_request:\n  workflow_dispatch:\n\njobs:\n");

        for (job, contributions) in &self.jobs {
            out.push_str("  ");
            out.push_str(job);
            out.push_str(":\n    runs-on: ubuntu-latest\n    steps:\n");
            for ((order, layer), steps) in contributions {
                out.push_str("      # layer:");
                out.push_str(layer.as_str());
                out.push_str(" order:");
                out.push_str(&order.to_string());
                out.push('\n');
                for step in steps {
                    render_step(&mut out, step);
                }
            }
        }
        out
    }
}

fn render_step(out: &mut String, step: &Step) {
    out.push_str("      - name: \"");
    out.push_str(&escape(&step.name));
    out.push_str("\"\n");

    if let Some(uses) = &step.uses {
        out.push_str("        uses: \"");
        out.push_str(&escape(uses));
        out.push_str("\"\n");
    }
    if !step.with.is_empty() {
        out.push_str("        with:\n");
        for (key, value) in &step.with {
            out.push_str("          ");
            out.push_str(key);
            out.push_str(": \"");
            out.push_str(&escape(value));
            out.push_str("\"\n");
        }
    }
    if !step.run.is_empty() {
        /* A block scalar rather than a quoted string, so a command carrying a
         * colon or a quote needs no escaping and reads as it would in a shell. */
        out.push_str("        run: |\n");
        for line in &step.run {
            out.push_str("          ");
            out.push_str(line);
            out.push('\n');
        }
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

    fn target() -> &'static Path {
        Path::new(".github/workflows/ci.yml")
    }

    fn run_step(name: &str, command: &str) -> Step {
        Step {
            name: name.to_owned(),
            uses: None,
            with: BTreeMap::new(),
            run: vec![command.to_owned()],
        }
    }

    #[test]
    fn the_workflow_is_named_after_its_file() {
        let doc = YamlStepsDoc::seed(
            Path::new(".github/workflows/release.yml"),
            &layer("a"),
            "build",
            10,
            &[run_step("Build", "cargo build")],
        );
        assert!(doc.render().starts_with("name: release\n"));
    }

    /* The whole reason `order` exists: a checkout has to happen before anything
     * that reads the repository, and neither layer can know the other exists. */
    #[test]
    fn steps_run_in_declared_order_not_insertion_order() {
        let mut doc = YamlStepsDoc::seed(
            target(),
            &layer("quality/clippy"),
            "gate",
            30,
            &[run_step("Clippy", "cargo clippy")],
        );
        doc.add(
            &layer("infra/github-actions"),
            "gate",
            10,
            &[run_step("Checkout", "true")],
        );

        let rendered = doc.render();
        assert!(rendered.find("Checkout").unwrap() < rendered.find("Clippy").unwrap());
    }

    #[test]
    fn a_tie_on_order_breaks_on_layer_id() {
        let mut forward =
            YamlStepsDoc::seed(target(), &layer("b"), "gate", 20, &[run_step("B", "b")]);
        forward.add(&layer("a"), "gate", 20, &[run_step("A", "a")]);

        let mut backward =
            YamlStepsDoc::seed(target(), &layer("a"), "gate", 20, &[run_step("A", "a")]);
        backward.add(&layer("b"), "gate", 20, &[run_step("B", "b")]);

        assert_eq!(forward.render(), backward.render());
    }

    #[test]
    fn separate_jobs_do_not_merge_into_one() {
        let mut doc = YamlStepsDoc::seed(target(), &layer("a"), "rust", 10, &[run_step("T", "t")]);
        doc.add(&layer("b"), "node", 10, &[run_step("P", "p")]);
        assert_eq!(doc.jobs().collect::<Vec<_>>(), vec!["node", "rust"]);
    }

    #[test]
    fn addition_order_does_not_change_the_output() {
        let mut forward = YamlStepsDoc::seed(target(), &layer("b"), "n", 20, &[run_step("B", "b")]);
        forward.add(&layer("a"), "r", 10, &[run_step("A", "a")]);

        let mut backward =
            YamlStepsDoc::seed(target(), &layer("a"), "r", 10, &[run_step("A", "a")]);
        backward.add(&layer("b"), "n", 20, &[run_step("B", "b")]);

        assert_eq!(forward.render(), backward.render());
    }

    /* An action step and a command step are the two shapes a CI file is made
     * of, and a strategy supporting only one pushes layers into shell hacks. */
    #[test]
    fn an_action_step_renders_uses_and_with() {
        let step = Step {
            name: "Checkout".to_owned(),
            uses: Some("actions/checkout@v4".to_owned()),
            with: BTreeMap::from([("fetch-depth".to_owned(), "0".to_owned())]),
            run: Vec::new(),
        };
        let doc = YamlStepsDoc::seed(target(), &layer("a"), "gate", 10, &[step]);
        let rendered = doc.render();
        assert!(rendered.contains("uses: \"actions/checkout@v4\""));
        assert!(rendered.contains("fetch-depth: \"0\""));
        assert!(!rendered.contains("run:"));
    }

    /* A block scalar, so a command carrying a colon needs no escaping and the
     * rendered file reads the way the command would in a shell. */
    #[test]
    fn a_command_carrying_a_colon_needs_no_escaping() {
        let doc = YamlStepsDoc::seed(
            target(),
            &layer("a"),
            "gate",
            10,
            &[run_step("Run", "echo a: b")],
        );
        let rendered = doc.render();
        assert!(rendered.contains("run: |\n          echo a: b\n"));
    }

    #[test]
    fn multiple_run_lines_stay_in_written_order() {
        let step = Step {
            name: "Build".to_owned(),
            uses: None,
            with: BTreeMap::new(),
            run: vec!["cd app".to_owned(), "pnpm build".to_owned()],
        };
        let doc = YamlStepsDoc::seed(target(), &layer("a"), "gate", 10, &[step]);
        let rendered = doc.render();
        assert!(rendered.find("cd app").unwrap() < rendered.find("pnpm build").unwrap());
    }
}
