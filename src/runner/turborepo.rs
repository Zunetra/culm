// === Turborepo backend

/* Decision 3: the scaffolder must be able to emit Nx, Turborepo, Moon, or
 * nothing, and none of those may be a config only culm's own runner can
 * consume. This module writes `turbo.json`, a file the real `turbo` binary
 * reads, so culm never gets involved once it is written.
 *
 * Turborepo needs to know the task shape before it can run anything: which
 * task names exist, which of those are scoped to a single package versus the
 * workspace root, and which are safe to cache. culm already knows this from
 * the layers selected and the layout they were given, so this turns that
 * knowledge into Turbo's own format rather than asking the user to hand
 * write it. */

use std::collections::BTreeSet;

/// One application's own package, as it will appear as `name` in its
/// `package.json` once decision 23's per-app manifest lands it there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurboApp {
    pub pkg_name: String,
    pub tasks: Vec<String>,
}

/// Builds the whole `turbo.json` document as text, ready to write to disk.
///
/// Every app-scoped task name (`build`, `test`, `dev`, ...) becomes one
/// ordinary entry in Turbo's `tasks` table, so it applies to every package
/// that defines a script by that name; Turbo does not need to be told which
/// packages have it. `build` depends on its own dependencies' `build`
/// (`^build`, Turbo's own syntax for "this task in every package I depend
/// on"), and produces output Turbo can cache. `dev` is a long running
/// process, never cached, matching Turbo's own documented shape for a dev
/// server. Anything else defaults to an empty entry, cacheable with no
/// declared outputs, which is still a valid task Turbo will run and report.
///
/// `root_tasks` are scripts owned by the workspace root rather than any one
/// package (a quality layer's `lint` or `format`, which decision 23 leaves
/// unnamespaced because at most one of each is ever selected). Turbo's own
/// root-task syntax is `//#<name>`; a bare name here would tell Turbo to run
/// that script in every package instead of the one place it actually lives.
pub fn generate(apps: &[TurboApp], root_tasks: &[String]) -> String {
    let mut names: BTreeSet<&str> = BTreeSet::new();
    for app in apps {
        for task in &app.tasks {
            names.insert(task.as_str());
        }
    }

    let mut tasks = serde_json::Map::new();
    for name in &names {
        let entry = match *name {
            "build" => serde_json::json!({
                "dependsOn": ["^build"],
                "outputs": ["dist/**", ".next/**", ".output/**", "build/**", "target/**"]
            }),
            "dev" => serde_json::json!({ "cache": false, "persistent": true }),
            "test" => serde_json::json!({ "dependsOn": ["build"] }),
            _ => serde_json::json!({}),
        };
        tasks.insert((*name).to_owned(), entry);
    }

    for name in root_tasks {
        tasks.insert(format!("//#{name}"), serde_json::json!({}));
    }

    let doc = serde_json::json!({
        "$schema": "https://turbo.build/schema.json",
        "tasks": tasks
    });

    serde_json::to_string_pretty(&doc).expect("a map of json! values always serialises") + "\n"
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn app(pkg_name: &str, tasks: &[&str]) -> TurboApp {
        TurboApp {
            pkg_name: pkg_name.to_owned(),
            tasks: tasks.iter().map(|t| (*t).to_owned()).collect(),
        }
    }

    #[test]
    fn build_depends_on_upstream_build_and_declares_outputs() {
        let doc = generate(&[app("web", &["build"])], &[]);
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert_eq!(parsed["tasks"]["build"]["dependsOn"][0], "^build");
        assert!(parsed["tasks"]["build"]["outputs"].is_array());
    }

    #[test]
    fn dev_is_never_cached() {
        let doc = generate(&[app("web", &["dev"])], &[]);
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert_eq!(parsed["tasks"]["dev"]["cache"], false);
        assert_eq!(parsed["tasks"]["dev"]["persistent"], true);
    }

    #[test]
    fn test_depends_on_this_packages_own_build() {
        let doc = generate(&[app("web", &["test"])], &[]);
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert_eq!(parsed["tasks"]["test"]["dependsOn"][0], "build");
    }

    #[test]
    fn a_root_only_task_uses_the_workspace_root_syntax() {
        let doc = generate(&[], &["lint".to_owned(), "format".to_owned()]);
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert!(parsed["tasks"].get("//#lint").is_some());
        assert!(parsed["tasks"].get("//#format").is_some());
        assert!(parsed["tasks"].get("lint").is_none());
    }

    #[test]
    fn two_apps_sharing_a_task_name_contribute_one_entry() {
        let doc = generate(&[app("web", &["build"]), app("api", &["build"])], &[]);
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert_eq!(parsed["tasks"].as_object().unwrap().len(), 1);
    }
}
