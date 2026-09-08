// === culm run

/* r0.1: `culm run <task>` reads the `culm.toml` in the working directory,
 * resolves the task and its dependency closure, and runs them in order. No
 * caching and no `--affected` yet, per docs/02-runner.md section 6. */

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Args;

use crate::runner::{RunnerConfig, TaskGraph, exec, tasks_of};

#[derive(Debug, Args)]
pub struct RunArgs {
    /// The task to run, as named under `[tasks.<name>]` in `culm.toml`.
    pub task: String,

    /// Read a `culm.toml` at this path instead of the one in the working directory.
    /// For the tests, the same way `--layers-dir` lets `culm new` be pointed elsewhere.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

/// Runs `args.task` and its dependencies, in the working directory `cwd`.
pub fn run_task(args: &RunArgs, cwd: &Path) -> Result<()> {
    run_task_to(args, cwd, &mut io::stdout())
}

/// The same run, reporting into a writer the caller owns, so a test can assert what ran.
pub fn run_task_to(args: &RunArgs, cwd: &Path, out: &mut impl Write) -> Result<()> {
    let config_path = args.config.clone().unwrap_or_else(|| cwd.join("culm.toml"));

    let config = RunnerConfig::load(&config_path)?;
    let project = project_name(cwd);
    let tasks = tasks_of(&config, &project);

    let graph = TaskGraph::build(tasks, &config_path)?;
    let order = graph.execution_order(&args.task, &config_path)?;

    exec::run(&order, cwd, out)?;
    Ok(())
}

/// The directory name culm is run in, used only to label tasks in error output.
fn project_name(cwd: &Path) -> String {
    cwd.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_owned())
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn args(task: &str, config: &Path) -> RunArgs {
        RunArgs {
            task: task.to_owned(),
            config: Some(config.to_owned()),
        }
    }

    fn write_config(dir: &Path, contents: &str) -> PathBuf {
        let path = dir.join("culm.toml");
        fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn a_task_with_a_dependency_runs_both_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("order.txt");
        let config = write_config(
            dir.path(),
            &format!(
                r#"
                [tasks.build]
                command = "echo build >> {marker}"

                [tasks.test]
                command = "echo test >> {marker}"
                depends_on = ["build"]
                "#,
                marker = marker.display()
            ),
        );

        run_task_to(&args("test", &config), dir.path(), &mut Vec::new()).unwrap();

        let written = fs::read_to_string(&marker).unwrap();
        assert_eq!(written.lines().collect::<Vec<_>>(), vec!["build", "test"]);
    }

    #[test]
    fn a_failing_task_stops_the_run() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_config(
            dir.path(),
            r#"
            [tasks.build]
            command = "exit 1"
            "#,
        );

        let error = run_task_to(&args("build", &config), dir.path(), &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("build"));
    }

    #[test]
    fn an_undefined_task_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_config(
            dir.path(),
            r#"
            [tasks.build]
            command = "echo ok"
            "#,
        );

        let error =
            run_task_to(&args("missing", &config), dir.path(), &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("missing"));
    }

    #[test]
    fn a_cycle_is_refused_before_anything_runs() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("ran.txt");
        let config = write_config(
            dir.path(),
            &format!(
                r#"
                [tasks.a]
                command = "touch {marker}"
                depends_on = ["b"]

                [tasks.b]
                command = "touch {marker}"
                depends_on = ["a"]
                "#,
                marker = marker.display()
            ),
        );

        let error = run_task_to(&args("a", &config), dir.path(), &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("a"));
        assert!(error.to_string().contains("b"));
        assert!(!marker.exists());
    }
}
