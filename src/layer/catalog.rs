// === The layer catalog

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::category::Category;
use super::error::LayerError;
use super::manifest::Manifest;
use super::source::{LayerSource, embedded_file, embedded_manifests};
use crate::merge::LayerId;
use crate::scaffold::Variables;

/// The file a layer is defined by. Named once so the walk and the re-read cannot disagree.
const MANIFEST: &str = "layer.toml";

/// One layer, from whichever tree it was loaded out of.
/// The source travels with the manifest because `[[files]]` paths are relative to the layer's own directory, and the copy plan cannot resolve them otherwise.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    pub manifest: Manifest,
    pub source: LayerSource,
}

impl CatalogEntry {
    /// The layer's own directory, for messages and for tests that reason about layout.
    pub fn dir(&self) -> &Path {
        self.source.dir()
    }

    /// This layer's manifest with the scaffold's variables substituted throughout.
    ///
    /// Rendered over the manifest text and re-parsed, rather than walked field
    /// by field, so every field is covered by construction: a merge strategy
    /// added later needs no change here to interpolate, and none can be
    /// forgotten. The substituted values are culm's own, derived from the
    /// layer id and the shape of the selection, so they cannot carry a quote
    /// into the TOML that a hand-written value might.
    pub fn interpolated(&self, variables: &Variables) -> Result<Manifest, LayerError> {
        let path = self.dir().join(MANIFEST);
        let raw = self
            .source
            .read(Path::new(MANIFEST))
            .ok_or_else(|| LayerError::Unreadable {
                path: path.clone(),
                message: "the layer's manifest is no longer readable".to_owned(),
            })?;
        Manifest::parse(&path.display().to_string(), &variables.render(&raw))
    }
}

/// Every layer available to a selection, keyed by qualified id.
#[derive(Debug, Clone, Default)]
pub struct Catalog {
    entries: BTreeMap<LayerId, CatalogEntry>,
}

impl Catalog {
    /// The layers compiled into this binary, which is what a released `culm` scaffolds from.
    /// Decision 19: culm never reaches for a template at scaffold time, so the catalog a user gets is the one that shipped, forever.
    pub fn embedded() -> Result<Self, LayerError> {
        let mut entries: BTreeMap<LayerId, CatalogEntry> = BTreeMap::new();
        for path in embedded_manifests() {
            let text = embedded_file(&path).ok_or_else(|| LayerError::Unreadable {
                path: path.clone(),
                message: "the compiled-in manifest is not valid UTF-8".to_owned(),
            })?;
            let dir = layer_dir(&path);
            insert(&mut entries, &path, text, LayerSource::Embedded(dir))?;
        }
        Ok(Self { entries })
    }

    /// Loads every `layer.toml` under `root`, at any depth.
    /// Walking rather than reading an index is deliberate: a layer added tomorrow is available without anybody remembering to register it. This is the path layer authoring and the tests take; a release takes [`Catalog::embedded`].
    pub fn load(root: &Path) -> Result<Self, LayerError> {
        let mut paths = Vec::new();
        collect(root, &mut paths)?;
        paths.sort();

        let mut entries: BTreeMap<LayerId, CatalogEntry> = BTreeMap::new();
        for path in paths {
            let text = fs::read_to_string(&path).map_err(|e| LayerError::Unreadable {
                path: path.clone(),
                message: e.to_string(),
            })?;
            let dir = layer_dir(&path);
            insert(&mut entries, &path, &text, LayerSource::Disk(dir))?;
        }
        Ok(Self { entries })
    }

    /// Builds a catalog from entries already in memory, for tests and for presets that do not come off disk.
    pub fn from_entries<I: IntoIterator<Item = CatalogEntry>>(
        entries: I,
    ) -> Result<Self, LayerError> {
        let mut map: BTreeMap<LayerId, CatalogEntry> = BTreeMap::new();
        for entry in entries {
            let id = entry.manifest.qualified_id();
            if map.insert(id.clone(), entry).is_some() {
                return Err(LayerError::DuplicateId { id });
            }
        }
        Ok(Self { entries: map })
    }

    pub fn get(&self, id: &LayerId) -> Option<&CatalogEntry> {
        self.entries.get(id)
    }

    /// Every entry, in qualified id order.
    pub fn entries(&self) -> impl Iterator<Item = &CatalogEntry> {
        self.entries.values()
    }

    pub fn by_category(&self, category: Category) -> impl Iterator<Item = &CatalogEntry> {
        self.entries
            .values()
            .filter(move |entry| entry.manifest.category == category)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Parses one manifest and files it under its qualified id.
/// Shared by both loaders so that a layer means the same thing whichever tree it came out of, which is what the equivalence test rests on.
fn insert(
    entries: &mut BTreeMap<LayerId, CatalogEntry>,
    path: &Path,
    text: &str,
    source: LayerSource,
) -> Result<(), LayerError> {
    let manifest = Manifest::parse(&path.display().to_string(), text)?;
    let id = manifest.qualified_id();
    if entries
        .insert(id.clone(), CatalogEntry { manifest, source })
        .is_some()
    {
        return Err(LayerError::DuplicateId { id });
    }
    Ok(())
}

fn layer_dir(manifest_path: &Path) -> PathBuf {
    manifest_path
        .parent()
        .expect("a layer.toml always sits inside its layer directory")
        .to_path_buf()
}

fn collect(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), LayerError> {
    let entries = fs::read_dir(dir).map_err(|e| LayerError::Unreadable {
        path: dir.to_path_buf(),
        message: e.to_string(),
    })?;
    for entry in entries {
        let path = entry
            .map_err(|e| LayerError::Unreadable {
                path: dir.to_path_buf(),
                message: e.to_string(),
            })?
            .path();
        if path.is_dir() {
            collect(&path, found)?;
        } else if path.file_name().is_some_and(|name| name == MANIFEST) {
            found.push(path);
        }
    }
    Ok(())
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("layers")
    }

    #[test]
    fn the_shipped_layers_load() {
        let catalog = Catalog::load(&root()).unwrap();
        assert!(!catalog.is_empty());
        assert!(catalog.get(&LayerId::new("frontend/next")).is_some());
        assert!(catalog.get(&LayerId::new("toolchain/rust-cargo")).is_some());
    }

    /* The directory is what makes a `[[files]]` path resolvable, so an entry
     * that lost it would fail much later, in the copy plan. */
    #[test]
    fn an_entry_carries_the_directory_its_files_are_relative_to() {
        let catalog = Catalog::load(&root()).unwrap();
        let entry = catalog.get(&LayerId::new("frontend/next")).unwrap();
        assert_eq!(entry.dir(), root().join("frontend").join("next"));
    }

    #[test]
    fn the_embedded_catalog_loads() {
        let catalog = Catalog::embedded().unwrap();
        assert!(!catalog.is_empty());
        assert!(catalog.get(&LayerId::new("frontend/next")).is_some());
        assert!(catalog.get(&LayerId::new("toolchain/rust-cargo")).is_some());
    }

    /* The whole point of keeping `load` after decision 19: a layer author edits
     * the tree on disk and the tests read it, while a release reads the copy
     * compiled in. The moment those two disagree, every test is testing
     * something the user will never run. */
    #[test]
    fn the_embedded_catalog_matches_the_one_on_disk() {
        let disk = Catalog::load(&root()).unwrap();
        let embedded = Catalog::embedded().unwrap();

        let ids = |c: &Catalog| -> Vec<String> {
            c.entries()
                .map(|e| e.manifest.qualified_id().as_str().to_owned())
                .collect()
        };
        assert_eq!(ids(&disk), ids(&embedded));
        assert_eq!(disk.len(), embedded.len());

        for entry in disk.entries() {
            let id = entry.manifest.qualified_id();
            let other = embedded.get(&id).unwrap();
            assert_eq!(entry.manifest, other.manifest, "{id} parses differently");

            for spec in &entry.manifest.files {
                assert_eq!(
                    entry.source.read(&spec.from),
                    other.source.read(&spec.from),
                    "{id} reads `{}` differently",
                    spec.from.display()
                );
            }
        }
    }

    #[test]
    fn a_missing_root_is_reported_with_its_path() {
        let missing = root().join("does-not-exist");
        match Catalog::load(&missing).unwrap_err() {
            LayerError::Unreadable { path, .. } => assert_eq!(path, missing),
            other => panic!("expected an unreadable root, got {other:?}"),
        }
    }

    #[test]
    fn layers_are_grouped_by_category() {
        let catalog = Catalog::load(&root()).unwrap();
        let toolchains: Vec<&str> = catalog
            .by_category(Category::Toolchain)
            .map(|entry| entry.manifest.id.as_str())
            .collect();
        assert_eq!(toolchains, vec!["node-pnpm", "rust-cargo"]);
    }
}
