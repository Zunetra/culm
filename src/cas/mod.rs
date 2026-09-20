// === cas

/* Decision 11: a content-addressable store, built as a module here rather
 * than as its own crate, extracted only if a second real consumer ever
 * needs it. It hashes a file's bytes, stores the file under that hash, and
 * hands the hash back; a build cache is exactly that shape, which is what
 * docs/02-runner.md section 5 explains at length. r0.2 milestone. */

pub mod entry;
pub mod error;
pub mod key;
pub mod store;

pub use entry::CacheEntry;
pub use error::CasError;
pub use key::CacheKey;
pub use store::Store;

/// A blake3 digest, used both as a `cas` object's address and as a cache
/// key's own digest. Re-exported under this name so nothing outside this
/// module has to depend on `blake3` directly to name the type.
pub type Hash = blake3::Hash;

/// Hashes a byte slice with blake3, the one hash this whole module ever
/// computes with, so every cache key and every stored object shares the
/// same collision resistance.
pub fn hash(bytes: &[u8]) -> Hash {
    blake3::hash(bytes)
}
