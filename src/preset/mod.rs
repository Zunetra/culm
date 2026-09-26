// === Presets

/* Decision 6: first-party presets ship in the binary, the same way layers do,
 * for the same reason. A preset naming a layer the catalog cannot resolve is a
 * broken shortcut, and shipping both halves together is what keeps them from
 * drifting apart.
 *
 * TOML rather than the YAML first sketched for presets (decision 20): the crate
 * already parses TOML, every layer is TOML, and `serde_yaml` is archived. A
 * second serialisation format would be a dependency bought for one file
 * shape. */

use std::path::Path;

use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::merge::LayerId;

pub mod remote;

static PRESETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/presets");

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PresetError {
    #[error("{path}: {message}")]
    Malformed { path: String, message: String },

    #[error("there is no preset called `{name}`. Available: {}", .available.join(", "))]
    Unknown {
        name: String,
        available: Vec<String>,
    },

    #[error("preset `{name}` is filed as `{file}`, so one of the two is wrong")]
    Misnamed { name: String, file: String },
}

/// A named selection of layers, as one `presets/*.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub name: String,
    pub description: String,
    /// Qualified ids, to match what the catalog is keyed by and what `--layer` takes.
    pub layers: Vec<String>,
}

impl Preset {
    pub fn ids(&self) -> Vec<LayerId> {
        self.layers.iter().map(LayerId::new).collect()
    }
}

/// Every preset compiled into this binary, in name order.
pub fn all() -> Result<Vec<Preset>, PresetError> {
    let mut files: Vec<&include_dir::File<'_>> = PRESETS
        .files()
        .filter(|file| file.path().extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort_by_key(|file| file.path());

    let mut presets = Vec::new();
    for file in files {
        presets.push(parse(
            file.path(),
            file.contents_utf8().ok_or_else(|| PresetError::Malformed {
                path: file.path().display().to_string(),
                message: "the compiled-in preset is not valid UTF-8".to_owned(),
            })?,
        )?);
    }
    Ok(presets)
}

/// One preset by name, or an error listing the ones that exist.
///
/// The list is in the error rather than behind a separate `culm presets`
/// command, because someone who mistyped a name is already looking at their
/// terminal and should not have to run a second thing to be told the answer.
pub fn get(name: &str) -> Result<Preset, PresetError> {
    let presets = all()?;
    presets
        .iter()
        .find(|preset| preset.name == name)
        .cloned()
        .ok_or_else(|| PresetError::Unknown {
            name: name.to_owned(),
            available: presets.into_iter().map(|preset| preset.name).collect(),
        })
}

/// Parses one preset and checks that its `name` agrees with its filename.
/// The two are separate keys into the same thing, and a preset found by filename but printed by name would be one users could not reliably ask for again.
fn parse(path: &Path, source: &str) -> Result<Preset, PresetError> {
    let preset: Preset = toml::from_str(source).map_err(|error| PresetError::Malformed {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;

    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    if preset.name != stem {
        return Err(PresetError::Misnamed {
            name: preset.name,
            file: path.display().to_string(),
        });
    }
    Ok(preset)
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::Catalog;

    #[test]
    fn the_three_first_party_presets_ship() {
        let names: Vec<String> = all().unwrap().into_iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["backend", "frontend", "monorepo"]);
    }

    /* A preset naming a layer that does not exist is a shortcut that fails only
     * for the user who takes it, which is the worst place to find out. */
    #[test]
    fn every_preset_resolves_against_the_catalog() {
        let catalog = Catalog::embedded().unwrap();
        for preset in all().unwrap() {
            for id in preset.ids() {
                assert!(
                    catalog.get(&id).is_some(),
                    "preset `{}` names `{id}`, which the catalog does not have",
                    preset.name
                );
            }
        }
    }

    #[test]
    fn every_preset_scaffolds_a_legal_selection() {
        let catalog = Catalog::embedded().unwrap();
        for preset in all().unwrap() {
            let entries = crate::layer::resolve(&catalog, &preset.ids(), &[])
                .unwrap_or_else(|e| panic!("preset `{}` does not resolve: {e}", preset.name));
            assert!(!entries.is_empty());
        }
    }

    #[test]
    fn an_unknown_name_is_answered_with_the_ones_that_exist() {
        let error = get("nosuch").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("nosuch"), "{message}");
        assert!(message.contains("monorepo"), "{message}");
    }

    #[test]
    fn a_preset_whose_name_disagrees_with_its_file_is_refused() {
        let source = "name = \"other\"\ndescription = \"x\"\nlayers = []\n";
        assert!(matches!(
            parse(Path::new("backend.toml"), source),
            Err(PresetError::Misnamed { .. })
        ));
    }
}
