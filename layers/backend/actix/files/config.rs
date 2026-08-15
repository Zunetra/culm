// === Configuration

use std::env;

use thiserror::Error;

/// Everything the service reads from the environment.
/// Resolved once at startup rather than at each use, so a misconfigured deployment fails immediately instead of on the first request that happens to need a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("{name} is set to `{value}`, which is not a {expected}")]
    Invalid {
        name: &'static str,
        value: String,
        expected: &'static str,
    },
}

impl Config {
    /// Reads the environment, falling back to values that work on a developer's machine.
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_owned()),
            port: match env::var("PORT") {
                Ok(value) => value.parse().map_err(|_| ConfigError::Invalid {
                    name: "PORT",
                    value,
                    expected: "port number",
                })?,
                Err(_) => 3000,
            },
        })
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_port_that_is_not_a_number_is_refused() {
        assert_eq!(
            "eighty".parse::<u16>().map_err(|_| ConfigError::Invalid {
                name: "PORT",
                value: "eighty".to_owned(),
                expected: "port number",
            }),
            Err(ConfigError::Invalid {
                name: "PORT",
                value: "eighty".to_owned(),
                expected: "port number",
            })
        );
    }
}
