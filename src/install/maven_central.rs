// === Resolving a JVM dependency's latest version

// Decision 5: no template names a version. Neither Gradle nor Maven has a
// first-party "add a dependency" verb, so this asks Maven Central directly,
// via the metadata endpoint rather than the lagging search index.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum MavenCentralError {
    #[error("could not reach Maven Central for {group}:{artifact}: {message}")]
    Unreachable {
        group: String,
        artifact: String,
        message: String,
    },

    #[error("Maven Central has no metadata for {group}:{artifact} (HTTP {status})")]
    NotFound {
        group: String,
        artifact: String,
        status: u16,
    },

    #[error("Maven Central's metadata for {group}:{artifact} has no <release> tag")]
    NoRelease { group: String, artifact: String },
}

/// Where `maven-metadata.xml` lives for a coordinate.
pub fn metadata_url(group: &str, artifact: &str) -> String {
    let path = group.replace('.', "/");
    format!("https://repo1.maven.org/maven2/{path}/{artifact}/maven-metadata.xml")
}

/// Pulls `<release>` out of a `maven-metadata.xml` body. Hand-rolled rather
/// than a dependency, for one tag.
pub fn parse_release(xml: &str) -> Option<&str> {
    let start = xml.find("<release>")? + "<release>".len();
    let end = xml[start..].find("</release>")? + start;
    Some(xml[start..end].trim())
}

/// Resolves `group:artifact` to its latest released version. Not covered by
/// `cargo test`, same as `preset::remote::fetch`'s network path.
pub fn latest_version(group: &str, artifact: &str) -> Result<String, MavenCentralError> {
    let url = metadata_url(group, artifact);
    let unreachable = |message: String| MavenCentralError::Unreachable {
        group: group.to_owned(),
        artifact: artifact.to_owned(),
        message,
    };

    let mut response = ureq::get(&url)
        .call()
        .map_err(|e| unreachable(e.to_string()))?;

    let status = response.status().as_u16();
    if status != 200 {
        return Err(MavenCentralError::NotFound {
            group: group.to_owned(),
            artifact: artifact.to_owned(),
            status,
        });
    }

    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| unreachable(e.to_string()))?;

    parse_release(&body)
        .map(str::to_owned)
        .ok_or_else(|| MavenCentralError::NoRelease {
            group: group.to_owned(),
            artifact: artifact.to_owned(),
        })
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_group_id_s_dots_become_path_segments() {
        assert_eq!(
            metadata_url("org.springframework.boot", "spring-boot-starter-web"),
            "https://repo1.maven.org/maven2/org/springframework/boot/spring-boot-starter-web/maven-metadata.xml"
        );
    }

    #[test]
    fn a_release_tag_is_extracted_and_trimmed() {
        let xml = "<metadata>\n  <versioning>\n    <release>  3.3.4  </release>\n  </versioning>\n</metadata>";
        assert_eq!(parse_release(xml), Some("3.3.4"));
    }

    #[test]
    fn a_missing_release_tag_yields_none() {
        let xml = "<metadata><versioning><latest>3.3.4</latest></versioning></metadata>";
        assert_eq!(parse_release(xml), None);
    }

    #[test]
    fn an_unclosed_release_tag_yields_none() {
        let xml = "<metadata><versioning><release>3.3.4</metadata>";
        assert_eq!(parse_release(xml), None);
    }
}
