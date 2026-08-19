// === Installing dependencies

/* Decision 5 is LOCKED: no template file names a version. That is only
 * affordable because the package manager writes them, which means the scaffold
 * is not finished until the installs have run. A workspace whose manifests have
 * no dependencies in them does not build, so this module is the difference
 * between a demo and a tool. */

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use crate::layer::{CatalogEntry, InstallSpec};
use crate::merge::LayerId;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InstallError {
    #[error("layer `{layer}` asks for package manager `{manager}`, which culm cannot run")]
    UnknownManager { layer: LayerId, manager: String },

    #[error("layer `{layer}` gives features to a `{manager}` install, which has no such concept")]
    FeaturesUnsupported { layer: LayerId, manager: String },

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
}

/// One package manager invocation, held as argv rather than as a string.
/// A shell string would have to be quoted to be correct and parsed to be tested, and neither is worth doing when the only consumer spawns a process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCommand {
    pub program: String,
    pub args: Vec<String>,
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
pub fn plan(entries: &[&CatalogEntry]) -> Result<Vec<InstallCommand>, InstallError> {
    let mut groups: Vec<Group> = Vec::new();

    for entry in entries {
        let layer = entry.manifest.qualified_id();
        let manifest = cargo_manifest(entry);

        for spec in &entry.manifest.installs {
            check(&layer, spec)?;

            let manifest = match spec.manager.as_str() {
                "cargo" => manifest.clone(),
                _ => None,
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
        _ => Err(InstallError::UnknownManager {
            layer: layer.clone(),
            manager: spec.manager.clone(),
        }),
    }
}

fn command(group: &Group) -> InstallCommand {
    let mut args = vec![String::from("add")];

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
                merges: Vec::new(),
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
        }
    }

    fn lines(commands: &[InstallCommand]) -> Vec<String> {
        commands.iter().map(InstallCommand::display).collect()
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
