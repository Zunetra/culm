// === Where a layer's files come from

/* Decision 19: templates ship inside the binary, because nightly CI can only
 * build what shipped, and a scaffold that reaches for a directory at run time
 * breaks for everyone who is not standing in this repository. The catalog still
 * has to load off disk for layer authoring and for the tests, so the difference
 * between the two is confined here rather than spread through the copy plan. */

use std::fs;
use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

/// The layer tree, compiled in. `files/` subdirectories come with it, which is the point: a manifest without its templates fails at write time rather than load time.
static LAYERS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/layers");

/// The directory a layer's `[[files]]` paths resolve against.
/// Both variants name a directory; they differ only in which tree that directory sits in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerSource {
    /// A directory on disk, as an absolute or working-directory-relative path.
    Disk(PathBuf),
    /// A directory inside the compiled-in tree, named relative to that tree's root.
    Embedded(PathBuf),
}

impl LayerSource {
    /// Reads a template named relative to this layer's directory.
    /// `None` means the layer declares a file it does not carry, which the copy plan reports as a missing template. An unreadable file and an absent one are the same failure to a caller that has no way to fix either.
    pub fn read(&self, relative: &Path) -> Option<String> {
        match self {
            Self::Disk(dir) => fs::read_to_string(dir.join(relative)).ok(),
            Self::Embedded(dir) => LAYERS
                .get_file(dir.join(relative))
                .and_then(|file| file.contents_utf8())
                .map(str::to_owned),
        }
    }

    /// The layer's own directory, for messages and for tests that reason about layout.
    pub fn dir(&self) -> &Path {
        match self {
            Self::Disk(dir) | Self::Embedded(dir) => dir,
        }
    }
}

/// Every `layer.toml` in the compiled-in tree, as paths relative to its root, sorted.
/// Sorted because the catalog rejects a duplicate id by whichever entry it meets second, and an unordered walk would report a different one per build.
pub(super) fn embedded_manifests() -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect(&LAYERS, &mut found);
    found.sort();
    found
}

fn collect(dir: &Dir<'_>, found: &mut Vec<PathBuf>) {
    for file in dir.files() {
        if file
            .path()
            .file_name()
            .is_some_and(|name| name == "layer.toml")
        {
            found.push(file.path().to_path_buf());
        }
    }
    for child in dir.dirs() {
        collect(child, found);
    }
}

/// The contents of a compiled-in file, named relative to the layer tree's root.
pub(super) fn embedded_file(path: &Path) -> Option<&'static str> {
    LAYERS.get_file(path).and_then(|file| file.contents_utf8())
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layer_tree_is_compiled_in() {
        let manifests = embedded_manifests();
        assert!(!manifests.is_empty());
        assert!(manifests.contains(&PathBuf::from("frontend/next/layer.toml")));
    }

    /* Embedding the manifests without their templates is the failure this task
     * exists to prevent: it would load cleanly and only break at write time. */
    #[test]
    fn a_layer_carries_its_templates_too() {
        let source = LayerSource::Embedded(PathBuf::from("backend/axum"));
        assert!(source.read(Path::new("files/main.rs")).is_some());
    }

    #[test]
    fn a_template_a_layer_does_not_carry_is_absent() {
        let source = LayerSource::Embedded(PathBuf::from("backend/axum"));
        assert!(source.read(Path::new("files/nothing-here.rs")).is_none());
    }

    #[test]
    fn the_two_sources_read_the_same_bytes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("layers");
        let disk = LayerSource::Disk(root.join("backend").join("axum"));
        let embedded = LayerSource::Embedded(PathBuf::from("backend/axum"));
        let template = Path::new("files/main.rs");
        assert_eq!(disk.read(template), embedded.read(template));
    }
}
