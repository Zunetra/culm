// === culm run

/* `culm run <task>`: read the `culm.toml` in the working directory, resolve
 * the task and its dependency closure, and run them in order (r0.1), each
 * restored from or recorded to the local cache (r0.2). `--affected` picks
 * the tasks from a git diff instead of a name, and `--parallel` runs each
 * dependency level at once (r0.3). docs/DESIGN.md section 3.5. */

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use clap::Args;

use crate::cas::Store;
use crate::runner::remote_cache::RemoteCache;
use crate::runner::{Cache, CacheMode, Session, TaskGraph, affected, exec, parallel, workspace};
use crate::telemetry::{ErrorKind, RunStats};

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

    /// Run every task for real, reading and writing no cache.
    #[arg(long, conflicts_with = "verify")]
    pub no_cache: bool,

    /// Run every task for real, and fail if a cacheable task's output differs
    /// from what its cache entry holds. The check behind the r0.2 checkpoint.
    #[arg(long)]
    pub verify: bool,

    /// Use this directory as the cache instead of the OS cache directory.
    #[arg(long, value_name = "PATH")]
    pub cache_dir: Option<PathBuf>,

    /// Read a `culm.toml` at this path instead of the one in the working directory.
    /// For the tests, the same way `--layers-dir` lets `culm new` be pointed elsewhere.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

/// Runs `args.task` (or the `--affected` set) and its dependencies, in the working directory `cwd`.
pub fn run_task(args: &RunArgs, cwd: &Path) -> Result<()> {
    run_task_to(args, cwd, &mut io::stdout())
}

/// What a `culm run` learned about itself, for telemetry: counts and flags only.
#[derive(Debug, Default)]
pub struct RunReport {
    pub run: RunStats,
    /// The stage the run is in; the stage it failed in, if it failed.
    pub stage: ErrorKind,
}

/// The same run, reporting into a writer the caller owns, so a test can assert what ran.
pub fn run_task_to(args: &RunArgs, cwd: &Path, out: &mut impl Write) -> Result<()> {
    run_task_reporting(args, cwd, out, &mut RunReport::default())
}

/// The same run, also filling `report` for telemetry.
pub fn run_task_reporting(
    args: &RunArgs,
    cwd: &Path,
    out: &mut impl Write,
    report: &mut RunReport,
) -> Result<()> {
    report.run.parallel = args.parallel;
    report.run.affected = args.affected.is_some();
    report.run.remote_cache = RemoteCache::from_env().is_some();

    report.stage = ErrorKind::Config;
    let loaded = workspace::load(cwd, args.config.as_deref())?;
    let config_path = loaded.config_path.clone();
    report.run.projects = count(loaded.projects);

    report.stage = ErrorKind::Graph;
    let graph = TaskGraph::build(loaded.tasks.clone(), &config_path)?;

    let order = match (&args.task, &args.affected) {
        (Some(task), None) => {
            let targets = loaded.targets(task)?;
            let targets: Vec<&str> = targets.iter().map(String::as_str).collect();
            graph.execution_order_of(&targets, &config_path)?
        }
        (None, Some(base_ref)) => {
            let changed = affected::changed_files(cwd, base_ref)?;
            let seeds = affected::seed_tasks(&loaded.tasks, &changed)?;
            if seeds.is_empty() {
                writeln!(out, "Nothing affected since {base_ref}.").ok();
                return Ok(());
            }
            graph.affected_order(&seeds, &config_path)?
        }
        (None, None) => bail!("pass a task name, or --affected <REF>"),
        (Some(_), Some(_)) => unreachable!("clap already refuses task and --affected together"),
    };

    report.run.tasks = count(order.len());
    report.run.cacheable = count(order.iter().filter(|task| task.cacheable).count());

    report.stage = ErrorKind::Cache;
    let mode = if args.no_cache {
        CacheMode::Off
    } else if args.verify {
        CacheMode::Verify
    } else {
        CacheMode::ReadWrite
    };
    let cache_root = match &args.cache_dir {
        Some(dir) => dir.clone(),
        None => Store::default_root()?,
    };
    let cache = Cache::open(&cache_root, mode)?;
    let mut session = Session::new(Some(&cache));

    report.stage = ErrorKind::TaskFailed;
    let result = if args.parallel {
        parallel::run(&order, &mut session, out)
    } else {
        exec::run(&order, &mut session, out)
    };
    /* Telemetry's `hits` has always counted verified runs too; keep its meaning. */
    report.run.hits = session.hits + session.verified;
    report.run.misses = session.misses;
    result?;
    session.summarise(out);
    Ok(())
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
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
            no_cache: false,
            verify: false,
            /* Never the machine's real cache, from a test. */
            cache_dir: Some(config.with_file_name(".culm-cache")),
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
            no_cache: false,
            verify: false,
            cache_dir: Some(dir.path().join(".culm-cache")),
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
