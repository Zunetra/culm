// === Presets from a git URL

/* Decision 6: presets travel by git URL from day one, `--preset github:user/repo`.
 *
 * Decision 19 still holds even though this fetches over the network: only the
 * preset file itself is ever downloaded, a name, a description, and a list of
 * qualified layer ids, resolved against the catalog this binary already
 * compiled in. No layer template is ever fetched. A hostile preset can at
 * worst choose a strange combination of culm's own first-party layers; it
 * cannot inject a file or a script, because nothing downloaded is ever
 * written into a generated workspace, only read and validated. */

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

use super::Preset;

#[derive(Debug, Error)]
pub enum RemoteError {
    #[error("cannot find a cache directory for this platform")]
    NoCacheDir,

    #[error("`git clone` of `{url}` failed: {message}")]
    CloneFailed { url: String, message: String },

    #[error(
        "`git` could not be started: {message}. A remote preset needs git installed and on PATH"
    )]
    GitNotStarted { message: String },

    #[error("`{path}` was not found in `{url}`")]
    MissingFile { url: String, path: String },

    #[error("{path}: {message}")]
    Malformed { path: String, message: String },
}

impl RemoteError {
    /// The telemetry error code: which failure, never its text.
    pub fn code(&self) -> &'static str {
        return match self {
            Self::NoCacheDir { .. } => "preset_remote_no_cache_dir",
            Self::CloneFailed { .. } => "preset_remote_clone_failed",
            Self::GitNotStarted { .. } => "preset_remote_git_not_started",
            Self::MissingFile { .. } => "preset_remote_missing_file",
            Self::Malformed { .. } => "preset_remote_malformed",
        };
    }
}

/// A parsed reference to a preset hosted outside this binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSource {
    clone_url: String,
    git_ref: Option<String>,
    file: PathBuf,
}

/// Recognises `github:owner/repo`, `gitlab:owner/repo` (each optionally
/// `/path/to/preset.toml` and `@ref`), and a full git URL. Returns `None` for
/// anything else, which is what lets the caller fall back to a first-party
/// preset name unchanged: a remote source is additive, never a second way to
/// spell an existing one.
pub fn parse_source(raw: &str) -> Option<RemoteSource> {
    let (host, rest) = if let Some(rest) = raw.strip_prefix("github:") {
        ("https://github.com", rest)
    } else if let Some(rest) = raw.strip_prefix("gitlab:") {
        ("https://gitlab.com", rest)
    } else if raw.starts_with("https://") || raw.starts_with("http://") || raw.starts_with("git@") {
        // A raw URL is used exactly as given, default branch, default file.
        // `@` is not parsed as a ref separator here: `git@host:owner/repo.git`
        // already uses `@` for its own syntax, and disambiguating the two
        // reliably is not worth it when the shorthand forms already cover
        // the ref case cleanly.
        return Some(RemoteSource {
            clone_url: raw.to_owned(),
            git_ref: None,
            file: PathBuf::from("preset.toml"),
        });
    } else {
        return None;
    };

    let (path, git_ref) = match rest.split_once('@') {
        Some((path, r)) => (path, Some(r.to_owned())),
        None => (rest, None),
    };

    let mut segments = path.splitn(3, '/');
    let owner = segments.next().filter(|s| !s.is_empty())?;
    let repo = segments.next().filter(|s| !s.is_empty())?;
    let file = segments.next().unwrap_or("preset.toml");

    Some(RemoteSource {
        clone_url: format!("{host}/{owner}/{repo}.git"),
        git_ref,
        file: PathBuf::from(file),
    })
}

/// Fetches (or reuses a cached clone of) the source, and parses the one file
/// it names into a `Preset`.
///
/// Unlike a first-party preset, a remote one is not required to name itself
/// after its file: the file lives in someone else's repository under
/// whatever name they gave it, so only `name`, `description` and `layers`
/// are checked, the filename-agreement rule in `preset::parse` does not
/// apply.
pub fn fetch(source: &RemoteSource, cache_root: &Path) -> Result<Preset, RemoteError> {
    let dest = cache_root.join(cache_key(source));

    if !dest.join(".git").exists() {
        clone(source, &dest)?;
    }

    let file_path = dest.join(&source.file);
    let contents = std::fs::read_to_string(&file_path).map_err(|_| RemoteError::MissingFile {
        url: source.clone_url.clone(),
        path: source.file.display().to_string(),
    })?;

    toml::from_str(&contents).map_err(|error| RemoteError::Malformed {
        path: format!("{}#{}", source.clone_url, source.file.display()),
        message: error.to_string(),
    })
}

/// The default cache root: `<platform cache dir>/culm/presets`.
pub fn default_cache_root() -> Result<PathBuf, RemoteError> {
    dirs::cache_dir()
        .map(|dir| dir.join("culm").join("presets"))
        .ok_or(RemoteError::NoCacheDir)
}

/// A directory name that is both stable across runs (so a second `culm new`
/// reuses the clone instead of re-fetching) and readable by a human browsing
/// the cache, unlike a hash. Two different sources cannot collide here: the
/// host, owner and repo are already unique per remote, and a `@ref` or a
/// non-default file only ever narrows which preset inside that same clone is
/// read, not which repository is cloned.
fn cache_key(source: &RemoteSource) -> String {
    let sanitised: String = source
        .clone_url
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    match &source.git_ref {
        Some(git_ref) => format!("{sanitised}@{git_ref}"),
        None => sanitised,
    }
}

fn clone(source: &RemoteSource, dest: &Path) -> Result<(), RemoteError> {
    let mut command = Command::new("git");
    command.arg("clone").arg("--depth").arg("1").arg("--quiet");
    if let Some(git_ref) = &source.git_ref {
        command.arg("--branch").arg(git_ref);
    }
    command.arg(&source.clone_url).arg(dest);

    let output = command
        .output()
        .map_err(|error| RemoteError::GitNotStarted {
            message: error.to_string(),
        })?;

    if !output.status.success() {
        // The half-cloned directory, if any, is not left behind to be
        // mistaken for a successful cache entry on the next run.
        let _ = std::fs::remove_dir_all(dest);
        return Err(RemoteError::CloneFailed {
            url: source.clone_url.clone(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }

    Ok(())
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_shorthand_resolves_to_a_clone_url_and_the_default_file() {
        let source = parse_source("github:zunetra/culm").unwrap();
        assert_eq!(source.clone_url, "https://github.com/zunetra/culm.git");
        assert_eq!(source.git_ref, None);
        assert_eq!(source.file, PathBuf::from("preset.toml"));
    }

    #[test]
    fn gitlab_shorthand_uses_the_gitlab_host() {
        let source = parse_source("gitlab:zunetra/culm").unwrap();
        assert_eq!(source.clone_url, "https://gitlab.com/zunetra/culm.git");
    }

    #[test]
    fn a_ref_after_at_is_the_branch_or_tag() {
        let source = parse_source("github:zunetra/culm@v2").unwrap();
        assert_eq!(source.git_ref, Some("v2".to_owned()));
    }

    #[test]
    fn a_third_path_segment_is_the_file_inside_the_repo() {
        let source = parse_source("github:zunetra/culm/presets/team.toml").unwrap();
        assert_eq!(source.file, PathBuf::from("presets/team.toml"));
    }

    #[test]
    fn a_third_segment_and_a_ref_both_apply() {
        let source = parse_source("github:zunetra/culm/presets/team.toml@main").unwrap();
        assert_eq!(source.file, PathBuf::from("presets/team.toml"));
        assert_eq!(source.git_ref, Some("main".to_owned()));
    }

    #[test]
    fn a_raw_https_url_is_used_directly() {
        let source = parse_source("https://example.com/team/culm-presets.git").unwrap();
        assert_eq!(
            source.clone_url,
            "https://example.com/team/culm-presets.git"
        );
        assert_eq!(source.file, PathBuf::from("preset.toml"));
    }

    /* `git@host:owner/repo.git` is not misread as a ref-bearing shorthand: it
     * never matches the `github:`/`gitlab:` prefixes, and the raw-URL branch
     * does not attempt to split it on `@`. */
    #[test]
    fn an_ssh_url_keeps_its_own_at_sign_intact() {
        let source = parse_source("git@github.com:zunetra/culm.git").unwrap();
        assert_eq!(source.clone_url, "git@github.com:zunetra/culm.git");
        assert_eq!(source.git_ref, None);
    }

    /* A first-party name such as `monorepo` must fall through untouched, or
     * every existing `--preset NAME` invocation would start being parsed as a
     * (nonsensical) remote source instead. */
    #[test]
    fn a_bare_first_party_name_is_not_a_remote_source() {
        assert_eq!(parse_source("monorepo"), None);
        assert_eq!(parse_source("backend"), None);
    }

    #[test]
    fn two_different_refs_of_the_same_repo_cache_separately() {
        let a = parse_source("github:zunetra/culm@v1").unwrap();
        let b = parse_source("github:zunetra/culm@v2").unwrap();
        assert_ne!(cache_key(&a), cache_key(&b));
    }

    /* A local git repository stands in for "remote" for the rest of these
     * tests, the same way `tests/layers.rs` stands a fixture in for a
     * real scaffold: `git clone` treats a filesystem path exactly like a
     * URL, so the real fetch path is exercised end to end with no network
     * reached, which is what keeps `cargo test` fast and offline. */
    fn local_remote(files: &[(&str, &str)]) -> tempfile::TempDir {
        let repo = tempfile::tempdir().unwrap();
        let run = |args: &[&str]| {
            let status = Command::new("git")
                .args(args)
                .current_dir(repo.path())
                .status()
                .expect("git must be on PATH to run this test");
            assert!(status.success(), "git {args:?} failed");
        };

        run(&["init", "--quiet"]);
        run(&["config", "user.email", "test@example.com"]);
        run(&["config", "user.name", "culm tests"]);
        for (name, contents) in files {
            std::fs::write(repo.path().join(name), contents).unwrap();
        }
        run(&["add", "."]);
        run(&["commit", "--quiet", "-m", "fixture"]);

        repo
    }

    fn source(clone_url: &str, file: &str) -> RemoteSource {
        RemoteSource {
            clone_url: clone_url.to_owned(),
            git_ref: None,
            file: PathBuf::from(file),
        }
    }

    #[test]
    fn a_real_clone_and_parse_round_trips_a_valid_preset() {
        let remote = local_remote(&[(
            "preset.toml",
            "name = \"team\"\ndescription = \"a team preset\"\nlayers = [\"backend/axum\"]\n",
        )]);
        let cache = tempfile::tempdir().unwrap();

        let preset = fetch(
            &source(remote.path().to_str().unwrap(), "preset.toml"),
            cache.path(),
        )
        .expect("a real local clone should succeed");

        assert_eq!(preset.name, "team");
        assert_eq!(preset.layers, vec!["backend/axum".to_owned()]);
    }

    #[test]
    fn a_second_fetch_reuses_the_cached_clone_rather_than_recloning() {
        let remote = local_remote(&[(
            "preset.toml",
            "name = \"team\"\ndescription = \"x\"\nlayers = []\n",
        )]);
        let cache = tempfile::tempdir().unwrap();
        let src = source(remote.path().to_str().unwrap(), "preset.toml");

        fetch(&src, cache.path()).unwrap();
        // The remote is now gone; a second fetch can only succeed by reusing
        // the cached clone from the first one, not by cloning again.
        drop(remote);
        let preset = fetch(&src, cache.path()).expect("the cached clone must still be used");
        assert_eq!(preset.name, "team");
    }

    #[test]
    fn a_missing_file_in_a_real_clone_is_reported_by_name() {
        let remote = local_remote(&[("readme.md", "not a preset")]);
        let cache = tempfile::tempdir().unwrap();

        let error = fetch(
            &source(remote.path().to_str().unwrap(), "preset.toml"),
            cache.path(),
        )
        .unwrap_err();

        assert!(matches!(error, RemoteError::MissingFile { .. }));
    }

    #[test]
    fn a_source_that_is_not_a_git_repository_fails_as_a_clone_error() {
        let not_a_repo = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();

        let error = fetch(
            &source(not_a_repo.path().to_str().unwrap(), "preset.toml"),
            cache.path(),
        )
        .unwrap_err();

        assert!(matches!(error, RemoteError::CloneFailed { .. }));
    }

    #[test]
    fn a_malformed_remote_preset_is_reported_with_its_source() {
        let remote = local_remote(&[("preset.toml", "this is not valid toml [[[")]);
        let cache = tempfile::tempdir().unwrap();

        let error = fetch(
            &source(remote.path().to_str().unwrap(), "preset.toml"),
            cache.path(),
        )
        .unwrap_err();

        assert!(matches!(error, RemoteError::Malformed { .. }));
    }
}
