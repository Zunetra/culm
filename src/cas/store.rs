// === Store

/* The actual content-addressable store: hash bytes in, get the same bytes
 * back later by that hash, or an error if they are missing or the bytes on
 * disk no longer match their own name. Two identical files stored twice
 * occupy one slot, because identical bytes hash identically, which is the
 * whole reason a cache built this way is cheap to keep around. */

use std::fs;
use std::path::{Path, PathBuf};

use super::Hash;
use super::error::CasError;

/// A store rooted at one directory. Objects are addressed by their blake3
/// digest, split into a two-character prefix directory and the rest as the
/// file name (git's own layout), so one directory never has to hold tens of
/// thousands of entries.
#[derive(Debug, Clone)]
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Opens a store at `root`, creating the directory if it does not exist
    /// yet. Opening never touches an existing object, only the directory
    /// that will hold them.
    pub fn open(root: &Path) -> Result<Self, CasError> {
        fs::create_dir_all(root).map_err(|error| CasError::Io {
            path: root.to_owned(),
            message: error.to_string(),
        })?;
        Ok(Store {
            root: root.to_owned(),
        })
    }

    /// The default store location for this machine: the OS cache directory
    /// under `culm/cas`, so a user never has to know this exists to get its
    /// benefit and a `culm cache clear` has one well-known place to empty.
    pub fn default_root() -> Result<PathBuf, CasError> {
        dirs::cache_dir()
            .map(|dir| dir.join("culm").join("cas"))
            .ok_or(CasError::NoCacheDir)
    }

    /// Stores `bytes` under their own blake3 digest and returns it. Storing
    /// the same bytes twice is a no-op the second time: the destination
    /// already holds the same content by definition, so nothing is
    /// rewritten.
    pub fn put(&self, bytes: &[u8]) -> Result<Hash, CasError> {
        let digest = super::hash(bytes);
        let path = self.path_for(&digest);
        if path.exists() {
            return Ok(digest);
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| CasError::Io {
                path: parent.to_owned(),
                message: error.to_string(),
            })?;
        }
        fs::write(&path, bytes).map_err(|error| CasError::Io {
            path,
            message: error.to_string(),
        })?;
        Ok(digest)
    }

    /// Reads the bytes stored under `digest` back out, verifying they still
    /// hash to the digest they are named after. A cache hit must always be
    /// correct, so a mismatch here (disk corruption, a hand-edited file) is
    /// reported rather than silently served.
    pub fn get(&self, digest: &Hash) -> Result<Vec<u8>, CasError> {
        let path = self.path_for(digest);
        let bytes = fs::read(&path).map_err(|_| CasError::Missing {
            hash: digest.to_hex().to_string(),
        })?;
        if super::hash(&bytes) != *digest {
            return Err(CasError::Corrupted {
                hash: digest.to_hex().to_string(),
            });
        }
        Ok(bytes)
    }

    pub fn has(&self, digest: &Hash) -> bool {
        self.path_for(digest).exists()
    }

    /// Removes every object this store holds. Used by `culm cache clear`;
    /// the store itself stays open and usable afterward, the directory is
    /// simply empty again.
    pub fn clear(&self) -> Result<(), CasError> {
        if !self.root.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&self.root).map_err(|error| CasError::Io {
            path: self.root.clone(),
            message: error.to_string(),
        })? {
            let entry = entry.map_err(|error| CasError::Io {
                path: self.root.clone(),
                message: error.to_string(),
            })?;
            let path = entry.path();
            let result = if path.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                fs::remove_file(&path)
            };
            result.map_err(|error| CasError::Io {
                path: path.clone(),
                message: error.to_string(),
            })?;
        }
        Ok(())
    }

    /// The total size on disk of every object this store holds, in bytes,
    /// and how many objects that is. `culm cache stats` reports both.
    pub fn stats(&self) -> Result<StoreStats, CasError> {
        let mut stats = StoreStats::default();
        if !self.root.exists() {
            return Ok(stats);
        }
        let mut stack = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).map_err(|error| CasError::Io {
                path: dir.clone(),
                message: error.to_string(),
            })? {
                let entry = entry.map_err(|error| CasError::Io {
                    path: dir.clone(),
                    message: error.to_string(),
                })?;
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let metadata = entry.metadata().map_err(|error| CasError::Io {
                    path: path.clone(),
                    message: error.to_string(),
                })?;
                stats.object_count += 1;
                stats.total_bytes += metadata.len();
            }
        }
        Ok(stats)
    }

    fn path_for(&self, digest: &Hash) -> PathBuf {
        let hex = digest.to_hex();
        let hex = hex.as_str();
        self.root.join(&hex[..2]).join(&hex[2..])
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StoreStats {
    pub object_count: u64,
    pub total_bytes: u64,
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_then_get_returns_the_same_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let digest = store.put(b"hello world").unwrap();
        assert_eq!(store.get(&digest).unwrap(), b"hello world");
    }

    #[test]
    fn storing_identical_bytes_twice_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let a = store.put(b"same").unwrap();
        let b = store.put(b"same").unwrap();
        assert_eq!(a, b);
        assert_eq!(store.stats().unwrap().object_count, 1);
    }

    #[test]
    fn a_missing_object_is_reported_rather_than_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let digest = super::super::hash(b"never stored");
        assert!(matches!(store.get(&digest), Err(CasError::Missing { .. })));
    }

    #[test]
    fn a_hand_edited_object_is_reported_as_corrupted_rather_than_served() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let digest = store.put(b"original").unwrap();
        let hex = digest.to_hex();
        let hex = hex.as_str();
        let path = dir.path().join(&hex[..2]).join(&hex[2..]);
        std::fs::write(&path, b"tampered").unwrap();
        assert!(matches!(
            store.get(&digest),
            Err(CasError::Corrupted { .. })
        ));
    }

    #[test]
    fn clear_empties_the_store_without_closing_it() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store.put(b"one").unwrap();
        store.put(b"two").unwrap();
        assert_eq!(store.stats().unwrap().object_count, 2);

        store.clear().unwrap();
        assert_eq!(store.stats().unwrap().object_count, 0);

        let digest = store.put(b"three").unwrap();
        assert_eq!(store.get(&digest).unwrap(), b"three");
    }
}
