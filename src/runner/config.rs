// === culm.toml

/* One `culm.toml` per project directory, decision 22. Parsing only ever adds
 * a field, per docs/02-runner.md section 8's own instruction, so a config
 * written against a later milestone still loads here: `cache` is stored and
 * ignored, since r0.1 has no cache to consult. */

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::error::RunnerError;

/// The raw shape of `culm.toml`, before it becomes a `Task` per project.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RunnerConfig {
    pub tasks: BTreeMap<String, TaskSpec>,
}

/// One `[tasks.<name>]` table.
///
/// `cache` and `inputs`/`outputs` are parsed and kept even though r0.1 does
/// not act on them, so a `culm.toml` written today is not a migration once
/// r0.2 adds hashing. `depends_on` is what r0.1 actually uses.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TaskSpec {
    pub command: String,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub cache: bool,
    #[serde(default)]
    pub declared_env: Vec<String>,
}

impl RunnerConfig {
    /// Reads and parses `culm.toml` at the given path.
    pub fn load(path: &Path) -> Result<Self, RunnerError> {
        let raw = fs::read_to_string(path).map_err(|e| RunnerError::Unreadable {
            path: path.to_owned(),
            message: e.to_string(),
        })?;
        toml::from_str(&raw).map_err(|e| RunnerError::Malformed {
            path: path.to_owned(),
            message: e.to_string(),
        })
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minimal_config_parses() {
        let raw = r#"
            [tasks.build]
            command = "cargo build"
        "#;
        let config: RunnerConfig = toml::from_str(raw).unwrap();
        let build = &config.tasks["build"];
        assert_eq!(build.command, "cargo build");
        assert!(build.inputs.is_empty());
        assert!(build.outputs.is_empty());
        assert!(build.depends_on.is_empty());
        assert!(!build.cache);
    }

    #[test]
    fn the_worked_example_from_the_design_doc_parses() {
        let raw = r#"
            [tasks.build]
            command = "cargo build --release"
            inputs = ["src/**/*.rs", "Cargo.toml", "Cargo.lock"]
            outputs = ["target/release/backend"]
            cache = true

            [tasks.test]
            command = "cargo nextest run"
            inputs = ["src/**/*.rs", "tests/**/*.rs", "Cargo.toml"]
            depends_on = ["build"]
            cache = true
        "#;
        let config: RunnerConfig = toml::from_str(raw).unwrap();
        assert_eq!(config.tasks.len(), 2);
        assert_eq!(config.tasks["test"].depends_on, vec!["build".to_owned()]);
        assert!(config.tasks["build"].cache);
    }

    #[test]
    fn a_missing_file_is_reported_by_path() {
        let error = RunnerConfig::load(Path::new("/does/not/exist/culm.toml")).unwrap_err();
        assert!(matches!(error, RunnerError::Unreadable { .. }));
    }

    #[test]
    fn malformed_toml_is_reported_by_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("culm.toml");
        fs::write(&path, "not = [valid").unwrap();
        let error = RunnerConfig::load(&path).unwrap_err();
        assert!(matches!(error, RunnerError::Malformed { .. }));
    }
}
