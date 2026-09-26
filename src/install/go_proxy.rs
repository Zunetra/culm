// === The Go module proxy

/* The newest version of a module old enough for the release age guard.
 * `@v/list` names every tagged version but not when it was published, and
 * each `@v/<version>.info` answers that for one version, so this walks from
 * the newest version down and stops at the first one old enough rather than
 * asking about every version. Not covered by `cargo test`, the same as
 * `maven_central`'s network path. */

use semver::{Version, VersionReq};
use serde::Deserialize;

use super::release_age::unix_seconds;

const PROXY: &str = "https://proxy.golang.org";

#[derive(Deserialize)]
struct Info {
    #[serde(rename = "Time")]
    time: String,
}

/// The chosen version, without its `v`, or `None` when nothing qualifies.
pub fn choose(
    module: &str,
    range: Option<&VersionReq>,
    min_age_secs: i64,
    now: i64,
) -> Result<Option<Version>, String> {
    let base = format!("{PROXY}/{}/@v", escape(module));
    let list = get(&format!("{base}/list"), module)?;

    for version in candidates(&list, range) {
        let info: Info = serde_json::from_str(&get(&format!("{base}/v{version}.info"), module)?)
            .map_err(|e| format!("the Go proxy sent an unreadable answer for `{module}`: {e}"))?;
        let published = unix_seconds(&info.time)
            .ok_or_else(|| format!("the Go proxy sent an unreadable time for `{module}`"))?;
        if now - published >= min_age_secs {
            return Ok(Some(version));
        }
    }
    Ok(None)
}

/// Every listed release version inside `range`, newest first. Pre-releases
/// are never chosen.
fn candidates(list: &str, range: Option<&VersionReq>) -> Vec<Version> {
    let mut versions: Vec<Version> = list
        .lines()
        .filter_map(|line| Version::parse(line.trim().strip_prefix('v')?).ok())
        .filter(|version| version.pre.is_empty())
        .filter(|version| range.is_none_or(|range| range.matches(version)))
        .collect();
    versions.sort();
    versions.reverse();
    versions
}

/// The proxy's case encoding: each capital letter becomes `!` and its lower
/// case, since module paths are case sensitive and some filesystems are not.
fn escape(module: &str) -> String {
    let mut escaped = String::with_capacity(module.len());
    for c in module.chars() {
        if c.is_ascii_uppercase() {
            escaped.push('!');
            escaped.push(c.to_ascii_lowercase());
        } else {
            escaped.push(c);
        }
    }
    escaped
}

fn get(url: &str, module: &str) -> Result<String, String> {
    ureq::get(url)
        .call()
        .map_err(|e| format!("cannot reach the Go proxy for `{module}`: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("the Go proxy sent an unreadable answer for `{module}`: {e}"))
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitals_are_escaped_the_way_the_proxy_expects() {
        assert_eq!(escape("github.com/BurntSushi/toml"), "github.com/!burnt!sushi/toml");
        assert_eq!(escape("github.com/go-chi/chi/v5"), "github.com/go-chi/chi/v5");
    }

    #[test]
    fn candidates_are_newest_first_without_pre_releases() {
        let list = "v5.0.10\nv5.1.0\nv5.2.0-rc.1\nv5.0.9\n";
        let found: Vec<String> = candidates(list, None).iter().map(Version::to_string).collect();
        assert_eq!(found, vec!["5.1.0", "5.0.10", "5.0.9"]);
    }

    #[test]
    fn a_range_narrows_the_candidates() {
        let range = VersionReq::parse("<5.1").unwrap();
        let found: Vec<String> =
            candidates("v5.0.10\nv5.1.0\n", Some(&range)).iter().map(Version::to_string).collect();
        assert_eq!(found, vec!["5.0.10"]);
    }
}
