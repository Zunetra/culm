// === Task

use std::path::{Path, PathBuf};

use super::config::RunnerConfig;

/// One task, resolved from a `culm.toml` table.
///
/// Matches the sketch in docs/DESIGN.md section 3.3. `inputs`, `outputs`,
/// `cacheable` and `declared_env` feed the cache key in `runner::cache`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// Unique across the run: the bare name in a single project, or
    /// `<project>:<name>` in a workspace.
    pub name: String,
    pub project: String,
    /// The project directory: where the command runs and what `inputs` and
    /// `outputs` are relative to.
    pub dir: PathBuf,
    pub command: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub depends_on: Vec<String>,
    pub cacheable: bool,
    pub declared_env: Vec<String>,
    pub persistent: bool,
}

/// Every task in a project's `culm.toml`, in no particular order.
/// Ordering is the graph's job, not the config's.
pub fn tasks_of(config: &RunnerConfig, project: &str, dir: &Path) -> Vec<Task> {
    config
        .tasks
        .iter()
        .map(|(name, spec)| Task {
            name: name.clone(),
            project: project.to_owned(),
            dir: dir.to_owned(),
            command: spec.command.clone(),
            inputs: spec.inputs.clone(),
            outputs: spec.outputs.clone(),
            depends_on: spec.depends_on.clone(),
            cacheable: spec.cache,
            declared_env: spec.declared_env.clone(),
            persistent: spec.persistent,
        })
        .collect()
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::config::TaskSpec;
    use std::collections::BTreeMap;

    fn spec(command: &str, depends_on: &[&str]) -> TaskSpec {
        TaskSpec {
            command: command.to_owned(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            depends_on: depends_on.iter().map(|s| (*s).to_owned()).collect(),
            cache: false,
            declared_env: Vec::new(),
            persistent: false,
        }
    }

    #[test]
    fn each_config_table_becomes_one_task_named_for_the_project() {
        let mut tasks = BTreeMap::new();
        tasks.insert("build".to_owned(), spec("cargo build", &[]));
        tasks.insert("test".to_owned(), spec("cargo test", &["build"]));
        let config = RunnerConfig {
            tasks,
            ..RunnerConfig::default()
        };

        let mut resolved = tasks_of(&config, "backend", Path::new("."));
        resolved.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(resolved[0].name, "build");
        assert_eq!(resolved[0].project, "backend");
        assert_eq!(resolved[1].depends_on, vec!["build".to_owned()]);
    }
}
