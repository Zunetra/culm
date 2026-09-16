// === Writing to disk

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::error::ScaffoldError;
use super::plan::CopyPlan;

/// Writes a validated plan plus the merged file set into `root`.
/// Everything is assembled and checked before this is called, so this is the only function that touches disk and it does so once.
pub fn write(
    root: &Path,
    plan: &CopyPlan,
    merged: &BTreeMap<PathBuf, String>,
    modes: &BTreeMap<PathBuf, u32>,
) -> Result<(), ScaffoldError> {
    for planned in plan.files() {
        write_one(root, &planned.to, &planned.contents, planned.mode)?;
    }
    /* A merged file carries a mode only when the strategy that built it says
     * so, which today means hooks. A hook without the executable bit is
     * skipped by git in silence, so losing it here would be invisible.
     *
     * A merge that rendered to nothing is skipped rather than written as an
     * empty file: `yaml-list` renders "" when every layer's contribution to a
     * target resolved to a blank entry for this selection (`pnpm-workspace.yaml`
     * outside a monorepo, say), and a file that exists only to be empty is
     * worse than one that was never generated. */
    for (path, contents) in merged {
        if contents.is_empty() {
            continue;
        }
        write_one(root, path, contents, modes.get(path).copied())?;
    }
    Ok(())
}

fn write_one(
    root: &Path,
    relative: &Path,
    contents: &str,
    mode: Option<u32>,
) -> Result<(), ScaffoldError> {
    let target = root.join(relative);

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| ScaffoldError::Io {
            path: parent.to_path_buf(),
            message: e.to_string(),
        })?;
    }

    fs::write(&target, contents).map_err(|e| ScaffoldError::Io {
        path: target.clone(),
        message: e.to_string(),
    })?;

    apply_mode(&target, mode)
}

#[cfg(unix)]
fn apply_mode(target: &Path, mode: Option<u32>) -> Result<(), ScaffoldError> {
    use std::os::unix::fs::PermissionsExt;

    let Some(mode) = mode else {
        return Ok(());
    };
    fs::set_permissions(target, fs::Permissions::from_mode(mode)).map_err(|e| ScaffoldError::Io {
        path: target.to_path_buf(),
        message: e.to_string(),
    })
}

/* Windows has no POSIX permission bits, and a hook script's executable bit is
 * meaningless there. Silently ignoring it beats failing the scaffold. */
#[cfg(not(unix))]
fn apply_mode(_target: &Path, _mode: Option<u32>) -> Result<(), ScaffoldError> {
    Ok(())
}
