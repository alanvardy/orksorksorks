use serde::{Deserialize, Serialize};

/// Application configuration, serialized as TOML in `orksorksorks.toml`.
///
/// The `version` field tracks the config format version so future
/// migrations can detect and upgrade older files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Config format version.
    pub version: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn default_config_serializes_to_expected_toml() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("version"), "{toml_str}");
        assert!(toml_str.contains(r#""0.1.0""#), "{toml_str}");
    }

    #[test]
    fn config_round_trip_serialize_deserialize() {
        let config = Config::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
