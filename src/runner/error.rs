// === Errors

use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RunnerError {
    #[error("{}: {message}", .path.display())]
    Unreadable { path: PathBuf, message: String },

    #[error("{}: {message}", .path.display())]
    Malformed { path: PathBuf, message: String },

    #[error("task `{task}` is not defined in `{}`", .path.display())]
    UnknownTask { task: String, path: PathBuf },

    #[error("task `{task}` depends on `{depends_on}`, which is not defined in `{}`", .path.display())]
    UnknownDependency {
        task: String,
        depends_on: String,
        path: PathBuf,
    },

    #[error("dependency cycle: {}", .cycle.join(" -> "))]
    Cycle { cycle: Vec<String> },

    #[error("`{command}` could not be started: {message}")]
    NotStarted { command: String, message: String },

    #[error("task `{task}` failed{}", .status.map(|code| format!(" with exit code {code}")).unwrap_or_else(|| " (terminated by a signal)".to_owned()))]
    TaskFailed { task: String, status: Option<i32> },

    #[error("cannot read the git repository at `{}`: {message}", .path.display())]
    GitUnavailable { path: PathBuf, message: String },

    #[error("`{git_ref}` is not a ref this repository knows about: {message}")]
    UnknownRef { git_ref: String, message: String },

    #[error(
        "project `{project}` depends on project `{dependency}`, which has no `culm.toml` in this workspace"
    )]
    UnknownProject { project: String, dependency: String },

    #[error("`{}` declares `[workspace]` inside another workspace; only the root may", .path.display())]
    NestedWorkspace { path: PathBuf },

    #[error("`{pattern}` is not a valid glob: {message}")]
    BadPattern { pattern: String, message: String },

    #[error(
        "task `{task}` declares output `{path}`, which is outside its project; outputs are deleted before a cache restore, so they must stay inside it"
    )]
    OutputOutsideProject { task: String, path: String },

    #[error("the task cache failed: {message}")]
    Cache { message: String },

    #[error("task `{task}` produced different output than its cache entry holds: {}. The cache is not safe for this task; report it, then `culm cache clear`", .paths.join(", "))]
    CacheMismatch { task: String, paths: Vec<String> },
}
