// === The exact payload

/* Every field decision 9 names, and nothing it forbids: no repository name, no
 * file path, no task name, no environment variable value, no command string,
 * no git remote. Adding a field here is a decision 9 change, not a one-line
 * patch, which is why the command modules fill these types rather than
 * defining their own: nothing outside this file can decide what leaves the
 * machine.
 *
 * Every key is always present, `null` where it does not apply, so the shape
 * is fixed and docs/TELEMETRY.md can show it verbatim. */

use serde::Serialize;
use uuid::Uuid;

/// Bumped whenever a field is added, removed, or changes meaning.
pub const SCHEMA: u32 = 2;

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub schema: u32,
    pub id: Uuid,
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    #[serde(flatten)]
    pub outcome: Outcome,
}

/// Everything about one invocation that is not about the machine.
#[derive(Debug, Clone, Serialize)]
pub struct Outcome {
    /// The subcommand name alone: `new`, `run`, `cache`, `graph`.
    pub command: &'static str,
    pub exit: Exit,
    /// Which stage failed, from a closed set. Never an error message.
    pub error_kind: Option<ErrorKind>,
    pub duration_ms: u64,
    /// Present only for `new`.
    pub scaffold: Option<Scaffold>,
    /// Present only for `run`.
    pub run: Option<RunStats>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Exit {
    Ok,
    Error,
    Cancelled,
}

/// Where a run was when it failed.
/// A command sets its current stage as it goes, so a failure reports how far
/// it got without any error text being inspected or sent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    #[default]
    Other,
    /// Choosing layers: flags, the wizard, or reading a preset.
    Select,
    /// Fetching a preset from a git URL.
    PresetFetch,
    /// Resolving and validating the selection.
    Resolve,
    /// Building the merged file set.
    Merge,
    /// Writing the workspace to disk.
    Write,
    /// A package manager install.
    Install,
    /// Resolving a version against a registry's release age guard.
    VersionResolve,
    /// Reading or parsing `culm.toml`.
    Config,
    /// Building or walking the task graph.
    Graph,
    /// A task's own command exited non-zero.
    TaskFailed,
    /// The local or remote cache.
    Cache,
}

/// What a `culm new` run chose. Only first-party layer ids are ever listed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Scaffold {
    pub shape: Option<String>,
    pub layers: Vec<String>,
    pub selected_via: Option<SelectedVia>,
    /// A first-party preset's name, or just `file` or `git`. Never the source
    /// string itself, which can be a repository name or a path.
    pub preset: Option<String>,
    pub install: Option<Install>,
    /// `pnpm`, `cargo`, or `go`, when `install` is `failed`.
    pub install_failed_manager: Option<String>,
    pub saved_preset: bool,
    /// The wizard step, or `confirm`, where the user backed out.
    pub cancelled_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectedVia {
    Flags,
    Preset,
    Wizard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Install {
    Ok,
    Failed,
    Skipped,
    /// The selection had nothing to install.
    None,
}

/// What a `culm run` did, as counts and flags only. Never a task or project name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RunStats {
    pub projects: u32,
    pub tasks: u32,
    pub cacheable: u32,
    pub hits: u32,
    pub misses: u32,
    pub parallel: bool,
    pub affected: bool,
    pub remote_cache: bool,
}

impl Event {
    pub fn new(id: Uuid, outcome: Outcome) -> Self {
        Self {
            schema: SCHEMA,
            id,
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            outcome,
        }
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(value: &serde_json::Value) -> Vec<&str> {
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort();
        keys
    }

    fn outcome(scaffold: Option<Scaffold>, run: Option<RunStats>) -> Outcome {
        Outcome {
            command: "new",
            exit: Exit::Ok,
            error_kind: None,
            duration_ms: 10,
            scaffold,
            run,
        }
    }

    /* The binding list in docs/TELEMETRY.md. A new key fails this test, which
     * is the point: growing the payload means editing that document too. */
    #[test]
    fn the_payload_carries_exactly_the_documented_keys() {
        let event = Event::new(
            Uuid::new_v4(),
            outcome(Some(Scaffold::default()), Some(RunStats::default())),
        );
        let value = serde_json::to_value(&event).unwrap();

        assert_eq!(
            keys(&value),
            vec![
                "arch",
                "command",
                "duration_ms",
                "error_kind",
                "exit",
                "id",
                "os",
                "run",
                "scaffold",
                "schema",
                "version"
            ]
        );
        assert_eq!(
            keys(&value["scaffold"]),
            vec![
                "cancelled_at",
                "install",
                "install_failed_manager",
                "layers",
                "preset",
                "saved_preset",
                "selected_via",
                "shape"
            ]
        );
        assert_eq!(
            keys(&value["run"]),
            vec![
                "affected",
                "cacheable",
                "hits",
                "misses",
                "parallel",
                "projects",
                "remote_cache",
                "tasks"
            ]
        );
    }

    #[test]
    fn absent_blocks_are_null_rather_than_missing() {
        let event = Event::new(Uuid::new_v4(), outcome(None, None));
        let value = serde_json::to_value(&event).unwrap();
        assert!(value["scaffold"].is_null());
        assert!(value["run"].is_null());
        assert!(value["error_kind"].is_null());
        assert_eq!(value["schema"], 2);
    }

    #[test]
    fn enums_render_as_snake_case_words() {
        let mut o = outcome(None, None);
        o.exit = Exit::Error;
        o.error_kind = Some(ErrorKind::TaskFailed);
        let value = serde_json::to_value(Event::new(Uuid::new_v4(), o)).unwrap();
        assert_eq!(value["exit"], "error");
        assert_eq!(value["error_kind"], "task_failed");
    }
}
