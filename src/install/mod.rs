// === Installing dependencies

/* Decision 5 is LOCKED: no template file names a version. That is only
 * affordable because the package manager writes them, which means the scaffold
 * is not finished until the installs have run. A workspace whose manifests have
 * no dependencies in them does not build, so this module is the difference
 * between a demo and a tool. */

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use crate::layer::{CatalogEntry, InstallSpec, MergeSpec};
use crate::merge::LayerId;

pub mod crates_io;
pub mod go_proxy;
pub mod maven_central;
pub mod release_age;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InstallError {
    #[error("layer `{layer}` asks for package manager `{manager}`, which culm cannot run")]
    UnknownManager { layer: LayerId, manager: String },

    #[error("layer `{layer}` gives features to a `{manager}` install, which has no such concept")]
    FeaturesUnsupported { layer: LayerId, manager: String },

    #[error("layer `{layer}` marks a `{manager}` install as dev-only, which has no such concept")]
    DevUnsupported { layer: LayerId, manager: String },

    #[error("`{command}` could not be started: {message}")]
    NotStarted { command: String, message: String },

    #[error(
        "`{command}` failed{}\n{stderr}\nThe workspace is still on disk. Fix the problem and run the command yourself in that directory.",
        .status.map(|code| format!(" with exit code {code}")).unwrap_or_default()
    )]
    Failed {
        command: String,
        status: Option<i32>,
        stderr: String,
    },

    #[error("`{}` has no `package.json` to pin a package manager into", .root.display())]
    NoRootPackageJson { root: PathBuf },

    #[error("`{}` is not valid JSON: {message}", .path.display())]
    MalformedPackageJson { path: PathBuf, message: String },
}

/// One package manager invocation, held as argv rather than as a string.
/// A shell string would have to be quoted to be correct and parsed to be tested, and neither is worth doing when the only consumer spawns a process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCommand {
    pub program: String,
    pub args: Vec<String>,
    /// How many of `args`, counted from the end, are package specs. The
    /// release age guard rewrites exactly those.
    pub packages: usize,
}

impl InstallCommand {
    /// The command as a user would type it, for the `--no-install` plan and for error messages.
    pub fn display(&self) -> String {
        let mut line = String::from(&self.program);
        for arg in &self.args {
            line.push(' ');
            line.push_str(arg);
        }
        line
    }
}

/* What separates two commands. Manager and dev are the obvious ones; features
 * are per entry because `--features` applies to every package on the line, and
 * the manifest path is per entry because a Rust workspace root is virtual and
 * `cargo add` there has no package to add to. */
#[derive(Debug, PartialEq, Eq)]
struct Group {
    manager: String,
    dev: bool,
    features: Vec<String>,
    manifest: Option<PathBuf>,
    packages: Vec<String>,
}

/// Turns a resolved selection into the commands that make its workspace build.
///
/// Entries keep the selection's order, and packages keep the order their layer
/// wrote them in, so the same selection always yields the same argv. That is
/// what makes the plan assertable in a test.
///
/// A workspace-root install always runs before a per-app one, regardless of
/// which layer's turn produced it first. A root devDependency can carry a
/// lifecycle script (`quality/husky`'s `prepare`, say), and once a real
/// `pnpm-workspace.yaml` exists (decision 23's `{{workspace_glob}}`), pnpm
/// runs the root's lifecycle scripts on *any* `pnpm add` in the workspace,
/// including a per-app one. An app layer commonly comes before the quality
/// layers that add the root's own devDependencies in the selection order, so
/// without this the very first per-app install could try to run a root
/// script that has not been installed yet and fail outright.
pub fn plan(entries: &[&CatalogEntry]) -> Result<Vec<InstallCommand>, InstallError> {
    let mut groups: Vec<Group> = Vec::new();

    for entry in entries {
        let layer = entry.manifest.qualified_id();
        let manifest = cargo_manifest(entry);

        for spec in &entry.manifest.installs {
            check(&layer, spec)?;

            let manifest = if spec.root {
                None
            } else {
                match spec.manager.as_str() {
                    "cargo" => manifest.clone(),
                    "go" => go_module_dir(entry),
                    "pnpm" => pnpm_package_dir(entry),
                    _ => None,
                }
            };

            let existing = groups.iter_mut().find(|group| {
                group.manager == spec.manager
                    && group.dev == spec.dev
                    && group.features == spec.features
                    && group.manifest == manifest
            });

            let group = match existing {
                Some(group) => group,
                None => {
                    groups.push(Group {
                        manager: spec.manager.clone(),
                        dev: spec.dev,
                        features: spec.features.clone(),
                        manifest,
                        packages: Vec::new(),
                    });
                    groups.last_mut().expect("just pushed")
                }
            };

            /* Two layers may both want the same package. Installing it twice is
             * harmless but noisy, and it makes the plan harder to read than the
             * selection that produced it. */
            for package in &spec.packages {
                if !group.packages.contains(package) {
                    group.packages.push(package.clone());
                }
            }
        }
    }

    /* Stable, so within "root" and within "per app" the original first-seen
     * order (and therefore the existing tests asserting it) is untouched;
     * only the two partitions swap relative to each other. */
    groups.sort_by_key(|group| group.manifest.is_some());

    Ok(groups.iter().map(command).collect())
}

/// Runs the plan in the generated workspace, stopping at the first failure.
///
/// Nothing is rolled back. The files on disk are what the user asked for and
/// are still useful without their dependencies, whereas deleting them would
/// throw away the part that worked because the part that needs a network did
/// not.
pub fn run(commands: &[InstallCommand], root: &Path) -> Result<(), InstallError> {
    for command in commands {
        let output = Command::new(&command.program)
            .args(&command.args)
            .current_dir(root)
            .output()
            .map_err(|error| InstallError::NotStarted {
                command: command.display(),
                message: error.to_string(),
            })?;

        if !output.status.success() {
            return Err(InstallError::Failed {
                command: command.display(),
                status: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            });
        }
    }
    Ok(())
}

// === Package manager pinning

/* The one narrow exception to decision 5. Turborepo refuses to run at all
 * without a `packageManager` field in the root `package.json`: it reads that
 * field to decide which package manager's lockfile and workspace layout to
 * trust, and a missing field is a hard error rather than a fallback. No
 * layer template can write that value, because decision 5 forbids a version
 * number in a layer file and this is nothing but a version number. So it is
 * resolved from the pnpm actually on this machine, after scaffolding, and
 * patched into the file the scaffold already wrote. */

/// Asks the pnpm on `PATH` for its own version and returns it as the
/// `packageManager` field's value (`pnpm@<version>`), the exact string
/// corepack and Turborepo expect.
pub fn pin_pnpm_version() -> Result<String, InstallError> {
    let output = Command::new("pnpm")
        .arg("--version")
        .output()
        .map_err(|error| InstallError::NotStarted {
            command: "pnpm --version".to_owned(),
            message: error.to_string(),
        })?;

    if !output.status.success() {
        return Err(InstallError::Failed {
            command: "pnpm --version".to_owned(),
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok(format!("pnpm@{version}"))
}

/// Patches `root/package.json`'s `packageManager` field in place with
/// whatever `pin_pnpm_version` resolves. The rest of the file is left
/// untouched, so this must run after the scaffold has already written the
/// merged `package.json`, never before.
pub fn apply_package_manager_pin(root: &Path) -> Result<(), InstallError> {
    let path = root.join("package.json");
    if !path.exists() {
        return Err(InstallError::NoRootPackageJson {
            root: root.to_owned(),
        });
    }

    let raw = std::fs::read_to_string(&path).map_err(|error| InstallError::NotStarted {
        command: format!("read {}", path.display()),
        message: error.to_string(),
    })?;

    let mut doc: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| InstallError::MalformedPackageJson {
            path: path.clone(),
            message: error.to_string(),
        })?;

    let pinned = pin_pnpm_version()?;
    doc["packageManager"] = serde_json::Value::String(pinned);

    let rendered = serde_json::to_string_pretty(&doc).map_err(|error| {
        InstallError::MalformedPackageJson {
            path: path.clone(),
            message: error.to_string(),
        }
    })?;
    std::fs::write(&path, rendered + "\n").map_err(|error| InstallError::NotStarted {
        command: format!("write {}", path.display()),
        message: error.to_string(),
    })?;

    Ok(())
}

/// Where a cargo install has to be pointed.
///
/// A layer that adds a crate ships that crate's `Cargo.toml`, and the root
/// manifest the layers assemble is a `[workspace]` with no `[package]` in it.
/// `cargo add` run there has nothing to add to, so the layer's own manifest is
/// the target. Deriving it from the layer's files rather than from a new
/// manifest key keeps the layer format from growing a field whose only correct
/// value is one the layer already stated.
fn cargo_manifest(entry: &CatalogEntry) -> Option<PathBuf> {
    entry
        .manifest
        .files
        .iter()
        .find(|file| file.to.file_name().is_some_and(|name| name == "Cargo.toml"))
        .map(|file| file.to.clone())
}

/// Where a `go get` has to be pointed.
///
/// Go modules are directory scoped, and `go get` run from the workspace root
/// has no module to add to unless the app happens to sit there. `go`'s own
/// `-C <dir>` flag changes directory before the subcommand runs, which is
/// the same move `cargo add --manifest-path` makes for a virtual workspace
/// root: point the command at the file the layer already shipped, rather
/// than growing the layer format a field whose only correct value is one it
/// already stated.
fn go_module_dir(entry: &CatalogEntry) -> Option<PathBuf> {
    entry
        .manifest
        .files
        .iter()
        .find(|file| file.to.file_name().is_some_and(|name| name == "go.mod"))
        .and_then(|file| file.to.parent().map(Path::to_path_buf))
}

/// Where a `pnpm add` has to be pointed. `None` when the layer's own
/// package.json is the workspace root one, matching how `cargo_manifest`
/// and `go_module_dir` both return `None` for the same case.
fn pnpm_package_dir(entry: &CatalogEntry) -> Option<PathBuf> {
    entry.manifest.merges.iter().find_map(|merge| match merge {
        MergeSpec::JsonDeep { target, .. }
            if target.file_name().is_some_and(|name| name == "package.json") =>
        {
            target
                .parent()
                .filter(|dir| !dir.as_os_str().is_empty())
                .map(Path::to_path_buf)
        }
        _ => None,
    })
}

fn check(layer: &LayerId, spec: &InstallSpec) -> Result<(), InstallError> {
    match spec.manager.as_str() {
        "cargo" => Ok(()),
        "pnpm" => {
            if spec.features.is_empty() {
                Ok(())
            } else {
                Err(InstallError::FeaturesUnsupported {
                    layer: layer.clone(),
                    manager: spec.manager.clone(),
                })
            }
        }
        "go" => {
            if !spec.features.is_empty() {
                return Err(InstallError::FeaturesUnsupported {
                    layer: layer.clone(),
                    manager: spec.manager.clone(),
                });
            }
            if spec.dev {
                return Err(InstallError::DevUnsupported {
                    layer: layer.clone(),
                    manager: spec.manager.clone(),
                });
            }
            Ok(())
        }
        _ => Err(InstallError::UnknownManager {
            layer: layer.clone(),
            manager: spec.manager.clone(),
        }),
    }
}

fn command(group: &Group) -> InstallCommand {
    // `go get` is not spelled `go add`, and its `-C` is a global flag that
    // has to precede the subcommand, so it cannot share the `add`-first
    // shape every other manager here takes.
    if group.manager == "go" {
        let mut args = Vec::new();
        if let Some(dir) = &group.manifest {
            args.push(String::from("-C"));
            args.push(dir.display().to_string());
        }
        args.push(String::from("get"));
        args.extend(group.packages.iter().cloned());
        return InstallCommand {
            program: group.manager.clone(),
            args,
            packages: group.packages.len(),
        };
    }

    let mut args = Vec::new();
    if group.manager == "pnpm"
        && let Some(dir) = &group.manifest
    {
        args.push(String::from("-C"));
        args.push(dir.display().to_string());
    }
    args.push(String::from("add"));

    match group.manager.as_str() {
        "cargo" => {
            if group.dev {
                args.push(String::from("--dev"));
            }
            if let Some(manifest) = &group.manifest {
                args.push(String::from("--manifest-path"));
                args.push(manifest.display().to_string());
            }
            if !group.features.is_empty() {
                args.push(String::from("--features"));
                args.push(group.features.join(","));
            }
        }
        _ => {
            if group.dev {
                args.push(String::from("-D"));
            }
        }
    }

    args.extend(group.packages.iter().cloned());

    InstallCommand {
        program: group.manager.clone(),
        args,
        packages: group.packages.len(),
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{Category, FileSpec, LayerSource, Manifest};

    fn entry(
        id: &str,
        category: Category,
        files: Vec<&str>,
        installs: Vec<InstallSpec>,
    ) -> CatalogEntry {
        entry_with_pkg_target(id, category, files, installs, None)
    }

    /// Like `entry`, but for a layer that owns its own `package.json` (a real
    /// per-app manifest, decision 23), so its `pnpm` installs group by that
    /// directory instead of the workspace root.
    fn entry_with_pkg_target(
        id: &str,
        category: Category,
        files: Vec<&str>,
        installs: Vec<InstallSpec>,
        pkg_target: Option<&str>,
    ) -> CatalogEntry {
        let merges = match pkg_target {
            Some(target) => vec![MergeSpec::JsonDeep {
                target: PathBuf::from(target),
                value: toml::Value::Table(Default::default()),
            }],
            None => Vec::new(),
        };
        CatalogEntry {
            manifest: Manifest {
                id: id.to_owned(),
                category,
                requires: Vec::new(),
                conflicts: Vec::new(),
                default: false,
                files: files
                    .into_iter()
                    .map(|to| FileSpec {
                        from: PathBuf::from("files/x"),
                        to: PathBuf::from(to),
                        mode: None,
                    })
                    .collect(),
                merges,
                installs,
            },
            source: LayerSource::Embedded(PathBuf::from(format!("{category}/{id}"))),
        }
    }

    fn spec(manager: &str, dev: bool, packages: &[&str], features: &[&str]) -> InstallSpec {
        InstallSpec {
            manager: manager.to_owned(),
            dev,
            packages: packages.iter().map(|p| (*p).to_owned()).collect(),
            features: features.iter().map(|f| (*f).to_owned()).collect(),
            root: false,
        }
    }

    fn root_spec(manager: &str, dev: bool, packages: &[&str]) -> InstallSpec {
        InstallSpec {
            manager: manager.to_owned(),
            dev,
            packages: packages.iter().map(|p| (*p).to_owned()).collect(),
            features: Vec::new(),
            root: true,
        }
    }

    fn lines(commands: &[InstallCommand]) -> Vec<String> {
        commands.iter().map(InstallCommand::display).collect()
    }

    /* The exact scenario a real `culm new` selection hits: a per-app layer
     * that owns its own package.json (decision 23) is chosen before the
     * quality layers that add root devDependencies, but a root install with a
     * lifecycle script (husky's `prepare`) has to run first once a real
     * `pnpm-workspace.yaml` exists, or the very first per-app `pnpm add`
     * triggers that script before it is installed. */
    #[test]
    fn a_root_install_runs_before_a_per_app_one_regardless_of_selection_order() {
        let react_vite = entry_with_pkg_target(
            "react-vite",
            Category::Frontend,
            vec![],
            vec![spec("pnpm", false, &["react"], &[])],
            Some("src/react-vite/package.json"),
        );
        let husky = entry(
            "husky",
            Category::Quality,
            vec![],
            vec![root_spec("pnpm", true, &["husky"])],
        );

        assert_eq!(
            lines(&plan(&[&react_vite, &husky]).unwrap()),
            vec!["pnpm add -D husky", "pnpm -C src/react-vite add react",]
        );
    }

    #[test]
    fn a_node_selection_becomes_one_command_per_dev_flag() {
        let nuxt = entry(
            "nuxt",
            Category::Frontend,
            vec![],
            vec![
                spec("pnpm", false, &["vue", "vue-router"], &[]),
                spec("pnpm", true, &["nuxt", "typescript"], &[]),
            ],
        );
        let express = entry(
            "express",
            Category::Backend,
            vec![],
            vec![spec("pnpm", false, &["express"], &[])],
        );

        assert_eq!(
            lines(&plan(&[&nuxt, &express]).unwrap()),
            vec![
                "pnpm add vue vue-router express",
                "pnpm add -D nuxt typescript",
            ]
        );
    }

    /* The root manifest is a virtual workspace, so an unqualified `cargo add`
     * there has no package to write into. */
    #[test]
    fn a_cargo_install_is_pointed_at_the_crate_its_layer_ships() {
        let axum = entry(
            "axum",
            Category::Backend,
            vec!["crates/axum/Cargo.toml", "crates/axum/src/main.rs"],
            vec![
                spec("cargo", false, &["axum", "serde_json"], &[]),
                spec("cargo", false, &["tokio"], &["full"]),
                spec("cargo", true, &["tower"], &[]),
            ],
        );

        assert_eq!(
            lines(&plan(&[&axum]).unwrap()),
            vec![
                "cargo add --manifest-path crates/axum/Cargo.toml axum serde_json",
                "cargo add --manifest-path crates/axum/Cargo.toml --features full tokio",
                "cargo add --dev --manifest-path crates/axum/Cargo.toml tower",
            ]
        );
    }

    /* Two crates in one workspace each get their own dependencies, which is the
     * whole reason the manifest path is part of the grouping key. */
    #[test]
    fn two_rust_layers_do_not_share_a_command() {
        let axum = entry(
            "axum",
            Category::Backend,
            vec!["crates/axum/Cargo.toml"],
            vec![spec("cargo", false, &["axum"], &[])],
        );
        let sqlx = entry(
            "sqlx",
            Category::Data,
            vec!["crates/sqlx/Cargo.toml"],
            vec![spec("cargo", false, &["sqlx"], &[])],
        );

        assert_eq!(
            lines(&plan(&[&axum, &sqlx]).unwrap()),
            vec![
                "cargo add --manifest-path crates/axum/Cargo.toml axum",
                "cargo add --manifest-path crates/sqlx/Cargo.toml sqlx",
            ]
        );
    }

    #[test]
    fn a_package_two_layers_both_want_is_installed_once() {
        let one = entry(
            "eslint",
            Category::Quality,
            vec![],
            vec![spec("pnpm", true, &["eslint"], &[])],
        );
        let two = entry(
            "prettier",
            Category::Quality,
            vec![],
            vec![spec("pnpm", true, &["eslint", "prettier"], &[])],
        );

        assert_eq!(
            lines(&plan(&[&one, &two]).unwrap()),
            vec!["pnpm add -D eslint prettier"]
        );
    }

    /* Go modules are directory scoped, the same reason a Rust workspace root
     * needs `--manifest-path`, but `go get` spells its equivalent as a
     * leading `-C <dir>` rather than a trailing flag. */
    #[test]
    fn a_go_install_is_pointed_at_the_module_its_layer_ships() {
        let go_chi = entry(
            "go-chi",
            Category::Backend,
            vec!["src/go-chi/go.mod", "src/go-chi/main.go"],
            vec![spec("go", false, &["github.com/go-chi/chi/v5"], &[])],
        );

        assert_eq!(
            lines(&plan(&[&go_chi]).unwrap()),
            vec!["go -C src/go-chi get github.com/go-chi/chi/v5"]
        );
    }

    #[test]
    fn features_on_a_go_install_are_refused() {
        let odd = entry(
            "odd",
            Category::Backend,
            vec![],
            vec![spec("go", false, &["x"], &["full"])],
        );

        assert!(matches!(
            plan(&[&odd]).unwrap_err(),
            InstallError::FeaturesUnsupported { .. }
        ));
    }

    #[test]
    fn dev_on_a_go_install_is_refused() {
        let odd = entry(
            "odd",
            Category::Backend,
            vec![],
            vec![spec("go", true, &["x"], &[])],
        );

        assert!(matches!(
            plan(&[&odd]).unwrap_err(),
            InstallError::DevUnsupported { .. }
        ));
    }

    #[test]
    fn a_selection_with_no_installs_is_an_empty_plan() {
        let none = entry("none", Category::Runner, vec![], vec![]);
        assert!(plan(&[&none]).unwrap().is_empty());
    }

    #[test]
    fn an_unrunnable_manager_is_refused_by_name() {
        let odd = entry(
            "odd",
            Category::Quality,
            vec![],
            vec![spec("yarn", false, &["left-pad"], &[])],
        );

        let error = plan(&[&odd]).unwrap_err();
        assert!(error.to_string().contains("yarn"), "{error}");
        assert!(error.to_string().contains("quality/odd"), "{error}");
    }

    #[test]
    fn features_on_a_node_install_are_refused() {
        let odd = entry(
            "odd",
            Category::Quality,
            vec![],
            vec![spec("pnpm", false, &["vue"], &["full"])],
        );

        assert!(matches!(
            plan(&[&odd]).unwrap_err(),
            InstallError::FeaturesUnsupported { .. }
        ));
    }
}
