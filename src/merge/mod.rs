// === Merge

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

pub mod hook_script;
pub mod json_deep;
pub mod line_block;
pub mod toml_members;
pub mod yaml_list;
pub mod yaml_steps;

pub use hook_script::{HOOK_MODE, HookScriptDoc};
pub use json_deep::ValueKind;
pub use toml_members::TomlMembersDoc;
pub use yaml_list::{YamlListDoc, YamlScalar};
pub use yaml_steps::{Step, YamlStepsDoc};

use json_deep::JsonDoc;
use line_block::LineBlockDoc;

// == Layer identity

/// Identifies the layer a contribution came from.
/// Ordered, because rendered block order is defined as sorted by layer id, and that is half of what makes output order independent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerId(String);

impl LayerId {
    /// Wraps a string as a layer id.
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// Borrows the underlying id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LayerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for LayerId {
    fn from(id: &str) -> Self {
        Self(id.to_owned())
    }
}

impl From<String> for LayerId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

// == Strategies

/// The merge strategy a target file is accumulated under.
/// A target is claimed by exactly one, and a second layer claiming it under another is an error rather than a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Strategy {
    JsonDeep,
    LineBlock,
    TomlMembers,
    YamlList,
    HookScript,
    YamlSteps,
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::JsonDeep => "json-deep",
            Self::LineBlock => "line-block",
            Self::TomlMembers => "toml-members",
            Self::YamlList => "yaml-list",
            Self::HookScript => "hook-script",
            Self::YamlSteps => "yaml-steps",
        })
    }
}

// == Errors

/// Every failure the merge engine can produce.
/// Each conflict variant names both culprit layers, because "conflict in package.json" without the culprits is useless to whoever hits it.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MergeError {
    /// Two layers set the same scalar to different values.
    #[error(
        "scalar conflict in {target}: layers '{a}' and '{b}' both set '{key}' to different values"
    )]
    ScalarConflict {
        target: PathBuf,
        key: String,
        a: LayerId,
        b: LayerId,
    },

    /// Two layers put incompatible kinds of value at the same path, such as an object against a string.
    #[error(
        "type conflict in {target}: layer '{a}' set '{key}' to {a_type}, layer '{b}' set it to {b_type}"
    )]
    TypeConflict {
        target: PathBuf,
        key: String,
        a: LayerId,
        b: LayerId,
        a_type: ValueKind,
        b_type: ValueKind,
    },

    /// Two layers claim the same target file under different merge strategies.
    /// One of the two manifests is wrong, and guessing which would be a silent resolution.
    #[error(
        "strategy conflict in {target}: layer '{a}' merges it as {a_strategy}, layer '{b}' as {b_strategy}"
    )]
    StrategyMismatch {
        target: PathBuf,
        a: LayerId,
        b: LayerId,
        a_strategy: Strategy,
        b_strategy: Strategy,
    },

    /// A json-deep contribution whose root is not an object.
    /// There is no dotted key path to report a conflict against below such a root, so it is rejected at the door.
    #[error("layer '{layer}' contributes a non-object root to {target} under json-deep")]
    NonObjectRoot { target: PathBuf, layer: LayerId },

    /// A selected layer needs another layer that was not selected.
    #[error("layer '{layer}' requires layer '{requires}', which is not selected")]
    MissingRequirement { layer: LayerId, requires: LayerId },

    /// Two selected layers declare each other mutually exclusive.
    #[error("layers '{a}' and '{b}' conflict and cannot be selected together")]
    Conflicting { a: LayerId, b: LayerId },
}

// == Contributions

/// A single layer's fragment for one shared file.
/// The variant chosen is the merge strategy, so a target cannot be merged two different ways by accident.
#[derive(Debug, Clone, PartialEq)]
pub enum Contribution {
    /// Objects merge recursively, arrays union then sort then dedupe, differing scalars are an error.
    JsonDeep(serde_json::Value),
    /// One `# layer:<id>` delimited block per layer, blocks sorted by layer id.
    LineBlock(Vec<String>),
    /// Crates appended to a Cargo workspace's `members`, unioned then sorted.
    TomlMembers(Vec<String>),
    /// Named YAML sequences, each unioned then sorted, plus single settings. The workspace globs and settings in `pnpm-workspace.yaml`.
    YamlList {
        lists: BTreeMap<String, Vec<String>>,
        scalars: BTreeMap<String, YamlScalar>,
    },
    /// A shell hook body placed at a declared order. Sequence is observable here, so it is declared rather than inferred.
    HookScript { order: u32, body: Vec<String> },
    /// CI steps placed in a named job at a declared order.
    YamlSteps {
        job: String,
        order: u32,
        steps: Vec<Step>,
    },
}

impl Contribution {
    /// The strategy this contribution merges under.
    pub fn strategy(&self) -> Strategy {
        match self {
            Self::JsonDeep(_) => Strategy::JsonDeep,
            Self::LineBlock(_) => Strategy::LineBlock,
            Self::TomlMembers(_) => Strategy::TomlMembers,
            Self::YamlList { .. } => Strategy::YamlList,
            Self::HookScript { .. } => Strategy::HookScript,
            Self::YamlSteps { .. } => Strategy::YamlSteps,
        }
    }
}

/// A layer as the merge engine sees it.
/// File copying and template interpolation are somebody else's problem; this is only what a layer merges.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub id: LayerId,
    pub requires: Vec<LayerId>,
    pub conflicts: Vec<LayerId>,
    pub merges: BTreeMap<PathBuf, Contribution>,
}

impl Layer {
    /// Builds a layer with no requirements and no conflicts.
    pub fn new<I: Into<LayerId>>(id: I) -> Self {
        Self {
            id: id.into(),
            requires: Vec::new(),
            conflicts: Vec::new(),
            merges: BTreeMap::new(),
        }
    }

    /// Adds a contribution for one target file.
    pub fn with_merge<P: Into<PathBuf>>(mut self, target: P, contribution: Contribution) -> Self {
        self.merges.insert(target.into(), contribution);
        self
    }

    /// Declares a layer this one cannot be applied without.
    pub fn requiring<I: Into<LayerId>>(mut self, requires: I) -> Self {
        self.requires.push(requires.into());
        self
    }

    /// Declares a layer this one is mutually exclusive with.
    pub fn conflicting<I: Into<LayerId>>(mut self, conflicts: I) -> Self {
        self.conflicts.push(conflicts.into());
        self
    }
}

// == The virtual file set

/// The accumulated content of one shared file.
#[derive(Debug, Clone, PartialEq)]
pub enum FileContent {
    JsonDeep(JsonDoc),
    LineBlock(LineBlockDoc),
    TomlMembers(TomlMembersDoc),
    YamlList(YamlListDoc),
    HookScript(HookScriptDoc),
    YamlSteps(YamlStepsDoc),
}

impl FileContent {
    /// The strategy this file is accumulated under.
    pub fn strategy(&self) -> Strategy {
        match self {
            Self::JsonDeep(_) => Strategy::JsonDeep,
            Self::LineBlock(_) => Strategy::LineBlock,
            Self::TomlMembers(_) => Strategy::TomlMembers,
            Self::YamlList(_) => Strategy::YamlList,
            Self::HookScript(_) => Strategy::HookScript,
            Self::YamlSteps(_) => Strategy::YamlSteps,
        }
    }

    /// The mode this file must be written with, if it needs one.
    /// Only hooks do: a git hook without the executable bit is skipped in silence, so the bit travels with the content rather than being a layer's responsibility to remember.
    pub fn mode(&self) -> Option<u32> {
        match self {
            Self::HookScript(doc) => Some(doc.mode()),
            _ => None,
        }
    }

    /// Renders this file to its final bytes.
    pub fn render(&self) -> String {
        match self {
            Self::JsonDeep(doc) => doc.render(),
            Self::LineBlock(doc) => doc.render(),
            Self::TomlMembers(doc) => doc.render(),
            Self::YamlList(doc) => doc.render(),
            Self::HookScript(doc) => doc.render(),
            Self::YamlSteps(doc) => doc.render(),
        }
    }

    /// Names the layer that first claimed this file.
    pub fn first_owner(&self) -> LayerId {
        match self {
            Self::JsonDeep(doc) => doc.first_owner(),
            Self::LineBlock(doc) => doc.first_owner(),
            Self::TomlMembers(doc) => doc.first_owner(),
            Self::YamlList(doc) => doc.first_owner(),
            Self::HookScript(doc) => doc.first_owner(),
            Self::YamlSteps(doc) => doc.first_owner(),
        }
    }
}

/// The virtual file set layers are applied to.
/// Nothing reaches disk until the whole run has validated, because a partial scaffold is worse than a failed one: a failed one leaves nothing to clean up.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileSet {
    /* BTreeMap, not HashMap: sorted iteration is what makes byte-comparison testing possible. */
    files: BTreeMap<PathBuf, FileContent>,
}

impl FileSet {
    /// An empty file set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates the selection, then applies every layer's contributions.
    /// The result is byte-identical for any permutation of `layers`.
    pub fn build<'a, I: IntoIterator<Item = &'a Layer>>(layers: I) -> Result<Self, MergeError> {
        let layers: Vec<&Layer> = layers.into_iter().collect();
        validate_selection(&layers)?;

        let mut set = Self::new();
        for layer in layers {
            set.apply(layer)?;
        }
        Ok(set)
    }

    /// Applies one layer's contributions without re-checking requirements.
    pub fn apply(&mut self, layer: &Layer) -> Result<(), MergeError> {
        for (target, contribution) in &layer.merges {
            self.apply_contribution(target, &layer.id, contribution)?;
        }
        Ok(())
    }

    fn apply_contribution(
        &mut self,
        target: &Path,
        layer: &LayerId,
        contribution: &Contribution,
    ) -> Result<(), MergeError> {
        match self.files.get_mut(target) {
            None => {
                let content = match contribution {
                    Contribution::JsonDeep(value) => {
                        FileContent::JsonDeep(JsonDoc::seed(target, layer, value)?)
                    }
                    Contribution::LineBlock(lines) => {
                        FileContent::LineBlock(LineBlockDoc::seed(layer, lines))
                    }
                    Contribution::TomlMembers(members) => {
                        FileContent::TomlMembers(TomlMembersDoc::seed(layer, members))
                    }
                    Contribution::YamlList { lists, scalars } => {
                        FileContent::YamlList(YamlListDoc::seed(layer, lists, scalars))
                    }
                    Contribution::HookScript { order, body } => {
                        FileContent::HookScript(HookScriptDoc::seed(layer, *order, body))
                    }
                    Contribution::YamlSteps { job, order, steps } => FileContent::YamlSteps(
                        YamlStepsDoc::seed(target, layer, job, *order, steps),
                    ),
                };
                self.files.insert(target.to_path_buf(), content);
                Ok(())
            }
            Some(FileContent::JsonDeep(doc)) => match contribution {
                Contribution::JsonDeep(value) => doc.merge(target, layer, value),
                other => Err(MergeError::StrategyMismatch {
                    target: target.to_path_buf(),
                    a: doc.first_owner(),
                    b: layer.clone(),
                    a_strategy: Strategy::JsonDeep,
                    b_strategy: other.strategy(),
                }),
            },
            Some(FileContent::LineBlock(doc)) => match contribution {
                Contribution::LineBlock(lines) => {
                    doc.add(layer, lines);
                    Ok(())
                }
                other => Err(MergeError::StrategyMismatch {
                    target: target.to_path_buf(),
                    a: doc.first_owner(),
                    b: layer.clone(),
                    a_strategy: Strategy::LineBlock,
                    b_strategy: other.strategy(),
                }),
            },
            Some(FileContent::TomlMembers(doc)) => match contribution {
                Contribution::TomlMembers(members) => {
                    doc.add(members);
                    Ok(())
                }
                other => Err(MergeError::StrategyMismatch {
                    target: target.to_path_buf(),
                    a: doc.first_owner(),
                    b: layer.clone(),
                    a_strategy: Strategy::TomlMembers,
                    b_strategy: other.strategy(),
                }),
            },
            Some(FileContent::YamlList(doc)) => match contribution {
                Contribution::YamlList { lists, scalars } => doc.add(target, layer, lists, scalars),
                other => Err(MergeError::StrategyMismatch {
                    target: target.to_path_buf(),
                    a: doc.first_owner(),
                    b: layer.clone(),
                    a_strategy: Strategy::YamlList,
                    b_strategy: other.strategy(),
                }),
            },
            Some(FileContent::HookScript(doc)) => match contribution {
                Contribution::HookScript { order, body } => {
                    doc.add(layer, *order, body);
                    Ok(())
                }
                other => Err(MergeError::StrategyMismatch {
                    target: target.to_path_buf(),
                    a: doc.first_owner(),
                    b: layer.clone(),
                    a_strategy: Strategy::HookScript,
                    b_strategy: other.strategy(),
                }),
            },
            Some(FileContent::YamlSteps(doc)) => match contribution {
                Contribution::YamlSteps { job, order, steps } => {
                    doc.add(layer, job, *order, steps);
                    Ok(())
                }
                other => Err(MergeError::StrategyMismatch {
                    target: target.to_path_buf(),
                    a: doc.first_owner(),
                    b: layer.clone(),
                    a_strategy: Strategy::YamlSteps,
                    b_strategy: other.strategy(),
                }),
            },
        }
    }

    /// Borrows the accumulated content of one file.
    pub fn get(&self, target: &Path) -> Option<&FileContent> {
        self.files.get(target)
    }

    /// Renders every file to its final bytes, still in memory.
    /// Disk writing happens once, after the whole run has validated.
    pub fn render(&self) -> BTreeMap<PathBuf, String> {
        self.files
            .iter()
            .map(|(path, content)| (path.clone(), content.render()))
            .collect()
    }

    /// The mode each merged file must be written with, for the files that need one.
    /// Separate from `render` so the rendered text can go through interpolation without a mode having to survive that trip untouched.
    pub fn modes(&self) -> BTreeMap<PathBuf, u32> {
        self.files
            .iter()
            .filter_map(|(path, content)| content.mode().map(|mode| (path.clone(), mode)))
            .collect()
    }

    /// Renders the whole set to one string, for byte comparison in tests.
    pub fn render_all(&self) -> String {
        let mut out = String::new();
        for (path, body) in self.render() {
            out.push_str("--- ");
            out.push_str(&path.display().to_string());
            out.push('\n');
            out.push_str(&body);
        }
        out
    }

    /// Lists the targets this set holds, in sorted order.
    pub fn targets(&self) -> impl Iterator<Item = &PathBuf> {
        self.files.keys()
    }
}

/* Requirements and conflicts are checked across the whole selection before
 * anything merges, so a doomed run cannot produce half a file set. */
fn validate_selection(layers: &[&Layer]) -> Result<(), MergeError> {
    let selected: std::collections::BTreeSet<&LayerId> = layers.iter().map(|l| &l.id).collect();

    for layer in layers {
        for required in &layer.requires {
            if !selected.contains(required) {
                return Err(MergeError::MissingRequirement {
                    layer: layer.id.clone(),
                    requires: required.clone(),
                });
            }
        }
    }

    /* Reported with the pair sorted, so the message does not depend on selection order. */
    let mut found: Option<(LayerId, LayerId)> = None;
    for layer in layers {
        for other in &layer.conflicts {
            if selected.contains(other) {
                let pair = if layer.id <= *other {
                    (layer.id.clone(), other.clone())
                } else {
                    (other.clone(), layer.id.clone())
                };
                if found.as_ref().is_none_or(|best| pair < *best) {
                    found = Some(pair);
                }
            }
        }
    }
    if let Some((a, b)) = found {
        return Err(MergeError::Conflicting { a, b });
    }

    Ok(())
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn json_layer(id: &str, target: &str, value: serde_json::Value) -> Layer {
        Layer::new(id).with_merge(target, Contribution::JsonDeep(value))
    }

    #[test]
    fn missing_requirement_names_both_layers() {
        let husky = Layer::new("husky").requiring("node-pnpm");
        let err = FileSet::build([&husky]).unwrap_err();
        assert_eq!(
            err,
            MergeError::MissingRequirement {
                layer: LayerId::new("husky"),
                requires: LayerId::new("node-pnpm"),
            }
        );
        assert!(err.to_string().contains("husky"));
        assert!(err.to_string().contains("node-pnpm"));
    }

    #[test]
    fn conflicting_layers_are_rejected() {
        let nx = Layer::new("nx").conflicting("turborepo");
        let turbo = Layer::new("turborepo");
        let err = FileSet::build([&nx, &turbo]).unwrap_err();
        assert_eq!(
            err,
            MergeError::Conflicting {
                a: LayerId::new("nx"),
                b: LayerId::new("turborepo"),
            }
        );
    }

    #[test]
    fn conflict_report_is_order_independent() {
        let nx = Layer::new("nx").conflicting("turborepo");
        let turbo = Layer::new("turborepo").conflicting("nx");
        let forward = FileSet::build([&nx, &turbo]).unwrap_err();
        let backward = FileSet::build([&turbo, &nx]).unwrap_err();
        assert_eq!(forward, backward);
    }

    #[test]
    fn strategy_mismatch_names_both_layers() {
        let a = json_layer("a", ".gitignore", json!({ "x": 1 }));
        let b = Layer::new("b").with_merge(
            ".gitignore",
            Contribution::LineBlock(vec!["target".to_owned()]),
        );
        let err = FileSet::build([&a, &b]).unwrap_err();
        assert_eq!(
            err,
            MergeError::StrategyMismatch {
                target: PathBuf::from(".gitignore"),
                a: LayerId::new("a"),
                b: LayerId::new("b"),
                a_strategy: Strategy::JsonDeep,
                b_strategy: Strategy::LineBlock,
            }
        );
    }

    #[test]
    fn non_object_json_root_is_rejected() {
        let a = json_layer("a", "package.json", json!("nope"));
        let err = FileSet::build([&a]).unwrap_err();
        assert_eq!(
            err,
            MergeError::NonObjectRoot {
                target: PathBuf::from("package.json"),
                layer: LayerId::new("a"),
            }
        );
    }

    #[test]
    fn independent_targets_render_sorted_by_path() {
        let a = Layer::new("a")
            .with_merge(
                "package.json",
                Contribution::JsonDeep(json!({ "name": "x" })),
            )
            .with_merge(
                ".gitignore",
                Contribution::LineBlock(vec!["node_modules".to_owned()]),
            );
        let set = FileSet::build([&a]).unwrap();
        let targets: Vec<&PathBuf> = set.targets().collect();
        assert_eq!(
            targets,
            vec![&PathBuf::from(".gitignore"), &PathBuf::from("package.json")]
        );
    }
}
