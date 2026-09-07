use serde::{Deserialize, Serialize};

/// A single named step, gated on the presence of a trigger artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Step {
    /// Human-readable name returned by the `step` subcommand.
    pub name: String,
    /// Filename whose presence at the artifact directory marks this step.
    pub trigger_artifact: String,
}

/// Application configuration, serialized as TOML in `orksorksorks.toml`.
///
/// The `version` field tracks the config format version so future
/// migrations can detect and upgrade older files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Config format version.
    pub version: String,
    /// Ordered steps; the current step is the last whose artifact exists.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<Step>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            steps: Vec::new(),
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

    #[test]
    fn steps_round_trip() {
        let config = Config {
            version: "0.1.0".to_string(),
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "a.txt".to_string(),
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "b.txt".to_string(),
                },
            ],
        };
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn missing_steps_deserializes_to_empty_vec() {
        let config: Config = toml::from_str("version = \"0.1.0\"\n").unwrap();
        assert!(config.steps.is_empty());
    }
}
