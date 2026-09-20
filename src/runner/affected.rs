// === Affected

/* r0.3, per docs/02-runner.md section 6: `--affected` via `git diff` against
 * a base ref, then walking the graph for downstream tasks. Two separate
 * questions live here on purpose: which files changed (git's job, this
 * module) and which of a task's declared `inputs` globs those files match
 * (an ordinary glob match, nothing git-specific about it). Turning "these
 * tasks changed" into "these tasks, plus what they need and what needs
 * them" is `TaskGraph::affected_order`'s job, not this module's; a changed
 * file only tells you where to start walking. */

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use git2::{DiffOptions, Repository};
use globset::{Glob, GlobSetBuilder};

use super::error::RunnerError;
use super::task::Task;

/// Every file that differs between `base_ref` and the working tree, relative
/// to the repository root. Covers both what changed since `base_ref` and
/// anything still uncommitted, because "what do I need to run before I
/// commit this" is the question `--affected` is actually asked to answer,
/// not only "what changed since the last commit".
pub fn changed_files(repo_root: &Path, base_ref: &str) -> Result<HashSet<PathBuf>, RunnerError> {
    let unavailable = |message: String| RunnerError::GitUnavailable {
        path: repo_root.to_owned(),
        message,
    };

    let repo = Repository::discover(repo_root).map_err(|e| unavailable(e.message().to_owned()))?;

    let base_object =
        repo.revparse_single(base_ref)
            .map_err(|e| RunnerError::UnknownRef {
                git_ref: base_ref.to_owned(),
                message: e.message().to_owned(),
            })?;
    let base_tree = base_object
        .peel_to_tree()
        .map_err(|e| unavailable(e.message().to_owned()))?;

    let mut options = DiffOptions::new();
    options.include_untracked(true).recurse_untracked_dirs(true);

    let diff = repo
        .diff_tree_to_workdir_with_index(Some(&base_tree), Some(&mut options))
        .map_err(|e| unavailable(e.message().to_owned()))?;

    let mut files = HashSet::new();
    diff.foreach(
        &mut |delta, _progress| {
            if let Some(path) = delta.new_file().path() {
                files.insert(path.to_path_buf());
            }
            if let Some(path) = delta.old_file().path() {
                files.insert(path.to_path_buf());
            }
            true
        },
        None,
        None,
        None,
    )
    .map_err(|e| unavailable(e.message().to_owned()))?;

    Ok(files)
}

/// Which of `tasks` are directly affected: at least one of their own
/// declared `inputs` glob patterns matches a changed file. This is only the
/// seed set; `TaskGraph::affected_order` is what expands it to the tasks
/// each seed needs and the tasks that need each seed.
pub fn seed_tasks<'a>(tasks: &'a [Task], changed: &HashSet<PathBuf>) -> Vec<&'a str> {
    tasks
        .iter()
        .filter(|task| task_matches(task, changed))
        .map(|task| task.name.as_str())
        .collect()
}

/// A task with no declared `inputs` is treated as always affected. Guessing
/// it is unaffected is the dangerous direction: a stale skip is exactly the
/// wrong-cache-hit failure mode docs/02-runner.md section 1 names as worse
/// than no cache at all, so an under-specified task errs toward running.
fn task_matches(task: &Task, changed: &HashSet<PathBuf>) -> bool {
    if task.inputs.is_empty() {
        return true;
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in &task.inputs {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    let Ok(set) = builder.build() else {
        return true;
    };

    changed.iter().any(|path| set.is_match(path))
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn task(name: &str, inputs: &[&str]) -> Task {
        Task {
            name: name.to_owned(),
            project: "demo".to_owned(),
            command: "echo".to_owned(),
            inputs: inputs.iter().map(|s| (*s).to_owned()).collect(),
            outputs: Vec::new(),
            depends_on: Vec::new(),
            cacheable: false,
            declared_env: Vec::new(),
        }
    }

    fn changed(paths: &[&str]) -> HashSet<PathBuf> {
        paths.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn a_task_whose_glob_matches_a_changed_file_is_a_seed() {
        let tasks = vec![task("build", &["src/**/*.rs"])];
        let seeds = seed_tasks(&tasks, &changed(&["src/main.rs"]));
        assert_eq!(seeds, vec!["build"]);
    }

    #[test]
    fn a_task_whose_glob_does_not_match_anything_changed_is_not_a_seed() {
        let tasks = vec![task("build", &["src/**/*.rs"])];
        let seeds = seed_tasks(&tasks, &changed(&["README.md"]));
        assert!(seeds.is_empty());
    }

    #[test]
    fn a_task_with_no_declared_inputs_is_always_a_seed() {
        let tasks = vec![task("build", &[])];
        let seeds = seed_tasks(&tasks, &changed(&["anything.txt"]));
        assert_eq!(seeds, vec!["build"]);
    }

    #[test]
    fn only_the_matching_tasks_among_several_become_seeds() {
        let tasks = vec![
            task("rust", &["src/**/*.rs"]),
            task("node", &["web/**/*.ts"]),
        ];
        let seeds = seed_tasks(&tasks, &changed(&["web/app.ts"]));
        assert_eq!(seeds, vec!["node"]);
    }
}
