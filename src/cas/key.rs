// === Cache key

/* docs/DESIGN.md section 3.3: the cache key includes everything, or it is
 * not a cache, it is a bug waiting to happen. Too narrow and a real change
 * produces a stale hit, which is the one failure mode this whole module
 * exists to prevent. Too broad only costs a cache miss, which is merely
 * disappointing. */

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

use super::Hash;

/// Serialized deterministically before hashing, so two identical logical
/// inputs always produce a byte-identical digest regardless of hash-map
/// iteration order. `BTreeMap` already iterates sorted by key, and
/// `serde_json`'s own `Map` (without the `preserve_order` feature, which
/// this crate does not enable) is itself a sorted map, so the JSON these
/// fields render as is already canonical without any extra sorting step
/// here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CacheKey {
    /// `project:task`, so two tasks that happen to share a command and
    /// inputs never share an entry: their declared outputs differ.
    pub task: String,
    pub command: String,
    /* Hex text rather than `blake3::Hash` directly: a hash's own byte
     * representation is not what changes the key's meaning, its ordering
     * in this map is, and hex text is what a person debugging a stale hit
     * actually wants to read. */
    pub input_hashes: BTreeMap<PathBuf, String>,
    /// Only the environment variables the task's own `declared_env`
    /// explicitly named, never the whole environment: `docs/DESIGN.md`
    /// section 3.3 is explicit that a broader key is merely disappointing
    /// while a narrower one is dangerous, so nothing here is inferred.
    /// `None` is an unset variable, which is not the same as an empty one.
    pub declared_env: BTreeMap<String, Option<String>>,
    /// Each dependency's own key digest, by task name. A change upstream
    /// then always reaches downstream, even when the downstream task's
    /// inputs do not name the upstream task's outputs.
    pub dependencies: BTreeMap<String, String>,
    /// `<os>-<arch>`. A binary built on one platform is not the output of
    /// the same task on another, which matters the moment a cache is shared.
    pub platform: String,
    pub tool_version: String,
}

impl CacheKey {
    /// A cache miss must always be safe. A cache hit must always be
    /// correct. This is the one function allowed to decide what "the same
    /// task" means, and it only ever gains inputs over time: every field
    /// added here is a reviewed decision, never an inference this module
    /// makes on its own.
    pub fn digest(&self) -> Hash {
        super::hash(&self.canonical_bytes())
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("CacheKey's fields are all plain, serialisable types")
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn key(command: &str, inputs: &[(&str, &str)]) -> CacheKey {
        CacheKey {
            task: "demo:build".to_owned(),
            command: command.to_owned(),
            input_hashes: inputs
                .iter()
                .map(|(p, h)| (PathBuf::from(p), (*h).to_owned()))
                .collect(),
            declared_env: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            platform: "linux-x86_64".to_owned(),
            tool_version: "1.0.0".to_owned(),
        }
    }

    #[test]
    fn identical_keys_digest_identically() {
        let a = key("cargo build", &[("src/main.rs", "aa"), ("Cargo.toml", "bb")]);
        let b = key("cargo build", &[("Cargo.toml", "bb"), ("src/main.rs", "aa")]);
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn a_changed_input_hash_changes_the_digest() {
        let a = key("cargo build", &[("src/main.rs", "aa")]);
        let b = key("cargo build", &[("src/main.rs", "cc")]);
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn a_changed_command_changes_the_digest() {
        let a = key("cargo build", &[]);
        let b = key("cargo build --release", &[]);
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn declared_env_and_tool_version_both_change_the_digest() {
        let mut a = key("cargo build", &[]);
        let mut b = a.clone();
        b.tool_version = "1.0.1".to_owned();
        assert_ne!(a.digest(), b.digest());

        b = a.clone();
        b.declared_env.insert("RUSTFLAGS".to_owned(), Some("-C opt".to_owned()));
        assert_ne!(a.digest(), b.digest());

        a.declared_env.insert("RUSTFLAGS".to_owned(), Some("-C opt".to_owned()));
        assert_eq!(a.digest(), b.digest());
    }

    #[test]
    fn an_unset_variable_is_not_an_empty_one() {
        let mut a = key("cargo build", &[]);
        let mut b = a.clone();
        a.declared_env.insert("RUSTFLAGS".to_owned(), None);
        b.declared_env.insert("RUSTFLAGS".to_owned(), Some(String::new()));
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn task_platform_and_dependencies_each_change_the_digest() {
        let a = key("make", &[]);

        let mut b = a.clone();
        b.task = "demo:other".to_owned();
        assert_ne!(a.digest(), b.digest());

        let mut c = a.clone();
        c.platform = "macos-aarch64".to_owned();
        assert_ne!(a.digest(), c.digest());

        let mut d = a.clone();
        d.dependencies.insert("build".to_owned(), "ff".to_owned());
        assert_ne!(a.digest(), d.digest());
    }
}
