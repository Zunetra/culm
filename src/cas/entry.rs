// === Cache entry

/* docs/02-runner.md section 5's `CacheEntry`, plus the record/restore pair
 * that actually uses a `Store` to save and replay one. An entry's own
 * metadata (which outputs exist, the exit code, the run's stdout/stderr) is
 * small and not itself worth content-addressing on its own account, so it is
 * looked up directly by the cache key's digest; the output bytes, stdout,
 * and stderr it references are each ordinary objects in the same `Store`,
 * so identical output content across two different tasks is still stored
 * once. */

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::error::CasError;
use super::key::CacheKey;
use super::store::Store;

/// One recorded run of a cacheable task, per docs/02-runner.md section 5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Each output path the task declared, and the digest of its content in
    /// the same `Store` this entry itself lives in.
    pub outputs: BTreeMap<PathBuf, String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl CacheEntry {
    fn metadata_path(store_root: &Path, key: &CacheKey) -> PathBuf {
        store_root.join("entries").join(format!(
            "{}.json",
            key.digest().to_hex()
        ))
    }

    /// Saves this entry's outputs, stdout, and stderr into `store`, then
    /// writes the entry's own metadata keyed by `key`'s digest. Called after
    /// a task actually ran, on a cache miss, so the next identical `key`
    /// hits.
    pub fn record(
        key: &CacheKey,
        outputs: &[(PathBuf, Vec<u8>)],
        stdout: &[u8],
        stderr: &[u8],
        exit_code: i32,
        duration_ms: u64,
        store: &Store,
        store_root: &Path,
    ) -> Result<(), CasError> {
        let mut recorded = BTreeMap::new();
        for (path, bytes) in outputs {
            let digest = store.put(bytes)?;
            recorded.insert(path.clone(), digest.to_hex().to_string());
        }

        let stdout_digest = store.put(stdout)?;
        let stderr_digest = store.put(stderr)?;

        let entry = CacheEntry {
            outputs: recorded,
            stdout: stdout_digest.to_hex().to_string(),
            stderr: stderr_digest.to_hex().to_string(),
            exit_code,
            duration_ms,
        };

        let path = Self::metadata_path(store_root, key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| CasError::Io {
                path: parent.to_owned(),
                message: error.to_string(),
            })?;
        }
        let body = serde_json::to_string(&entry).map_err(|error| CasError::Malformed {
            path: path.clone(),
            message: error.to_string(),
        })?;
        fs::write(&path, body).map_err(|error| CasError::Io {
            path,
            message: error.to_string(),
        })?;

        Ok(())
    }

    /// Looks up whatever entry `key` maps to, if any. `None` is an ordinary
    /// cache miss, not an error: a cache miss must always be safe.
    pub fn lookup(key: &CacheKey, store_root: &Path) -> Result<Option<CacheEntry>, CasError> {
        let path = Self::metadata_path(store_root, key);
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
        for (path, digest_hex) in &self.outputs {
            let digest = parse_digest(digest_hex)?;
            let bytes = store.get(&digest)?;
            let destination = root.join(path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).map_err(|error| CasError::Io {
                    path: parent.to_owned(),
                    message: error.to_string(),
                })?;
            }
            fs::write(&destination, bytes).map_err(|error| CasError::Io {
                path: destination,
                message: error.to_string(),
            })?;
        }
        Ok(())
    }
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
            command: command.to_owned(),
            input_hashes: Map::new(),
            declared_env: Map::new(),
            tool_version: "1.0.0".to_owned(),
        }
    }

    #[test]
    fn a_miss_before_anything_is_recorded_is_none_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let found = CacheEntry::lookup(&key("cargo build"), dir.path()).unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn a_recorded_entry_round_trips_its_outputs_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let cache_key = key("cargo build --release");

        CacheEntry::record(
            &cache_key,
            &[(PathBuf::from("target/release/backend"), b"binary bytes".to_vec())],
            b"stdout text",
            b"",
            0,
            1234,
            &store,
            dir.path(),
        )
        .unwrap();

        let found = CacheEntry::lookup(&cache_key, dir.path()).unwrap().unwrap();
        assert_eq!(found.exit_code, 0);
        assert_eq!(found.duration_ms, 1234);

        let restore_dir = tempfile::tempdir().unwrap();
        found.restore(&store, restore_dir.path()).unwrap();
        let restored =
            fs::read(restore_dir.path().join("target/release/backend")).unwrap();
        assert_eq!(restored, b"binary bytes");
    }

    #[test]
    fn two_different_commands_never_share_an_entry() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        CacheEntry::record(&key("a"), &[], b"", b"", 0, 1, &store, dir.path()).unwrap();

        let found = CacheEntry::lookup(&key("b"), dir.path()).unwrap();
        assert!(found.is_none());
    }
}
