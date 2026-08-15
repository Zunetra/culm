// === The layer manifest

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;

use super::category::Category;
use super::error::LayerError;
use crate::merge::{Contribution, Layer, LayerId};

/// One `layer.toml`, as written on disk.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub id: String,
    pub category: Category,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub default: bool,
    #[serde(default, rename = "files")]
    pub files: Vec<FileSpec>,
    #[serde(default, rename = "merge")]
    pub merges: Vec<MergeSpec>,
    #[serde(default, rename = "install")]
    pub installs: Vec<InstallSpec>,
}

/// A file copied verbatim into the generated workspace.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSpec {
    pub from: PathBuf,
    pub to: PathBuf,
    #[serde(default)]
    pub mode: Option<u32>,
}

/// A contribution to a file other layers may also own.
/// Tagged on `strategy`, so a target cannot declare one strategy and carry another's payload.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "strategy", rename_all = "kebab-case", deny_unknown_fields)]
pub enum MergeSpec {
    JsonDeep {
        target: PathBuf,
        value: toml::Value,
    },
    LineBlock {
        target: PathBuf,
        lines: Vec<String>,
    },
    TomlMembers {
        target: PathBuf,
        members: Vec<String>,
    },
}

/// Packages to install after scaffolding.
/// No versions here, ever: package managers write versions, per decision 5.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallSpec {
    pub manager: String,
    #[serde(default)]
    pub dev: bool,
    pub packages: Vec<String>,
    /* A feature is not a version. `cargo add tokio` without `--features full`
     * installs a crate that cannot run a `#[tokio::main]`, so a template that
     * cannot name features cannot be comprehensive, which decision 19 requires.
     * Applies to every package in the entry, so a set needing different
     * features is a separate entry. */
    #[serde(default)]
    pub features: Vec<String>,
}

impl Manifest {
    /// The globally unique identity of this layer, as `<category>/<id>`.
    /// Bare ids are not unique: `none` is a legitimate answer for both shape and runner, and the merge engine keys line blocks by identity, so two layers called `none` would silently share a block.
    pub fn qualified_id(&self) -> LayerId {
        LayerId::new(format!("{}/{}", self.category, self.id))
    }

    /// Parses one `layer.toml`.
    /// `path` is only used to name the file in error messages.
    pub fn parse(path: &str, source: &str) -> Result<Self, LayerError> {
        let manifest: Self = toml::from_str(source).map_err(|error| LayerError::Malformed {
            path: path.to_owned(),
            message: error.to_string(),
        })?;
        manifest.check_references()?;
        Ok(manifest)
    }

    /* A bare `node-pnpm` looks like it works and does not, because identity is
     * qualified and `none` exists in two categories. Caught at parse time, so
     * the resolver never has to guess which category a reference meant. */
    fn check_references(&self) -> Result<(), LayerError> {
        for (field, ids) in [("requires", &self.requires), ("conflicts", &self.conflicts)] {
            for id in ids {
                let qualified = id.split_once('/').is_some_and(|(category, rest)| {
                    Category::from_id(category).is_some() && !rest.is_empty()
                });
                if !qualified {
                    return Err(LayerError::UnqualifiedReference {
                        layer: self.qualified_id(),
                        field,
                        id: id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Converts to the shape the merge engine consumes.
    /// File copying, interpolation, and installs are not the merge engine's concern, so they stop here rather than being carried through it.
    pub fn to_layer(&self) -> Result<Layer, LayerError> {
        let id = self.qualified_id();
        let mut merges: BTreeMap<PathBuf, Contribution> = BTreeMap::new();

        for spec in &self.merges {
            let (target, contribution) = match spec {
                MergeSpec::JsonDeep { target, value } => {
                    let json = serde_json::to_value(value)
                        .map_err(|_| LayerError::NonTableMergeValue { layer: id.clone() })?;
                    if !json.is_object() {
                        return Err(LayerError::NonTableMergeValue { layer: id.clone() });
                    }
                    (target.clone(), Contribution::JsonDeep(json))
                }
                MergeSpec::LineBlock { target, lines } => {
                    (target.clone(), Contribution::LineBlock(lines.clone()))
                }
                MergeSpec::TomlMembers { target, members } => {
                    (target.clone(), Contribution::TomlMembers(members.clone()))
                }
            };
            merges.insert(target, contribution);
        }

        Ok(Layer {
            id,
            requires: self.requires.iter().map(LayerId::new).collect(),
            conflicts: self.conflicts.iter().map(LayerId::new).collect(),
            merges,
        })
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_full_manifest() {
        let source = r#"
id = "nuxt"
category = "frontend"
requires = ["toolchain/node-pnpm"]
default = false

[[files]]
from = "files/nuxt.config.ts"
to = "nuxt.config.ts"

[[merge]]
strategy = "json-deep"
target = "package.json"
[merge.value]
scripts = { dev = "nuxt dev" }

[[merge]]
strategy = "line-block"
target = ".gitignore"
lines = [".nuxt/", ".output/"]

[[install]]
manager = "pnpm"
dev = true
packages = ["nuxt"]
"#;
        let parsed = Manifest::parse("nuxt/layer.toml", source).unwrap();
        assert_eq!(parsed.category, Category::Frontend);
        assert_eq!(parsed.merges.len(), 2);
        assert_eq!(parsed.installs[0].packages, vec!["nuxt"]);
        assert!(parsed.installs[0].features.is_empty());

        let layer = parsed.to_layer().unwrap();
        assert_eq!(layer.requires, vec![LayerId::new("toolchain/node-pnpm")]);
        assert_eq!(layer.merges.len(), 2);
    }

    /* Bare ids are not unique, so one that slipped into a manifest would
     * resolve to whichever category happened to be looked at first. */
    #[test]
    fn an_unqualified_reference_is_rejected_at_parse_time() {
        let source = "id = \"nuxt\"\ncategory = \"frontend\"\nrequires = [\"node-pnpm\"]\n";
        assert_eq!(
            Manifest::parse("nuxt/layer.toml", source),
            Err(LayerError::UnqualifiedReference {
                layer: LayerId::new("frontend/nuxt"),
                field: "requires",
                id: "node-pnpm".to_owned(),
            })
        );
    }

    #[test]
    fn a_reference_to_a_category_that_does_not_exist_is_rejected() {
        let source = "id = \"nuxt\"\ncategory = \"frontend\"\nconflicts = [\"platform/deno\"]\n";
        assert!(matches!(
            Manifest::parse("nuxt/layer.toml", source),
            Err(LayerError::UnqualifiedReference { .. })
        ));
    }

    #[test]
    fn a_merge_entry_cannot_mix_strategies() {
        let source = r#"
id = "broken"
category = "quality"

[[merge]]
strategy = "line-block"
target = ".gitignore"
[merge.value]
scripts = { dev = "vite" }
"#;
        assert!(Manifest::parse("broken/layer.toml", source).is_err());
    }

    #[test]
    fn unknown_fields_are_rejected_rather_than_silently_ignored() {
        let source = "id = \"typo\"\ncategory = \"quality\"\ndefualt = true\n";
        assert!(Manifest::parse("typo/layer.toml", source).is_err());
    }
}
