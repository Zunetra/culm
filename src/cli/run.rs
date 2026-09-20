// === culm run

/* r0.1 covered `culm run <task>`: read the `culm.toml` in the working
 * directory, resolve the task and its dependency closure, run them in
 * order, no caching. r0.3 (docs/02-runner.md section 6) adds `--affected`,
 * git2-based against a base ref instead of a named task, and `--parallel`,
 * which runs the resolved order through `runner::parallel` instead of
 * `runner::exec` so independent tasks in the same graph level actually run
 * at once. */

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::Args;

use crate::runner::{RunnerConfig, Task, TaskGraph, affected, exec, parallel, tasks_of};

#[derive(Debug, Args)]
pub struct RunArgs {
    /// The task to run, as named under `[tasks.<name>]` in `culm.toml`.
    /// Required unless `--affected` is given, which picks its own set of
    /// tasks from what changed.
    pub task: Option<String>,

    /// Only run tasks affected by changes since this git ref, plus whatever
    /// downstream depends on them. Takes the place of naming a task.
    #[arg(long, value_name = "REF", conflicts_with = "task")]
    pub affected: Option<String>,

    /// Run independent tasks in the resolved order concurrently instead of
    /// one after another, grouped by dependency level.
    #[arg(long)]
    pub parallel: bool,

    /// Read a `culm.toml` at this path instead of the one in the working directory.
    /// For the tests, the same way `--layers-dir` lets `culm new` be pointed elsewhere.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

/// Runs `args.task` (or the `--affected` set) and its dependencies, in the working directory `cwd`.
pub fn run_task(args: &RunArgs, cwd: &Path) -> Result<()> {
    run_task_to(args, cwd, &mut io::stdout())
}

/// The same run, reporting into a writer the caller owns, so a test can assert what ran.
pub fn run_task_to(args: &RunArgs, cwd: &Path, out: &mut impl Write) -> Result<()> {
    let config_path = args.config.clone().unwrap_or_else(|| cwd.join("culm.toml"));

    let config = RunnerConfig::load(&config_path)?;
    let project = project_name(cwd);
    let tasks: Vec<Task> = tasks_of(&config, &project);

    let graph = TaskGraph::build(tasks.clone(), &config_path)?;

    let order = match (&args.task, &args.affected) {
        (Some(task), None) => graph.execution_order(task, &config_path)?,
        (None, Some(base_ref)) => {
            let changed = affected::changed_files(cwd, base_ref)?;
            let seeds = affected::seed_tasks(&tasks, &changed);
            if seeds.is_empty() {
                writeln!(out, "Nothing affected since {base_ref}.").ok();
                return Ok(());
            }
            graph.affected_order(&seeds, &config_path)?
        }
        (None, None) => bail!("pass a task name, or --affected <REF>"),
        (Some(_), Some(_)) => unreachable!("clap already refuses task and --affected together"),
    };

    if args.parallel {
        parallel::run(&order, cwd, out)?;
    } else {
        exec::run(&order, cwd, out)?;
    }
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
            task: Some(task.to_owned()),
            affected: None,
            parallel: false,
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

    #[test]
    fn neither_a_task_nor_affected_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_config(
            dir.path(),
            r#"
            [tasks.build]
            command = "echo ok"
            "#,
        );
        let none = RunArgs {
            task: None,
            affected: None,
            parallel: false,
            config: Some(config),
        };
        let error = run_task_to(&none, dir.path(), &mut Vec::new()).unwrap_err();
        assert!(error.to_string().contains("--affected"));
    }

    #[test]
    fn parallel_runs_every_task_in_the_closure() {
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

        let mut parallel_args = args("test", &config);
        parallel_args.parallel = true;
        run_task_to(&parallel_args, dir.path(), &mut Vec::new()).unwrap();

        let written = fs::read_to_string(&marker).unwrap();
        assert_eq!(written.lines().collect::<Vec<_>>(), vec!["build", "test"]);
    }
}
