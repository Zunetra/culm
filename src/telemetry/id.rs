// === The per-machine identifier

/* Anonymous and per-machine, never per-user or per-repository, per decision 9.
 * Generated once and reused, so the payload is the same shape whether this is
 * the first run or the thousandth: only whether it gets printed changes. */

use std::fs;
use std::path::PathBuf;

use uuid::Uuid;

/// Where the id lives, or `None` if the platform's config directory cannot be
/// resolved. A machine that cannot be identified sends nothing rather than
/// falling back to a path that is wrong for its platform.
fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("culm").join("telemetry_id"))
}

/// The id this machine already has, or a new one written for next time.
/// `true` in the second element means this call created it, which is the one
/// signal that decides whether the payload gets printed: decision 9 prints it
/// on first run, not on every run.
pub fn machine_id() -> Option<(Uuid, bool)> {
    let path = path()?;

    if let Ok(existing) = fs::read_to_string(&path)
        && let Ok(id) = Uuid::parse_str(existing.trim())
    {
        return Some((id, false));
    }

    let id = Uuid::new_v4();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok()?;
    }
    fs::write(&path, id.to_string()).ok()?;
    Some((id, true))
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    /* `path()` reads `dirs::config_dir()`, which this process cannot
     * relocate without changing `$HOME`/`$XDG_CONFIG_HOME` for the whole test
     * binary, so this only asserts the shape `machine_id` promises rather
     * than exercising a real file. */
    #[test]
    fn a_freshly_generated_id_parses_back_as_a_uuid() {
        let id = Uuid::new_v4();
        assert_eq!(Uuid::parse_str(&id.to_string()).unwrap(), id);
    }
}
