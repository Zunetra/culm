// === Root registration

/* The bridge between the scaffolder and the three real runner backends. Each
 * backend module (`turborepo`, `moon`, `nx`) only knows how to turn a list of
 * apps and tasks into that tool's own config text; this module is what
 * decides, for one real `culm new` run, which apps and tasks that list
 * actually is, and writes the result to disk. Split out from `cli::new`
 * because "what apps did this selection produce" is a scaffold question and
 * "what does Turbo/Moon/Nx need to be told" is a runner-backend question, and
 * conflating them would mean neither module could be read on its own. */

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::layer::{Category, CatalogEntry};
use crate::scaffold::Variables;

use super::moon::{MoonApp, MoonTask, generate_project, generate_workspace};
use super::nx::{NxApp, generate as generate_nx};
use super::turborepo::{TurboApp, generate as generate_turbo};

/* Every app layer's own `package.json` carries this same set regardless of
 * framework, per docs/CONTEXT.md decision 19's comprehensiveness bar: `dev`,
 * `build`, `test`. Backends that need to know a task's own dependency
 * (Moon's `test` depending on this project's own `build`) hardcode that
 * relationship here rather than reading it back out of a merged
 * `package.json`, which this module never parses. */
const APP_TASKS: [&str; 3] = ["dev", "build", "test"];

/// The quality scripts that live at the workspace root rather than in any
/// one app's `package.json`, matching `quality/eslint`'s `lint` and
/// `quality/prettier`'s `format`. Only offered to a backend when the layer
/// that owns it was actually selected, so an unselected quality layer never
/// produces a task pointing at a script that does not exist.
fn root_tasks(entries: &[&CatalogEntry]) -> Vec<String> {
    let mut tasks = Vec::new();
    if entries
        .iter()
        .any(|entry| entry.manifest.category == Category::Quality && entry.manifest.id == "eslint")
    {
        tasks.push("lint".to_owned());
    }
    if entries.iter().any(|entry| {
        entry.manifest.category == Category::Quality && entry.manifest.id == "prettier"
    }) {
        tasks.push("format".to_owned());
    }
    tasks
}

/// Every frontend or backend layer in the selection, paired with the
/// variables that resolved its own `app_dir`/`pkg_name`. Data and toolchain
/// layers are not applications, the same distinction `scaffold::layout`
/// already draws.
fn apps<'a, 'b>(
    entries: &'b [&'a CatalogEntry],
    variables: &'b [Variables],
) -> Vec<(&'a CatalogEntry, &'b Variables)> {
    entries
        .iter()
        .zip(variables.iter())
        .filter(|(entry, _)| {
            matches!(entry.manifest.category, Category::Frontend | Category::Backend)
        })
        .map(|(entry, vars)| (*entry, vars))
        .collect()
}

/// Writes whichever root config the selected runner layer needs, or does
/// nothing for `runner/none` and any runner this binary does not carry a
/// real backend for. Called once, after the scaffold itself has already
/// been written to disk, since a backend needs to see the final `app_dir`
/// for every application before it can register one.
pub fn write_root_config(
    entries: &[&CatalogEntry],
    variables: &[Variables],
    root: &Path,
) -> Result<()> {
    let Some(runner) = entries
        .iter()
        .find(|entry| entry.manifest.category == Category::Runner)
    else {
        return Ok(());
    };

    match runner.manifest.id.as_str() {
        "turborepo" => write_turborepo(entries, variables, root),
        "moon" => write_moon(entries, variables, root),
        "nx" => write_nx(entries, variables, root),
        _ => Ok(()),
    }
}

fn write_turborepo(entries: &[&CatalogEntry], variables: &[Variables], root: &Path) -> Result<()> {
    let apps: Vec<TurboApp> = apps(entries, variables)
        .into_iter()
        .map(|(entry, vars)| TurboApp {
            pkg_name: vars
                .get("pkg_name")
                .unwrap_or(&entry.manifest.id)
                .to_owned(),
            tasks: APP_TASKS.iter().map(|t| (*t).to_owned()).collect(),
        })
        .collect();

    let doc = generate_turbo(&apps, &root_tasks(entries));
    let path = root.join("turbo.json");
    fs::write(&path, doc).with_context(|| format!("cannot write {}", path.display()))
}

fn write_moon(entries: &[&CatalogEntry], variables: &[Variables], root: &Path) -> Result<()> {
    let moon_apps: Vec<MoonApp> = apps(entries, variables)
        .into_iter()
        .map(|(entry, vars)| MoonApp {
            id: entry.manifest.id.clone(),
            app_dir: vars.get("app_dir").unwrap_or("src").to_owned(),
            tasks: vec![
                MoonTask {
                    name: "dev".to_owned(),
                    command: "pnpm run dev".to_owned(),
                    deps: Vec::new(),
                },
                MoonTask {
                    name: "build".to_owned(),
                    command: "pnpm run build".to_owned(),
                    deps: Vec::new(),
                },
                MoonTask {
                    name: "test".to_owned(),
                    command: "pnpm run test".to_owned(),
                    deps: vec!["build".to_owned()],
                },
            ],
        })
        .collect();

    let workspace_dir = root.join(".moon");
    fs::create_dir_all(&workspace_dir)
        .with_context(|| format!("cannot create {}", workspace_dir.display()))?;
    let workspace_path = workspace_dir.join("workspace.yml");
    fs::write(&workspace_path, generate_workspace(&moon_apps))
        .with_context(|| format!("cannot write {}", workspace_path.display()))?;

    for app in &moon_apps {
        let project_dir = root.join(&app.app_dir);
        fs::create_dir_all(&project_dir)
            .with_context(|| format!("cannot create {}", project_dir.display()))?;
        let project_path = project_dir.join("moon.yml");
        fs::write(&project_path, generate_project(app))
            .with_context(|| format!("cannot write {}", project_path.display()))?;
    }

    Ok(())
}

fn write_nx(entries: &[&CatalogEntry], variables: &[Variables], root: &Path) -> Result<()> {
    let nx_apps: Vec<NxApp> = apps(entries, variables)
        .into_iter()
        .map(|(entry, vars)| NxApp {
            id: entry.manifest.id.clone(),
            app_dir: vars.get("app_dir").unwrap_or("src").to_owned(),
        })
        .collect();

    let doc = generate_nx(&nx_apps);
    let path = root.join("nx.json");
    fs::write(&path, doc).with_context(|| format!("cannot write {}", path.display()))
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{LayerSource, Manifest};
    use std::path::PathBuf;

    fn entry(source: &str) -> CatalogEntry {
        CatalogEntry {
            manifest: Manifest::parse("layer.toml", source).unwrap(),
            source: LayerSource::Disk(PathBuf::from("unused")),
        }
    }

    #[test]
    fn no_backend_for_runner_none_writes_nothing() {
        let none = entry("id = \"none\"\ncategory = \"runner\"\n");
        let dir = tempfile::tempdir().unwrap();
        write_root_config(&[&none], &[Variables::new()], dir.path()).unwrap();
        assert!(!dir.path().join("turbo.json").exists());
        assert!(!dir.path().join("nx.json").exists());
        assert!(!dir.path().join(".moon").exists());
    }

    #[test]
    fn turborepo_writes_turbo_json_with_the_apps_own_tasks() {
        let runner = entry("id = \"turborepo\"\ncategory = \"runner\"\n");
        let frontend = entry("id = \"react-vite\"\ncategory = \"frontend\"\n");
        let vars = Variables::new().with("pkg_name", "demo-react-vite");
        let dir = tempfile::tempdir().unwrap();

        write_root_config(
            &[&runner, &frontend],
            &[Variables::new(), vars],
            dir.path(),
        )
        .unwrap();

        let doc = fs::read_to_string(dir.path().join("turbo.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert!(parsed["tasks"]["build"].is_object());
        assert!(parsed["tasks"]["dev"]["persistent"] == true);
    }

    #[test]
    fn nx_writes_nx_json_registering_every_app_by_its_app_dir() {
        let runner = entry("id = \"nx\"\ncategory = \"runner\"\n");
        let backend = entry("id = \"axum\"\ncategory = \"backend\"\n");
        let vars = Variables::new().with("app_dir", "src/axum");
        let dir = tempfile::tempdir().unwrap();

        write_root_config(&[&runner, &backend], &[Variables::new(), vars], dir.path()).unwrap();

        let doc = fs::read_to_string(dir.path().join("nx.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert_eq!(parsed["projects"]["axum"], "src/axum");
    }

    #[test]
    fn moon_writes_the_workspace_file_and_one_project_file_per_app() {
        let runner = entry("id = \"moon\"\ncategory = \"runner\"\n");
        let frontend = entry("id = \"nuxt\"\ncategory = \"frontend\"\n");
        let vars = Variables::new().with("app_dir", "src/nuxt");
        let dir = tempfile::tempdir().unwrap();

        write_root_config(
            &[&runner, &frontend],
            &[Variables::new(), vars],
            dir.path(),
        )
        .unwrap();

        assert!(dir.path().join(".moon/workspace.yml").exists());
        assert!(dir.path().join("src/nuxt/moon.yml").exists());
    }

    #[test]
    fn root_tasks_only_appear_when_their_quality_layer_is_selected() {
        let runner = entry("id = \"turborepo\"\ncategory = \"runner\"\n");
        let eslint = entry("id = \"eslint\"\ncategory = \"quality\"\n");
        let dir = tempfile::tempdir().unwrap();

        write_root_config(
            &[&runner, &eslint],
            &[Variables::new(), Variables::new()],
            dir.path(),
        )
        .unwrap();

        let doc = fs::read_to_string(dir.path().join("turbo.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert!(parsed["tasks"].get("//#lint").is_some());
        assert!(parsed["tasks"].get("//#format").is_none());
    }
}
