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
}
