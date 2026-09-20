// === Nx backend

/* `layers/runner/nx` shipped as an honest stub: it installs the real `nx`
 * package and contributes nothing else, on record in docs/CONTEXT.md as the
 * one runner backend still owed real config. Nx discovers a project's tasks
 * from that project's own `package.json` scripts natively, so the only thing
 * missing was telling Nx where each project actually lives, which is what
 * `nx.json`'s `projects` map is for. */

/// One application, as Nx needs to see it: an id to register it under and
/// the directory it lives in relative to the workspace root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NxApp {
    pub id: String,
    pub app_dir: String,
}

/// The root `nx.json`, registering every project so `nx run-many` and
/// `nx graph` can find them. Per-project tasks are left to each package's own
/// `package.json` scripts, which Nx already reads without being told to, so
/// this only has to name where each project lives.
pub fn generate(apps: &[NxApp]) -> String {
    let mut projects = serde_json::Map::new();
    for app in apps {
        projects.insert(app.id.clone(), serde_json::json!(app.app_dir));
    }

    let doc = serde_json::json!({
        "$schema": "./node_modules/nx/schemas/nx-schema.json",
        "projects": projects
    });

    serde_json::to_string_pretty(&doc).expect("a map of json! values always serialises") + "\n"
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_app_is_registered_by_id_and_directory() {
        let apps = [
            NxApp {
                id: "web".to_owned(),
                app_dir: "src/web".to_owned(),
            },
            NxApp {
                id: "api".to_owned(),
                app_dir: "src/api".to_owned(),
            },
        ];
        let doc = generate(&apps);
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert_eq!(parsed["projects"]["web"], "src/web");
        assert_eq!(parsed["projects"]["api"], "src/api");
    }

    #[test]
    fn no_apps_still_produces_a_valid_empty_registration() {
        let doc = generate(&[]);
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert!(parsed["projects"].as_object().unwrap().is_empty());
    }
}
