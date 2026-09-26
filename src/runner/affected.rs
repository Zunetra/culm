// === Affected

/* r0.3, per docs/DESIGN.md section 3.5: `--affected` via `git diff` against
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

use super::error::RunnerError;
use super::inputs;
use super::task::Task;

/// Every file that differs between `base_ref` and the working tree, as an
/// absolute path, so each task can match it from its own directory. Covers both what changed since `base_ref` and
/// anything still uncommitted, because "what do I need to run before I
/// commit this" is the question `--affected` is actually asked to answer,
/// not only "what changed since the last commit".
pub fn changed_files(repo_root: &Path, base_ref: &str) -> Result<HashSet<PathBuf>, RunnerError> {
    let unavailable = |message: String| RunnerError::GitUnavailable {
        path: repo_root.to_owned(),
        message,
    };

    let repo = Repository::discover(repo_root).map_err(|e| unavailable(e.message().to_owned()))?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| unavailable("a bare repository has no working tree".to_owned()))?;
    let workdir = std::fs::canonicalize(workdir).unwrap_or_else(|_| workdir.to_owned());

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
                files.insert(workdir.join(path));
            }
            if let Some(path) = delta.old_file().path() {
                files.insert(workdir.join(path));
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
pub fn seed_tasks<'a>(tasks: &'a [Task], changed: &HashSet<PathBuf>) -> Result<Vec<&'a str>, RunnerError> {
    let mut seeds = Vec::new();
    for task in tasks {
        if task_matches(task, changed)? {
            seeds.push(task.name.as_str());
        }
    }
    Ok(seeds)
}

/// A task with no declared `inputs` is treated as always affected. Guessing
/// it is unaffected is the dangerous direction: a stale skip is exactly the
/// wrong-cache-hit failure mode docs/DESIGN.md section 3.1 names as worse
/// than no cache at all, so an under-specified task errs toward running.
fn task_matches(task: &Task, changed: &HashSet<PathBuf>) -> Result<bool, RunnerError> {
    if task.inputs.is_empty() {
        return Ok(true);
    }
    let dir = std::fs::canonicalize(&task.dir).unwrap_or_else(|_| task.dir.clone());
    for path in changed {
        if inputs::input_matches(&dir, &task.inputs, path)? {
            return Ok(true);
        }
    }
    Ok(false)
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn task(name: &str, inputs: &[&str]) -> Task {
        Task {
            name: name.to_owned(),
            project: "demo".to_owned(),
            dir: PathBuf::from("/repo"),
            command: "echo".to_owned(),
            inputs: inputs.iter().map(|s| (*s).to_owned()).collect(),
            outputs: Vec::new(),
            depends_on: Vec::new(),
            cacheable: false,
            declared_env: Vec::new(),
        }
    }

    fn changed(paths: &[&str]) -> HashSet<PathBuf> {
        paths.iter().map(|path| Path::new("/repo").join(path)).collect()
    }

    #[test]
    fn a_task_whose_glob_matches_a_changed_file_is_a_seed() {
        let tasks = vec![task("build", &["src/**/*.rs"])];
        let seeds = seed_tasks(&tasks, &changed(&["src/main.rs"])).unwrap();
        assert_eq!(seeds, vec!["build"]);
    }

    #[test]
    fn a_task_whose_glob_does_not_match_anything_changed_is_not_a_seed() {
        let tasks = vec![task("build", &["src/**/*.rs"])];
        let seeds = seed_tasks(&tasks, &changed(&["README.md"])).unwrap();
        assert!(seeds.is_empty());
    }

    #[test]
    fn a_task_with_no_declared_inputs_is_always_a_seed() {
        let tasks = vec![task("build", &[])];
        let seeds = seed_tasks(&tasks, &changed(&["anything.txt"])).unwrap();
        assert_eq!(seeds, vec!["build"]);
    }

    #[test]
    fn only_the_matching_tasks_among_several_become_seeds() {
        let tasks = vec![
            task("rust", &["src/**/*.rs"]),
            task("node", &["web/**/*.ts"]),
        ];
        let seeds = seed_tasks(&tasks, &changed(&["web/app.ts"])).unwrap();
        assert_eq!(seeds, vec!["node"]);
    }
}
