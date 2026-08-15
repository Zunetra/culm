// === The copy plan

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use super::error::ScaffoldError;
use super::interpolate::Variables;
use crate::layer::{LayerSource, Manifest};
use crate::merge::LayerId;

/// One file to be written into the generated workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub to: PathBuf,
    pub contents: String,
    pub mode: Option<u32>,
    pub owner: LayerId,
}

/// Everything the selected layers will write, built in memory before anything touches disk.
/// The whole plan is assembled and validated first, because a scaffold that fails halfway leaves a directory nobody can trust and nobody wants to clean up.
#[derive(Debug, Clone, Default)]
pub struct CopyPlan {
    files: BTreeMap<PathBuf, PlannedFile>,
}

impl CopyPlan {
    pub fn new() -> Self {
        Self::default()
    }

    /// Plans every `[[files]]` entry of every manifest.
    /// Each manifest arrives with the source its `from` paths resolve against, which is what lets the same plan be built from the compiled-in tree or from a directory on disk.
    pub fn build<'a, I>(manifests: I, variables: &Variables) -> Result<Self, ScaffoldError>
    where
        I: IntoIterator<Item = (&'a Manifest, &'a LayerSource)>,
    {
        let mut plan = Self::new();
        for (manifest, source) in manifests {
            plan.add(manifest, source, variables)?;
        }
        Ok(plan)
    }

    fn add(
        &mut self,
        manifest: &Manifest,
        source: &LayerSource,
        variables: &Variables,
    ) -> Result<(), ScaffoldError> {
        let owner = manifest.qualified_id();

        for spec in &manifest.files {
            let raw = source
                .read(&spec.from)
                .ok_or_else(|| ScaffoldError::TemplateMissing {
                    layer: owner.clone(),
                    from: spec.from.clone(),
                })?;

            let to = normalise(&spec.to).ok_or_else(|| ScaffoldError::EscapesRoot {
                layer: owner.clone(),
                to: spec.to.clone(),
            })?;

            let planned = PlannedFile {
                to: to.clone(),
                contents: variables.render(&raw),
                mode: spec.mode,
                owner: owner.clone(),
            };

            match self.files.get(&to) {
                /* Identical content from two layers is not a conflict: a shared
                 * editorconfig written twice is the same file either way. */
                Some(existing) if existing.contents == planned.contents => {}
                Some(existing) => {
                    let (a, b) = if existing.owner <= planned.owner {
                        (existing.owner.clone(), planned.owner.clone())
                    } else {
                        (planned.owner.clone(), existing.owner.clone())
                    };
                    return Err(ScaffoldError::FileCollision { to, a, b });
                }
                None => {
                    self.files.insert(to, planned);
                }
            }
        }

        Ok(())
    }

    pub fn files(&self) -> impl Iterator<Item = &PlannedFile> {
        self.files.values()
    }

    pub fn get(&self, to: &Path) -> Option<&PlannedFile> {
        self.files.get(to)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Resolves a destination to a path that stays inside the workspace root.
/// Returns `None` for anything absolute, rooted, or climbing out with `..`.
fn normalise(to: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for component in to.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if out.as_os_str().is_empty() {
        return None;
    }
    Some(out)
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_an_ordinary_relative_path() {
        assert_eq!(
            normalise(Path::new("src/main.rs")),
            Some(PathBuf::from("src/main.rs"))
        );
    }

    #[test]
    fn strips_a_leading_dot() {
        assert_eq!(
            normalise(Path::new("./bacon.toml")),
            Some(PathBuf::from("bacon.toml"))
        );
    }

    /* A layer is data. Data that can write outside the directory it was
     * pointed at can overwrite anything the user can. */
    #[test]
    fn refuses_to_climb_out_of_the_workspace() {
        assert_eq!(normalise(Path::new("../../.ssh/authorized_keys")), None);
        assert_eq!(normalise(Path::new("src/../../etc/passwd")), None);
    }

    #[test]
    fn refuses_an_absolute_path() {
        assert_eq!(normalise(Path::new("/etc/passwd")), None);
    }

    #[test]
    fn refuses_an_empty_destination() {
        assert_eq!(normalise(Path::new("")), None);
        assert_eq!(normalise(Path::new("./")), None);
    }
}
