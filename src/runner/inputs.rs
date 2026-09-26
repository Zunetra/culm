// === Inputs and outputs

/* What a task reads and what it leaves behind, resolved against its working
 * directory. Content is hashed, never mtime (docs/DESIGN.md section 3.3): a
 * fresh clone or a branch switch changes every mtime and possibly no content.
 *
 * Each glob is walked from its own literal base, the leading components with
 * no wildcard in them, so `../Cargo.lock` from a project directory works and
 * a narrow glob never walks the whole tree. Inputs honour `.gitignore`, the
 * ripgrep way, since a glob starting with a double star would otherwise
 * descend into `node_modules`; a literal path is always read, ignored or not,
 * so naming an ignored file explicitly is how it gets into the key. Outputs
 * ignore `.gitignore` entirely, because build output is exactly what it
 * usually lists. */

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use globset::{Glob, GlobBuilder, GlobMatcher};
use ignore::WalkBuilder;

use super::error::RunnerError;
use crate::cas::OutputFile;

/// What a missing literal input hashes to in the key.
/// Creating the file later then changes the key, as it should.
const MISSING: &str = "missing";

/// One `inputs` or `outputs` entry, split into where to start walking and
/// what to match there.
struct Pattern {
    /// The leading components with no glob syntax, as written.
    base: PathBuf,
    /// `None` when the whole pattern is a literal path.
    matcher: Option<GlobMatcher>,
}

impl Pattern {
    fn parse(raw: &str) -> Result<Self, RunnerError> {
        let mut base = PathBuf::new();
        let mut literal = true;
        for part in raw.split('/') {
            if part.contains(['*', '?', '[', '{']) {
                literal = false;
                break;
            }
            base.push(part);
        }
        if literal {
            return Ok(Pattern {
                base: PathBuf::from(raw),
                matcher: None,
            });
        }
        let glob: Glob = GlobBuilder::new(raw)
            .literal_separator(true)
            .build()
            .map_err(|error| RunnerError::BadPattern {
                pattern: raw.to_owned(),
                message: error.to_string(),
            })?;
        Ok(Pattern {
            base,
            matcher: Some(glob.compile_matcher()),
        })
    }
}

/// Every file the task's `inputs` name, by path as written relative to
/// `root`, mapped to the hex blake3 digest of its content.
pub fn hash_inputs(root: &Path, patterns: &[String]) -> Result<BTreeMap<PathBuf, String>, RunnerError> {
    let mut hashes = BTreeMap::new();
    for raw in patterns {
        let pattern = Pattern::parse(raw)?;
        let start = root.join(&pattern.base);

        if pattern.matcher.is_none() && !start.is_dir() {
            let digest = match fs::read(&start) {
                Ok(bytes) => crate::cas::hash(&bytes).to_hex().to_string(),
                Err(_) if !start.exists() => MISSING.to_owned(),
                Err(error) => return Err(unreadable(&start, error)),
            };
            hashes.insert(pattern.base.clone(), digest);
            continue;
        }

        for (relative, absolute) in walk(&start, &pattern.base, true)? {
            if matches(&pattern, &relative) {
                let bytes = fs::read(&absolute).map_err(|error| unreadable(&absolute, error))?;
                hashes.insert(relative, crate::cas::hash(&bytes).to_hex().to_string());
            }
        }
    }
    Ok(hashes)
}

/// Every file the task's `outputs` name, as the run just left them.
/// A declared output that does not exist is simply not recorded.
pub fn collect_outputs(root: &Path, patterns: &[String]) -> Result<Vec<OutputFile>, RunnerError> {
    let mut found: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for raw in patterns {
        let pattern = Pattern::parse(raw)?;
        let start = root.join(&pattern.base);
        if pattern.matcher.is_none() && start.is_file() {
            found.insert(pattern.base.clone(), start);
            continue;
        }
        if !start.exists() {
            continue;
        }
        for (relative, absolute) in walk(&start, &pattern.base, false)? {
            if matches(&pattern, &relative) {
                found.insert(relative, absolute);
            }
        }
    }

    found
        .into_iter()
        .map(|(path, absolute)| {
            let bytes = fs::read(&absolute).map_err(|error| unreadable(&absolute, error))?;
            Ok(OutputFile {
                executable: is_executable(&absolute),
                path,
                bytes,
            })
        })
        .collect()
}

/// Removes whatever the task's `outputs` name, so a restore leaves exactly
/// what was recorded and not a stale file from some earlier build beside it.
pub fn clear_outputs(root: &Path, patterns: &[String]) -> Result<(), RunnerError> {
    for raw in patterns {
        let pattern = Pattern::parse(raw)?;
        let start = root.join(&pattern.base);
        if pattern.matcher.is_none() {
            let result = if start.is_dir() {
                fs::remove_dir_all(&start)
            } else if start.exists() {
                fs::remove_file(&start)
            } else {
                Ok(())
            };
            result.map_err(|error| unreadable(&start, error))?;
            continue;
        }
        if !start.exists() {
            continue;
        }
        for (relative, absolute) in walk(&start, &pattern.base, false)? {
            if matches(&pattern, &relative) {
                fs::remove_file(&absolute).map_err(|error| unreadable(&absolute, error))?;
            }
        }
    }
    Ok(())
}

/// Whether `file`, an absolute path, is one of the inputs `patterns` name
/// from `dir`. Used by `--affected`, where the file may already be deleted,
/// so nothing here touches the disk.
pub fn input_matches(dir: &Path, patterns: &[String], file: &Path) -> Result<bool, RunnerError> {
    for raw in patterns {
        let pattern = Pattern::parse(raw)?;
        let start = normalise(&dir.join(&pattern.base));
        let Ok(tail) = file.strip_prefix(&start) else {
            continue;
        };
        if matches(&pattern, &pattern.base.join(tail)) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Resolves `.` and `..` lexically, without the filesystem.
fn normalise(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normal.pop();
            }
            other => normal.push(other),
        }
    }
    normal
}

/// Refuses an output that is absolute or climbs out of the task's directory.
/// Outputs are deleted before a restore, so this is what keeps a typo in
/// `culm.toml` from deleting something outside the project.
pub fn check_outputs(task: &str, patterns: &[String]) -> Result<(), RunnerError> {
    for raw in patterns {
        let path = Path::new(raw);
        let escapes = path
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir));
        if escapes || raw.is_empty() {
            return Err(RunnerError::OutputOutsideProject {
                task: task.to_owned(),
                path: raw.clone(),
            });
        }
    }
    Ok(())
}

fn matches(pattern: &Pattern, relative: &Path) -> bool {
    match &pattern.matcher {
        Some(matcher) => matcher.is_match(relative),
        /* A literal directory names everything under it. */
        None => true,
    }
}

/// Every file under `start`, paired with its path as `base` would spell it.
fn walk(start: &Path, base: &Path, honour_gitignore: bool) -> Result<Vec<(PathBuf, PathBuf)>, RunnerError> {
    let mut files = Vec::new();
    if !start.exists() {
        return Ok(files);
    }
    let walker = WalkBuilder::new(start)
        .standard_filters(honour_gitignore)
        .hidden(false)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|entry| entry.file_name() != ".git")
        .build();
    for entry in walker {
        let entry = entry.map_err(|error| RunnerError::Unreadable {
            path: start.to_owned(),
            message: error.to_string(),
        })?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let absolute = entry.into_path();
        let tail = absolute.strip_prefix(start).unwrap_or(&absolute);
        files.push((base.join(tail), absolute));
    }
    Ok(files)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    false
}

fn unreadable(path: &Path, error: std::io::Error) -> RunnerError {
    RunnerError::Unreadable {
        path: path.to_owned(),
        message: error.to_string(),
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, path: &str, contents: &str) {
        let full = root.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, contents).unwrap();
    }

    fn strings(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_glob_hashes_only_what_it_matches() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "src/main.rs", "fn main() {}");
        write(dir.path(), "src/deep/lib.rs", "pub fn f() {}");
        write(dir.path(), "src/notes.md", "not rust");

        let hashes = hash_inputs(dir.path(), &strings(&["src/**/*.rs"])).unwrap();
        let paths: Vec<&Path> = hashes.keys().map(PathBuf::as_path).collect();
        assert_eq!(paths, vec![Path::new("src/deep/lib.rs"), Path::new("src/main.rs")]);
    }

    #[test]
    fn changing_content_changes_the_hash_and_touching_does_not() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "a.txt", "one");
        let first = hash_inputs(dir.path(), &strings(&["a.txt"])).unwrap();

        write(dir.path(), "a.txt", "one");
        assert_eq!(first, hash_inputs(dir.path(), &strings(&["a.txt"])).unwrap());

        write(dir.path(), "a.txt", "two");
        assert_ne!(first, hash_inputs(dir.path(), &strings(&["a.txt"])).unwrap());
    }

    #[test]
    fn a_missing_literal_input_is_part_of_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let before = hash_inputs(dir.path(), &strings(&["Cargo.lock"])).unwrap();
        assert_eq!(before[Path::new("Cargo.lock")], MISSING);

        write(dir.path(), "Cargo.lock", "x");
        assert_ne!(before, hash_inputs(dir.path(), &strings(&["Cargo.lock"])).unwrap());
    }

    #[test]
    fn a_parent_relative_input_is_read_where_it_is() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "Cargo.lock", "root lockfile");
        let project = dir.path().join("crates/app");
        fs::create_dir_all(&project).unwrap();

        let hashes = hash_inputs(&project, &strings(&["../../Cargo.lock"])).unwrap();
        assert_ne!(hashes[Path::new("../../Cargo.lock")], MISSING);
    }

    #[test]
    fn gitignored_files_are_skipped_by_a_glob_but_not_when_named() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".gitignore", "secret.env\n");
        write(dir.path(), "secret.env", "A=1");
        write(dir.path(), "app.env", "B=2");

        let globbed = hash_inputs(dir.path(), &strings(&["*.env"])).unwrap();
        assert!(globbed.contains_key(Path::new("app.env")));
        assert!(!globbed.contains_key(Path::new("secret.env")));

        let named = hash_inputs(dir.path(), &strings(&["secret.env"])).unwrap();
        assert_ne!(named[Path::new("secret.env")], MISSING);
    }

    #[test]
    fn outputs_are_collected_even_when_gitignored() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), ".gitignore", "dist/\n");
        write(dir.path(), "dist/index.js", "built");
        write(dir.path(), "dist/assets/a.css", "css");

        let outputs = collect_outputs(dir.path(), &strings(&["dist"])).unwrap();
        let paths: Vec<&Path> = outputs.iter().map(|o| o.path.as_path()).collect();
        assert_eq!(paths, vec![Path::new("dist/assets/a.css"), Path::new("dist/index.js")]);
    }

    #[test]
    fn clearing_removes_stale_files_under_an_output_directory() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "dist/stale.js", "old");
        write(dir.path(), "keep.txt", "untouched");

        clear_outputs(dir.path(), &strings(&["dist"])).unwrap();
        assert!(!dir.path().join("dist").exists());
        assert!(dir.path().join("keep.txt").exists());
    }

    #[test]
    fn an_absolute_changed_file_is_matched_from_the_task_directory() {
        let dir = Path::new("/repo/apps/web");
        let globs = strings(&["src/**/*.ts", "../../pnpm-lock.yaml"]);
        assert!(input_matches(dir, &globs, Path::new("/repo/apps/web/src/a/b.ts")).unwrap());
        assert!(input_matches(dir, &globs, Path::new("/repo/pnpm-lock.yaml")).unwrap());
        assert!(!input_matches(dir, &globs, Path::new("/repo/apps/api/src/b.ts")).unwrap());
        assert!(!input_matches(dir, &globs, Path::new("/repo/apps/web/README.md")).unwrap());
    }

    #[test]
    fn an_output_outside_the_project_is_refused() {
        assert!(check_outputs("build", &strings(&["dist"])).is_ok());
        assert!(check_outputs("build", &strings(&["../other"])).is_err());
        assert!(check_outputs("build", &strings(&["/etc/passwd"])).is_err());
        assert!(check_outputs("build", &strings(&["a/../../b"])).is_err());
    }
}
