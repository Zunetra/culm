// === Remote cache client (r0.4)

/* docs/DESIGN.md section 3.5's r0.4: a thin HTTP client, no server
 * component ships from this repository. It uploads and downloads `cas`
 * objects by the same blake3 digest `cas::Store` already addresses them by
 * on disk, so the wire format is exactly the local format, nothing extra to
 * define. Section 7's open-core line draws the paid boundary at *hosting*
 * this, never at the client talking to it: a single team can point this at
 * their own bucket or thin store and pay nothing. */

use std::io::Read;

use thiserror::Error;

use crate::cas::Hash;

#[derive(Debug, Error)]
pub enum RemoteCacheError {
    #[error("could not reach the remote cache at {url}: {message}")]
    Unreachable { url: String, message: String },

    #[error("the remote cache at {url} rejected the request (HTTP {status})")]
    Rejected { url: String, status: u16 },
}

/// One remote cache endpoint. A cache miss against it must be exactly as
/// safe as a local cache miss, so every method here returns `Ok(false)` or
/// an ordinary error rather than panicking on an unreachable network; the
/// caller always still has the option of running the task for real.
#[derive(Debug, Clone)]
pub struct RemoteCache {
    base_url: String,
    token: Option<String>,
}

impl RemoteCache {
    /// Reads `CULM_REMOTE_CACHE_URL` and an optional `CULM_REMOTE_CACHE_TOKEN`,
    /// the same environment-variable convention `telemetry`'s own endpoint
    /// override already uses. `None` when no remote cache is configured,
    /// which is the ordinary case until a team opts in.
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("CULM_REMOTE_CACHE_URL").ok()?;
        let token = std::env::var("CULM_REMOTE_CACHE_TOKEN").ok();
        Some(RemoteCache { base_url, token })
    }

    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        RemoteCache {
            base_url: base_url.into(),
            token,
        }
    }

    fn object_url(&self, digest: &Hash) -> String {
        format!(
            "{}/objects/{}",
            self.base_url.trim_end_matches('/'),
            digest.to_hex()
        )
    }

    /// Downloads the object named `digest`, or `Ok(None)` if the remote
    /// cache does not have it. A miss here is not an error: the caller falls
    /// back to the local cache, then to running the task for real.
    pub fn download(&self, digest: &Hash) -> Result<Option<Vec<u8>>, RemoteCacheError> {
        let url = self.object_url(digest);
        let mut request = ureq::get(&url);
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let mut response = match request.call() {
            Ok(response) => response,
            Err(_) => return Ok(None),
        };

        let status = response.status().as_u16();
        if status == 404 {
            return Ok(None);
        }
        if status != 200 {
            return Err(RemoteCacheError::Rejected { url, status });
        }

        let mut bytes = Vec::new();
        response
            .body_mut()
            .as_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| RemoteCacheError::Unreachable {
                url,
                message: e.to_string(),
            })?;
        Ok(Some(bytes))
    }

    /// Uploads `bytes` under `digest`. Best-effort by the caller's own
    /// choice, never by this function: a failed upload here is a real error,
    /// because a "successful" push that silently did nothing is exactly the
    /// class of wrong-cache-hit failure docs/DESIGN.md section 3.1 exists
    /// to prevent, just on the write side instead of the read side.
    pub fn upload(&self, digest: &Hash, bytes: &[u8]) -> Result<(), RemoteCacheError> {
        let url = self.object_url(digest);
        let mut request = ureq::put(&url);
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request
            .send(bytes)
            .map_err(|e| RemoteCacheError::Unreachable {
                url: url.clone(),
                message: e.to_string(),
            })?;

        let status = response.status().as_u16();
        if status != 200 && status != 201 && status != 204 {
            return Err(RemoteCacheError::Rejected { url, status });
        }
        Ok(())
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_url_is_namespaced_under_objects_by_hex_digest() {
        let cache = RemoteCache::new("https://cache.example.com", None);
        let digest = crate::cas::hash(b"anything");
        let url = cache.object_url(&digest);
        assert!(url.starts_with("https://cache.example.com/objects/"));
        assert!(url.ends_with(&digest.to_hex().to_string()));
    }

    #[test]
    fn a_trailing_slash_on_the_base_url_is_not_doubled() {
        let cache = RemoteCache::new("https://cache.example.com/", None);
        let digest = crate::cas::hash(b"anything");
        let url = cache.object_url(&digest);
        assert!(!url.contains("//objects"));
    }
}
