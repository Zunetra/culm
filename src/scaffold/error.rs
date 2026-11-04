// === Errors

use std::path::PathBuf;

use thiserror::Error;

use crate::merge::LayerId;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScaffoldError {
    #[error("layer `{layer}` declares `{}` which is not in the layer directory", .from.display())]
    TemplateMissing { layer: LayerId, from: PathBuf },

    /// Two layers copy different content to the same destination.
    /// Named rather than resolved, for the same reason a scalar merge conflict is: last write wins is how a workspace ends up subtly wrong.
    #[error("layers `{a}` and `{b}` both write `{}` with different content", .to.display())]
    FileCollision { to: PathBuf, a: LayerId, b: LayerId },

    /// A destination that escapes the workspace root, whether by `..` or by being absolute.
    /// A layer is data, and a scaffolder that lets data write outside the directory it was pointed at is a scaffolder that can overwrite anything the user can.
    #[error("layer `{layer}` writes `{}`, which escapes the workspace root", .to.display())]
    EscapesRoot { layer: LayerId, to: PathBuf },

    #[error("{}: {message}", .path.display())]
    Io { path: PathBuf, message: String },
}

impl ScaffoldError {
    /// The telemetry error code: which failure, never its text.
    pub fn code(&self) -> &'static str {
        return match self {
            Self::TemplateMissing { .. } => "scaffold_template_missing",
            Self::FileCollision { .. } => "scaffold_file_collision",
            Self::EscapesRoot { .. } => "scaffold_escapes_root",
            Self::Io { .. } => "scaffold_io",
        };
    }
}
