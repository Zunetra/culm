// === Workspace

/* docs/DESIGN.md section 3.2: one `culm.toml` per project. The workspace
 * root is the nearest directory at or above the working directory whose
 * `culm.toml` has a `[workspace]` table; every other `culm.toml` under it
 * (the walk honours `.gitignore`) is a project, named by its path from the
 * root, so a name is the same on every machine and in every clone.
 *
 * Without a `[workspace]` anywhere above, the `culm.toml` in the working
 * directory is the whole run, exactly as before workspaces existed, and task
 * names stay bare.
 *
 * In a workspace every task is renamed `<project>:<task>` and every
 * `depends_on` entry is rewritten to that form before the graph sees it, so
 * the graph, the cache, and `--affected` never need to know workspaces
 * exist:
 *
 * - `build` is `build` in the same project.
 * - `libs/ui:build` is `build` in the project at `libs/ui`.
 * - `^build` is `build` in every project named in `[project] dependencies`
 *   that defines one. */

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use super::config::RunnerConfig;
use super::error::RunnerError;
use super::task::{Task, tasks_of};

const CONFIG: &str = "culm.toml";

/// The id of the workspace root's own project.
pub const ROOT: &str = ".";

/// Every task a run can see, plus where it was found.
#[derive(Debug)]
pub struct Loaded {
    pub tasks: Vec<Task>,
    /// The file named in error messages: the root `culm.toml` in a
    /// workspace, otherwise the one file read.
    pub config_path: PathBuf,
    /// `None` outside a workspace.
    pub root: Option<PathBuf>,
    /// The project the working directory is in, when it is a workspace
    /// member other than the root.
    pub current: Option<String>,
    pub projects: usize,
}

impl Loaded {
    /// Resolves what `culm run <name>` means here: a qualified name as is;
    /// a bare name inside a member as that member's task; a bare name at the
    /// root as that task in every project that defines it.
    pub fn targets(&self, name: &str) -> Result<Vec<String>, RunnerError> {
        if self.root.is_none() || name.contains(':') {
            return Ok(vec![name.to_owned()]);
        }
        if let Some(project) = &self.current {
            return Ok(vec![qualify(project, name)]);
        }
        let targets: Vec<String> = self
            .tasks
            .iter()
            .filter(|task| {
                task.name
                    .rsplit_once(':')
                    .is_some_and(|(_, bare)| bare == name)
            })
            .map(|task| task.name.clone())
            .collect();
        if targets.is_empty() {
            return Err(RunnerError::UnknownTask {
                task: name.to_owned(),
                path: self.config_path.clone(),
            });
        }
        Ok(targets)
    }
}

/// Loads the tasks for a run started in `cwd`. `explicit` is `--config`,
/// which always means a single project.
pub fn load(cwd: &Path, explicit: Option<&Path>) -> Result<Loaded, RunnerError> {
    if let Some(path) = explicit {
        return single(path);
    }
    match find_root(cwd)? {
        Some(root) => workspace(&root, cwd),
        None => single(&cwd.join(CONFIG)),
    }
}

fn single(path: &Path) -> Result<Loaded, RunnerError> {
    let config = RunnerConfig::load(path)?;
    let dir = path.parent().unwrap_or(Path::new(".")).to_owned();
    let project = dir
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| ROOT.to_owned());
    Ok(Loaded {
        tasks: tasks_of(&config, &project, &dir),
        config_path: path.to_owned(),
        root: None,
        current: None,
        projects: 1,
    })
}

/// The nearest directory at or above `cwd` whose `culm.toml` declares
/// `[workspace]`.
fn find_root(cwd: &Path) -> Result<Option<PathBuf>, RunnerError> {
    for dir in cwd.ancestors() {
        let path = dir.join(CONFIG);
        if path.is_file() && RunnerConfig::load(&path)?.workspace.is_some() {
            return Ok(Some(dir.to_owned()));
        }
    }
    Ok(None)
}

fn workspace(root: &Path, cwd: &Path) -> Result<Loaded, RunnerError> {
    let root_config = root.join(CONFIG);
    let mut projects: BTreeMap<String, (PathBuf, RunnerConfig)> = BTreeMap::new();

    let walker = WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build();
    for entry in walker {
        let entry = entry.map_err(|error| RunnerError::Unreadable {
            path: root.to_owned(),
            message: error.to_string(),
        })?;
        if entry.file_name() != CONFIG || !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.into_path();
        let dir = path.parent().unwrap_or(root).to_owned();
        let config = RunnerConfig::load(&path)?;
        if dir != root && config.workspace.is_some() {
            return Err(RunnerError::NestedWorkspace { path });
        }
        projects.insert(project_id(root, &dir), (dir, config));
    }

    let mut tasks = Vec::new();
    for (id, (dir, config)) in &projects {
        for mut task in tasks_of(config, id, dir) {
            let bare = task.name.clone();
            task.depends_on =
                resolve_dependencies(id, &bare, &task.depends_on, config, &projects, &root_config)?;
            task.name = qualify(id, &bare);
            tasks.push(task);
        }
    }

    let current = projects
        .iter()
        .filter(|(id, _)| id.as_str() != ROOT)
        .filter(|(_, (dir, _))| cwd.starts_with(dir))
        .max_by_key(|(_, (dir, _))| dir.components().count())
        .map(|(id, _)| id.clone());

    Ok(Loaded {
        tasks,
        config_path: root_config,
        root: Some(root.to_owned()),
        current,
        projects: projects.len(),
    })
}

fn resolve_dependencies(
    project: &str,
    task: &str,
    raw: &[String],
    config: &RunnerConfig,
    projects: &BTreeMap<String, (PathBuf, RunnerConfig)>,
    config_path: &Path,
) -> Result<Vec<String>, RunnerError> {
    let unknown = |depends_on: &str| RunnerError::UnknownDependency {
        task: qualify(project, task),
        depends_on: depends_on.to_owned(),
        path: config_path.to_owned(),
    };

    let mut resolved = Vec::new();
    for entry in raw {
        if let Some(bare) = entry.strip_prefix('^') {
            for dependency in &config.project.dependencies {
                let (_, other) = projects.get(dependency.as_str()).ok_or_else(|| {
                    RunnerError::UnknownProject {
                        project: project.to_owned(),
                        dependency: dependency.clone(),
                    }
                })?;
                if other.tasks.contains_key(bare) {
                    resolved.push(qualify(dependency, bare));
                }
            }
        } else if let Some((other, bare)) = entry.rsplit_once(':') {
            let (_, config) = projects.get(other).ok_or_else(|| unknown(entry))?;
            if !config.tasks.contains_key(bare) {
                return Err(unknown(entry));
            }
            resolved.push(entry.clone());
        } else {
            resolved.push(qualify(project, entry));
        }
    }
    Ok(resolved)
}

fn qualify(project: &str, task: &str) -> String {
    format!("{project}:{task}")
}

/// A project's path from the root, with `/` separators on every platform so
/// ids and cache keys agree across machines.
fn project_id(root: &Path, dir: &Path) -> String {
    let relative = dir.strip_prefix(root).unwrap_or(dir);
    let parts: Vec<String> = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        ROOT.to_owned()
    } else {
        parts.join("/")
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, path: &str, contents: &str) {
        let full = root.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, contents).unwrap();
    }

    fn names(loaded: &Loaded) -> Vec<&str> {
        let mut names: Vec<&str> = loaded.tasks.iter().map(|task| task.name.as_str()).collect();
        names.sort();
        names
    }

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "culm.toml",
            "[workspace]\n\n[tasks.lint]\ncommand = \"true\"\n",
        );
        write(
            dir.path(),
            "libs/ui/culm.toml",
            "[tasks.build]\ncommand = \"true\"\n",
        );
        write(
            dir.path(),
            "apps/web/culm.toml",
            r#"
            [project]
            dependencies = ["libs/ui"]

            [tasks.build]
            command = "true"
            depends_on = ["^build", "codegen"]

            [tasks.codegen]
            command = "true"

            [tasks.test]
            command = "true"
            depends_on = ["libs/ui:build"]
            "#,
        );
        dir
    }

    #[test]
    fn without_a_workspace_names_stay_bare() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "culm.toml",
            "[tasks.build]\ncommand = \"true\"\n",
        );
        let loaded = load(dir.path(), None).unwrap();
        assert_eq!(names(&loaded), vec!["build"]);
        assert!(loaded.root.is_none());
    }

    #[test]
    fn every_member_is_found_and_named_by_its_path() {
        let dir = fixture();
        let loaded = load(dir.path(), None).unwrap();
        assert_eq!(
            names(&loaded),
            vec![
                ".:lint",
                "apps/web:build",
                "apps/web:codegen",
                "apps/web:test",
                "libs/ui:build"
            ]
        );
        assert_eq!(loaded.projects, 3);
    }

    #[test]
    fn each_dependency_form_resolves_to_a_qualified_name() {
        let dir = fixture();
        let loaded = load(dir.path(), None).unwrap();
        let build = loaded
            .tasks
            .iter()
            .find(|t| t.name == "apps/web:build")
            .unwrap();
        assert_eq!(build.depends_on, vec!["libs/ui:build", "apps/web:codegen"]);
        let test = loaded
            .tasks
            .iter()
            .find(|t| t.name == "apps/web:test")
            .unwrap();
        assert_eq!(test.depends_on, vec!["libs/ui:build"]);
    }

    #[test]
    fn a_task_runs_in_its_own_project_directory() {
        let dir = fixture();
        let loaded = load(dir.path(), None).unwrap();
        let build = loaded
            .tasks
            .iter()
            .find(|t| t.name == "libs/ui:build")
            .unwrap();
        assert_eq!(build.dir, dir.path().join("libs/ui"));
    }

    #[test]
    fn a_bare_name_at_the_root_means_every_project_that_has_it() {
        let dir = fixture();
        let loaded = load(dir.path(), None).unwrap();
        assert_eq!(
            loaded.targets("build").unwrap(),
            vec!["apps/web:build", "libs/ui:build"]
        );
    }

    #[test]
    fn a_bare_name_inside_a_member_means_that_members_task() {
        let dir = fixture();
        let loaded = load(&dir.path().join("apps/web/src"), None).unwrap();
        assert_eq!(loaded.targets("build").unwrap(), vec!["apps/web:build"]);
    }

    #[test]
    fn a_reference_to_a_missing_project_task_is_refused() {
        let dir = fixture();
        write(
            dir.path(),
            "apps/api/culm.toml",
            "[tasks.build]\ncommand = \"true\"\ndepends_on = [\"libs/ui:nope\"]\n",
        );
        assert!(matches!(
            load(dir.path(), None),
            Err(RunnerError::UnknownDependency { .. })
        ));
    }

    #[test]
    fn gitignored_directories_are_not_searched_for_projects() {
        let dir = fixture();
        write(dir.path(), ".gitignore", "node_modules/\n");
        write(
            dir.path(),
            "node_modules/pkg/culm.toml",
            "[tasks.build]\ncommand = \"true\"\n",
        );
        let loaded = load(dir.path(), None).unwrap();
        assert!(
            !names(&loaded)
                .iter()
                .any(|name| name.starts_with("node_modules"))
        );
    }
}
