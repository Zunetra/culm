// === The exact payload

/* Every field decision 9 names, and nothing it forbids: no repository name, no
 * file path, no task name, no environment variable value, no command string,
 * no git remote. Adding a field here is a decision 9 change, not a one-line
 * patch, which is why the struct has no room for anything else to ride along. */

use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: Uuid,
    pub version: &'static str,
    pub os: &'static str,
    pub arch: &'static str,
    pub command: String,
    pub layers: Vec<String>,
    pub preset: Option<String>,
}

impl Event {
    pub fn new(command: &str, id: Uuid, layers: &[String], preset: Option<&str>) -> Self {
        Self {
            id,
            version: env!("CARGO_PKG_VERSION"),
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            command: command.to_owned(),
            layers: layers.to_vec(),
            preset: preset.map(str::to_owned),
        }
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_payload_carries_exactly_what_decision_9_names() {
        let id = Uuid::new_v4();
        let event = Event::new("new", id, &["frontend/nuxt".to_owned()], Some("monorepo"));

        let value = serde_json::to_value(&event).unwrap();
        let object = value.as_object().unwrap();
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort();

        assert_eq!(
            keys,
            vec!["arch", "command", "id", "layers", "os", "preset", "version"]
        );
    }
}
