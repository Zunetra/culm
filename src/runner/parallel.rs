// === Parallel execution

/* r0.3, per docs/DESIGN.md section 3.5: parallel execution respecting the
 * dependency graph. A topological order alone (what `exec::run` already
 * takes) is enough for correctness but leaves independent tasks running one
 * after another for no reason; grouping that same order into levels, where
 * every task in a level has already had every dependency it needs satisfied
 * by an earlier level, is what lets those independent tasks actually run at
 * once.
 *
 * Levels are computed from the tasks' own `depends_on` names rather than
 * from `TaskGraph`'s internal `petgraph` structure, so this stays a plain
 * function over the same `&[&Task]` slice `exec::run` already takes: nothing
 * here needs to know how the graph is stored, only that its input is already
 * topologically ordered (`TaskGraph::execution_order` and
 * `TaskGraph::affected_order` both guarantee that). */

use std::collections::HashMap;
use std::io::Write;

use super::cache::{self, Plan, Session, Status};
use super::error::RunnerError;
use super::task::Task;

/// Groups a topologically ordered slice into levels: level 0 holds every
/// task with no dependency in `order` at all, level `n` holds a task whose
/// dependencies' own highest level is `n - 1`. Every task in one level is
/// safe to start at the same time, because by definition nothing in it still
/// waits on anything else in it.
pub fn levels<'a>(order: &[&'a Task]) -> Vec<Vec<&'a Task>> {
    let mut level_of: HashMap<&str, usize> = HashMap::new();

    for task in order {
        let level = task
            .depends_on
            .iter()
            .map(|dep| level_of.get(dep.as_str()).copied().unwrap_or(0) + 1)
            .max()
            .unwrap_or(0);
        level_of.insert(task.name.as_str(), level);
    }

    let max_level = level_of.values().copied().max().unwrap_or(0);
    let mut grouped: Vec<Vec<&Task>> = (0..=max_level).map(|_| Vec::new()).collect();
    for task in order {
        grouped[level_of[task.name.as_str()]].push(task);
    }
    grouped
}

/// Runs every level in order, and every task within a level concurrently on
/// its own OS thread. Stops at the first level containing a failure, after
/// every task that level already started has finished, so a run never
/// reports a task as still "running" when the process actually exits; tasks
/// in a level after the failing one never start at all, matching
/// `exec::run`'s own "stop at the first failure" contract.
///
/// A level is planned on this thread before anything in it starts, so cache
/// hits are restored up front and only real work is spread across threads.
pub fn run(
    order: &[&Task],
    session: &mut Session,
    out: &mut impl Write,
) -> Result<(), RunnerError> {
    for level in levels(order) {
        let mut to_run = Vec::new();
        for task in &level {
            match session.plan(task, out)? {
                Plan::Restored => session.count(Status::Hit),
                Plan::Run(pending) => to_run.push((*task, pending)),
            }
        }

        let store = session.cache();
        let results: Vec<Result<Status, RunnerError>> = std::thread::scope(|scope| {
            let handles: Vec<_> = to_run
                .iter()
                .map(|(task, pending)| {
                    scope.spawn(move || {
                        cache::run(task, store, pending.key.as_ref(), pending.previous.as_ref())
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("a task thread must not panic"))
                .collect()
        });

        for result in results {
            session.count(result?);
        }
    }
    Ok(())
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn task(name: &str, command: &str, depends_on: &[&str]) -> Task {
        Task {
            name: name.to_owned(),
            project: "demo".to_owned(),
            dir: std::path::PathBuf::from("."),
            command: command.to_owned(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            depends_on: depends_on.iter().map(|s| (*s).to_owned()).collect(),
            cacheable: false,
            declared_env: Vec::new(),
            persistent: false,
        }
    }

    #[test]
    fn independent_tasks_land_in_the_same_level() {
        let a = task("a", "true", &[]);
        let b = task("b", "true", &[]);
        let grouped = levels(&[&a, &b]);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].len(), 2);
    }

    #[test]
    fn a_dependent_lands_one_level_after_its_dependency() {
        let base = task("base", "true", &[]);
        let dependent = task("dependent", "true", &["base"]);
        let grouped = levels(&[&base, &dependent]);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[0][0].name, "base");
        assert_eq!(grouped[1][0].name, "dependent");
    }

    #[test]
    fn a_diamond_keeps_both_middle_tasks_in_one_level() {
        let base = task("base", "true", &[]);
        let left = task("left", "true", &["base"]);
        let right = task("right", "true", &["base"]);
        let top = task("top", "true", &["left", "right"]);
        let grouped = levels(&[&base, &left, &right, &top]);
        assert_eq!(grouped.len(), 3);
        assert_eq!(grouped[1].len(), 2);
        assert_eq!(grouped[2][0].name, "top");
    }

    #[test]
    fn every_task_in_a_level_actually_runs() {
        let dir = tempfile::tempdir().unwrap();
        let a_marker = dir.path().join("a.txt");
        let b_marker = dir.path().join("b.txt");
        let a = task("a", &format!("touch {}", a_marker.display()), &[]);
        let b = task("b", &format!("touch {}", b_marker.display()), &[]);

        let mut out: Vec<u8> = Vec::new();
        run(&[&a, &b], &mut Session::new(None), &mut out).unwrap();

        assert!(a_marker.exists());
        assert!(b_marker.exists());
    }

    #[test]
    fn a_failure_in_one_level_stops_the_next_level_from_starting() {
        let dir = tempfile::tempdir().unwrap();
        let marker = dir.path().join("never.txt");
        let fails = task("fails", "exit 3", &[]);
        let never = task("never", &format!("touch {}", marker.display()), &["fails"]);

        let mut out: Vec<u8> = Vec::new();
        let error = run(&[&fails, &never], &mut Session::new(None), &mut out).unwrap_err();

        assert!(matches!(error, RunnerError::TaskFailed { .. }));
        assert!(!marker.exists());
    }
}
