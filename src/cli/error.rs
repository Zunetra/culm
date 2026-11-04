// === CliError

/* The commands' own refusals, as a type rather than ad-hoc `bail!` text, so
 * telemetry can report which one happened by its code without ever reading
 * the message, which carries the user's paths and names. */

use std::path::PathBuf;

use thiserror::Error;

/// A refusal raised by a command itself rather than by the module it drives.
#[derive(Debug, Error)]
pub enum CliError {
    #[error("pass a task name, or --affected <REF>")]
    NoTaskOrAffected,

    #[error("`{}` defines no tasks", path.display())]
    NoTasksDefined { path: PathBuf },

    #[error("the project name is empty")]
    ProjectNameEmpty,

    #[error("`{name}` is not a usable project name")]
    ProjectNameUnusable { name: String },

    #[error("the project name `{name}` must be a single directory name, not a path")]
    ProjectNameIsPath { name: String },

    #[error("the project name `{name}` must not start with a dash")]
    ProjectNameStartsWithDash { name: String },

    #[error("cancelled: nothing was chosen")]
    Cancelled,

    #[error(
        "no layers chosen: pass --preset NAME, or at least one --layer, for example --layer backend/axum"
    )]
    NoLayersChosen,

    #[error("{} already exists and is not a directory", path.display())]
    DestinationNotADirectory { path: PathBuf },

    #[error("{} already exists and is not empty", path.display())]
    DestinationNotEmpty { path: PathBuf },

    #[error(
        "{message}\nThe workspace is on disk. Pass --min-release-age 0 to install without the guard."
    )]
    ReleaseAge { message: String },
}

impl CliError {
    /// The telemetry error code: which refusal, never its text.
    pub fn code(&self) -> &'static str {
        return match self {
            Self::NoTaskOrAffected => "cli_no_task_or_affected",
            Self::NoTasksDefined { .. } => "cli_no_tasks_defined",
            Self::ProjectNameEmpty => "cli_project_name_empty",
            Self::ProjectNameUnusable { .. } => "cli_project_name_unusable",
            Self::ProjectNameIsPath { .. } => "cli_project_name_is_path",
            Self::ProjectNameStartsWithDash { .. } => "cli_project_name_starts_with_dash",
            Self::Cancelled => "cli_cancelled",
            Self::NoLayersChosen => "cli_no_layers_chosen",
            Self::DestinationNotADirectory { .. } => "cli_destination_not_a_directory",
            Self::DestinationNotEmpty { .. } => "cli_destination_not_empty",
            Self::ReleaseAge { .. } => "cli_release_age",
        };
    }
}
