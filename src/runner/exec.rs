// === Execution

/* Sequential only, per r0.1's own scope: no caching, no parallelism, just the
 * graph and correct ordering. A task's `command` is a shell string rather
 * than argv, so it is run through a shell rather than split on whitespace,
 * which would break the first command using a pipe or a quoted argument. */

use std::io::Write;
use std::path::Path;
use std::process::Command;

use super::error::RunnerError;
use super::task::Task;

/// Runs every task in `order`, in order, stopping at the first failure.
pub fn run(order: &[&Task], root: &Path, out: &mut impl Write) -> Result<(), RunnerError> {
    for task in order {
        writeln!(out, "> {} ({})", task.name, task.command).ok();

        let status = shell(&task.command)
            .current_dir(root)
            .status()
            .map_err(|e| RunnerError::NotStarted {
                command: task.command.clone(),
                message: e.to_string(),
            })?;

        if !status.success() {
            return Err(RunnerError::TaskFailed {
                task: task.name.clone(),
                status: status.code(),
            });
        }
    }
    Ok(())
}

/// Shared with `runner::parallel`, which runs the same shell invocation
/// concurrently instead of one after another; the two only ever differ in
/// scheduling, never in how a single command is actually started.
#[cfg(unix)]
pub(super) fn shell(command: &str) -> Command {
    let mut process = Command::new("sh");
    process.arg("-c").arg(command);
    process
}

#[cfg(windows)]
pub(super) fn shell(command: &str) -> Command {
    let mut process = Command::new("cmd");
    process.arg("/C").arg(command);
    process
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn task(name: &str, command: &str) -> Task {
        Task {
            name: name.to_owned(),
            project: "demo".to_owned(),
            command: command.to_owned(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            depends_on: Vec::new(),
            cacheable: false,
            declared_env: Vec::new(),
        }
    }

    #[test]
    fn every_task_runs_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("order.txt");

        let a = task("a", &format!("echo a >> {}", marker.display()));
        let b = task("b", &format!("echo b >> {}", marker.display()));
        let order = [&a, &b];

        let mut out: Vec<u8> = Vec::new();
        run(&order, dir.path(), &mut out).unwrap();

        let written = std::fs::read_to_string(&marker).unwrap();
        assert_eq!(written.lines().collect::<Vec<_>>(), vec!["a", "b"]);
    }

    #[test]
    fn a_failing_task_stops_the_run_and_reports_its_exit_code() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("reached.txt");

        let fails = task("fails", "exit 3");
        let never = task("never", &format!("touch {}", marker.display()));
        let order = [&fails, &never];

        let mut out: Vec<u8> = Vec::new();
        let error = run(&order, dir.path(), &mut out).unwrap_err();

        match error {
            RunnerError::TaskFailed { task, status } => {
                assert_eq!(task, "fails");
                assert_eq!(status, Some(3));
            }
            other => panic!("expected TaskFailed, got {other}"),
        }
        assert!(
            !marker.exists(),
            "the task after the failure must not have run"
        );
    }
}
