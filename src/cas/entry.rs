// === Cache entry

/* docs/DESIGN.md section 3.4's `CacheEntry`, plus the record/restore pair
 * that actually uses a `Store` to save and replay one. An entry's own
 * metadata (which outputs exist, the exit code, the run's stdout/stderr) is
 * small and not itself worth content-addressing on its own account, so it is
 * looked up directly by the cache key's digest; the output bytes, stdout,
 * and stderr it references are each ordinary objects in the same `Store`,
 * so identical output content across two different tasks is still stored
 * once. */

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::error::CasError;
use super::key::CacheKey;
use super::store::{Store, write_atomically};

/// One recorded run of a cacheable task, per docs/DESIGN.md section 3.4.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Each output path the task declared, and the digest of its content in
    /// the same `Store` this entry itself lives in.
    pub outputs: BTreeMap<PathBuf, String>,
    /// Outputs that were executable when recorded. A restored binary that
    /// has lost its execute bit is a wrong cache hit like any other.
    #[serde(default)]
    pub executables: BTreeSet<PathBuf>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// One output file as a finished run left it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFile {
    /// Relative to the task's working directory.
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub executable: bool,
}

/// Everything a finished run produced that a later hit has to reproduce.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Captured {
    pub outputs: Vec<OutputFile>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl CacheEntry {
    fn metadata_path(store: &Store, key: &CacheKey) -> PathBuf {
        store
            .root()
            .join("entries")
            .join(format!("{}.json", key.digest().to_hex()))
    }

    /// Saves a finished run's outputs, stdout, and stderr into `store`, then
    /// writes the entry's own metadata keyed by `key`'s digest. Called after
    /// a task actually ran, on a cache miss, so the next identical `key`
    /// hits.
    pub fn record(key: &CacheKey, run: &Captured, store: &Store) -> Result<Self, CasError> {
        let mut outputs = BTreeMap::new();
        let mut executables = BTreeSet::new();
        for output in &run.outputs {
            let digest = store.put(&output.bytes)?;
            outputs.insert(output.path.clone(), digest.to_hex().to_string());
            if output.executable {
                executables.insert(output.path.clone());
            }
        }

        let entry = CacheEntry {
            outputs,
            executables,
            stdout: store.put(&run.stdout)?.to_hex().to_string(),
            stderr: store.put(&run.stderr)?.to_hex().to_string(),
            exit_code: run.exit_code,
            duration_ms: run.duration_ms,
        };

        let path = Self::metadata_path(store, key);
        let body = serde_json::to_vec(&entry).map_err(|error| CasError::Malformed {
            path: path.clone(),
            message: error.to_string(),
        })?;
        write_atomically(&path, &body)?;
        Ok(entry)
    }

    /// Looks up whatever entry `key` maps to, if any. `None` is an ordinary
    /// cache miss, not an error: a cache miss must always be safe.
    pub fn lookup(key: &CacheKey, store: &Store) -> Result<Option<CacheEntry>, CasError> {
        let path = Self::metadata_path(store, key);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path).map_err(|error| CasError::Io {
            path: path.clone(),
            message: error.to_string(),
        })?;
        let entry = serde_json::from_str(&raw).map_err(|error| CasError::Malformed {
            path,
            message: error.to_string(),
        })?;
        Ok(Some(entry))
    }

    /// Writes every recorded output back to its declared path, relative to
    /// `root` (the task's working directory). A cache hit must always be
    /// correct, so this re-verifies each output's content against the
    /// `Store`'s own digest via `Store::get` rather than trusting the bytes
    /// on disk were never touched since they were recorded.
    pub fn restore(&self, store: &Store, root: &Path) -> Result<(), CasError> {
        let loaded = self.load(store)?;
        write_outputs(&loaded, root)
    }

    /// Reads and verifies every recorded output without writing anything,
    /// so a caller can refuse a corrupted entry before it touches the
    /// workspace at all.
    pub fn load(&self, store: &Store) -> Result<Vec<OutputFile>, CasError> {
        self.outputs
            .iter()
            .map(|(path, digest_hex)| {
                Ok(OutputFile {
                    path: path.clone(),
                    bytes: store.get(&parse_digest(digest_hex)?)?,
                    executable: self.executables.contains(path),
                })
            })
            .collect()
    }

    /// The recorded stdout and stderr, verified like any other object.
    pub fn logs(&self, store: &Store) -> Result<(Vec<u8>, Vec<u8>), CasError> {
        Ok((
            store.get(&parse_digest(&self.stdout)?)?,
            store.get(&parse_digest(&self.stderr)?)?,
        ))
    }
}

/// Writes already loaded outputs under `root`, creating directories as needed.
pub fn write_outputs(outputs: &[OutputFile], root: &Path) -> Result<(), CasError> {
    for output in outputs {
        let destination = root.join(&output.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| CasError::Io {
                path: parent.to_owned(),
                message: error.to_string(),
            })?;
        }
        fs::write(&destination, &output.bytes).map_err(|error| CasError::Io {
            path: destination.clone(),
            message: error.to_string(),
        })?;
        if output.executable {
            make_executable(&destination)?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), CasError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(|error| CasError::Io {
        path: path.to_owned(),
        message: error.to_string(),
    })
}

/* No execute bit to restore on a platform without POSIX permissions. */
#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), CasError> {
    Ok(())
}

fn parse_digest(hex: &str) -> Result<super::Hash, CasError> {
    blake3::Hash::from_hex(hex).map_err(|_| CasError::Corrupted {
        hash: hex.to_owned(),
    })
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap as Map;

    fn key(command: &str) -> CacheKey {
        CacheKey {
            task: "demo:build".to_owned(),
            command: command.to_owned(),
            input_hashes: Map::new(),
            declared_env: Map::new(),
            dependencies: Map::new(),
            platform: "linux-x86_64".to_owned(),
            tool_version: "1.0.0".to_owned(),
        }
    }

    fn run_with(path: &str, bytes: &[u8], executable: bool) -> Captured {
        Captured {
            outputs: vec![OutputFile {
                path: PathBuf::from(path),
                bytes: bytes.to_vec(),
                executable,
            }],
            stdout: b"stdout text".to_vec(),
            stderr: Vec::new(),
            exit_code: 0,
            duration_ms: 1234,
        }
    }

    #[test]
    fn a_miss_before_anything_is_recorded_is_none_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let found = CacheEntry::lookup(&key("cargo build"), &store).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn a_recorded_entry_round_trips_its_outputs_and_logs_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let cache_key = key("cargo build --release");

        CacheEntry::record(
            &cache_key,
            &run_with("target/release/backend", b"binary bytes", false),
            &store,
        )
        .unwrap();

        let found = CacheEntry::lookup(&cache_key, &store).unwrap().unwrap();
        assert_eq!(found.exit_code, 0);
        assert_eq!(found.duration_ms, 1234);
        assert_eq!(found.logs(&store).unwrap().0, b"stdout text");

        let restore_dir = tempfile::tempdir().unwrap();
        found.restore(&store, restore_dir.path()).unwrap();
        let restored = fs::read(restore_dir.path().join("target/release/backend")).unwrap();
        assert_eq!(restored, b"binary bytes");
    }

    #[cfg(unix)]
    #[test]
    fn an_executable_output_is_restored_executable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let cache_key = key("cargo build");
        let entry = CacheEntry::record(&cache_key, &run_with("bin/app", b"elf", true), &store)
            .unwrap();

        let restore_dir = tempfile::tempdir().unwrap();
        entry.restore(&store, restore_dir.path()).unwrap();
        let mode = fs::metadata(restore_dir.path().join("bin/app"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }

    #[test]
    fn two_different_commands_never_share_an_entry() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        CacheEntry::record(&key("a"), &Captured::default(), &store).unwrap();

        let found = CacheEntry::lookup(&key("b"), &store).unwrap();
        assert!(found.is_none());
    }
}
