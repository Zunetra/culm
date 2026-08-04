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
        toml::from_str(source).map_err(|error| LayerError::Malformed {
            path: path.to_owned(),
            message: error.to_string(),
        })
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
requires = ["node-pnpm"]
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

        let layer = parsed.to_layer().unwrap();
        assert_eq!(layer.requires, vec![LayerId::new("node-pnpm")]);
        assert_eq!(layer.merges.len(), 2);
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
