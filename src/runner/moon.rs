// === Moon backend

/* Moon needs each project registered twice: once in the workspace-level
 * `.moon/workspace.yml` (which projects exist and where each one lives), and
 * once per project in that project's own `moon.yml` (which tasks it has).
 * Neither is optional. An app directory Moon was never told about is
 * invisible to `moon run`, so culm has to write both, the same way it writes
 * a real `turbo.json` for the Turborepo backend rather than leaving Moon to
 * discover a shape culm already resolved at scaffold time. */

/// One task inside a project's own `moon.yml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoonTask {
    pub name: String,
    pub command: String,
    pub deps: Vec<String>,
}

/// One application, as Moon needs to see it: an id to register it under, the
/// directory it lives in relative to the workspace root, and its own tasks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoonApp {
    pub id: String,
    pub app_dir: String,
    pub tasks: Vec<MoonTask>,
}

/// `.moon/workspace.yml`: every project this workspace holds, named and
/// located. Written once for the whole workspace, never per app.
pub fn generate_workspace(apps: &[MoonApp]) -> String {
    let mut out = String::from("projects:\n");
    for app in apps {
        out.push_str(&format!("  {}: {}\n", app.id, app.app_dir));
    }
    out
}

/// One project's own `moon.yml`, naming its tasks and their inter-task
/// dependencies. `deps` names another task in the same project by its own
/// task id (Moon's `<task>` shorthand), matching how `culm.toml`'s
/// `depends_on` already names a same-project task by its bare name.
pub fn generate_project(app: &MoonApp) -> String {
    let mut out = String::from("tasks:\n");
    for task in &app.tasks {
        out.push_str(&format!("  {}:\n", task.name));
        out.push_str(&format!("    command: '{}'\n", task.command));
        if !task.deps.is_empty() {
            out.push_str("    deps:\n");
            for dep in &task.deps {
                out.push_str(&format!("      - {dep}\n"));
            }
        }
    }
    out
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, dir: &str, tasks: &[(&str, &str, &[&str])]) -> MoonApp {
        MoonApp {
            id: id.to_owned(),
            app_dir: dir.to_owned(),
            tasks: tasks
                .iter()
                .map(|(name, command, deps)| MoonTask {
                    name: (*name).to_owned(),
                    command: (*command).to_owned(),
                    deps: deps.iter().map(|d| (*d).to_owned()).collect(),
                })
                .collect(),
        }
    }

    #[test]
    fn the_workspace_file_registers_every_project_by_id_and_path() {
        let apps = [app("web", "src/web", &[]), app("api", "src/api", &[])];
        let doc = generate_workspace(&apps);
        assert!(doc.contains("web: src/web"));
        assert!(doc.contains("api: src/api"));
    }

    #[test]
    fn a_project_file_names_its_own_tasks_and_their_dependencies() {
        let web = app(
            "web",
            "src/web",
            &[
                ("build", "vite build", &[]),
                ("test", "vitest run", &["build"]),
            ],
        );
        let doc = generate_project(&web);
        assert!(doc.contains("build:"));
        assert!(doc.contains("command: 'vite build'"));
        assert!(doc.contains("test:"));
        assert!(doc.contains("- build"));
    }

    #[test]
    fn a_task_with_no_dependency_writes_no_deps_key() {
        let web = app("web", "src/web", &[("build", "vite build", &[])]);
        let doc = generate_project(&web);
        assert!(!doc.contains("deps:"));
    }
}
