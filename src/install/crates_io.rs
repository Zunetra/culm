// === crates.io

/* Every published version of a crate, with its publish time and whether it
 * was yanked, for the release age guard. The API rather than the sparse
 * index, because the index carries no dates. Not covered by `cargo test`,
 * the same as `maven_central`'s network path. */

use serde::Deserialize;

use super::release_age::{Release, unix_seconds};

const API: &str = "https://crates.io/api/v1/crates";

#[derive(Deserialize)]
struct Page {
    versions: Vec<Entry>,
    meta: Meta,
}

#[derive(Deserialize)]
struct Entry {
    num: String,
    yanked: bool,
    created_at: String,
}

#[derive(Deserialize)]
struct Meta {
    next_page: Option<String>,
}

/// Every version of `name` crates.io knows, across every page.
pub fn releases(name: &str) -> Result<Vec<Release>, String> {
    let mut releases = Vec::new();
    let mut url = format!("{API}/{name}/versions?per_page=100");
    loop {
        let page: Page = ureq::get(&url)
            /* crates.io refuses requests without an identifying agent. */
            .header(
                "User-Agent",
                concat!(
                    "culm/",
                    env!("CARGO_PKG_VERSION"),
                    " (https://github.com/Zunetra/culm)"
                ),
            )
            .call()
            .map_err(|e| format!("cannot reach crates.io for `{name}`: {e}"))?
            .body_mut()
            .read_json()
            .map_err(|e| format!("crates.io sent an unreadable answer for `{name}`: {e}"))?;
        releases.extend(parse(&page.versions));
        match page.meta.next_page {
            Some(next) => url = format!("{API}/{name}/versions{next}"),
            None => return Ok(releases),
        }
    }
}

/// Entries that do not parse are left out rather than failing the lookup:
/// an unparseable version cannot be chosen anyway.
fn parse(entries: &[Entry]) -> Vec<Release> {
    entries
        .iter()
        .filter_map(|entry| {
            Some(Release {
                version: semver::Version::parse(&entry.num).ok()?,
                published: unix_seconds(&entry.created_at)?,
                yanked: entry.yanked,
            })
        })
        .collect()
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_of_versions_parses_and_bad_entries_are_dropped() {
        let body = r#"{
            "versions": [
                {"num": "1.40.0", "yanked": false, "created_at": "2024-09-01T10:00:00.000000+00:00"},
                {"num": "1.39.9", "yanked": true, "created_at": "2024-08-01T10:00:00.000000+00:00"},
                {"num": "not-semver", "yanked": false, "created_at": "2024-08-01T10:00:00+00:00"}
            ],
            "meta": {"next_page": null}
        }"#;
        let page: Page = serde_json::from_str(body).unwrap();
        let releases = parse(&page.versions);
        assert_eq!(releases.len(), 2);
        assert!(releases[1].yanked);
        assert!(page.meta.next_page.is_none());
    }
}
