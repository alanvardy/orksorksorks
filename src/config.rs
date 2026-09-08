use crate::config_dir::ConfigPathSource;
use crate::errors::Error;
use serde::{Deserialize, Serialize};

/// A single named step, gated on the presence of a trigger artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Step {
    /// Human-readable name returned by the `step` subcommand.
    pub name: String,
    /// Filename whose presence at the artifact directory marks this step.
    pub trigger_artifact: String,
    /// Name of the model (a key into `Config::models`) to use at this step.
    pub model: String,
}

/// A named model definition, referenced by `Step::model`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// Key that `Step::model` references to select this model.
    pub name: String,
    /// Concrete model identifier (e.g. `openrouter/...`).
    pub model: String,
    /// Reasoning-budget hint for the model.
    pub thinking: String,
}

/// A single named prompt, keyed by step name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Prompt {
    /// Step name this prompt belongs to; also the lookup key.
    pub name: String,
    /// Full prompt text (a TOML multi-line string).
    pub content: String,
}

/// Application configuration, serialized as TOML in `orksorksorks.toml`.
///
/// The `version` field tracks the config format version so future
/// migrations can detect and upgrade older files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Config format version.
    pub version: String,
    /// Whether `prompt` output is prefixed with a context frontmatter block.
    #[serde(default = "default_true")]
    pub show_frontmatter: bool,
    /// Ordered steps; the current step is the last whose artifact exists.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<Step>,
    /// Named models referenced by `Step::model`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<Model>,
    /// Named prompts keyed by step name, returned by the `prompt` subcommand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<Prompt>,
}

/// Serde default path for `Config::show_frontmatter`: frontmatter is
/// shown unless the config explicitly disables it.
fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: Vec::new(),
            models: Vec::new(),
            prompts: Vec::new(),
        }
    }
}

/// Read and deserialize a `Config` from disk.
///
/// Missing/unreadable files map to `"io"` with the resolved path and its
/// resolution source embedded in the message; malformed TOML maps to
/// `"toml::de"` (via `From<toml::de::Error>`).
pub fn read_config(path: &std::path::Path, source: ConfigPathSource) -> Result<Config, Error> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            return Err(Error::new(
                "io",
                &format!(
                    "could not read config file at {} ({}): {}",
                    path.display(),
                    source,
                    e
                ),
            ));
        }
    };
    let config = toml::from_str(&contents)?;
    Ok(config)
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
        assert!(toml_str.contains("show_frontmatter = true"), "{toml_str}");
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
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "a.txt".to_string(),
                    model: "small".to_string(),
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "b.txt".to_string(),
                    model: "high".to_string(),
                },
            ],
            models: vec![],
            prompts: vec![],
        };
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn missing_steps_deserializes_to_empty_vec() {
        let config: Config = toml::from_str("version = \"0.1.0\"\n").unwrap();
        assert!(config.show_frontmatter);
        assert!(config.steps.is_empty());
        assert!(config.models.is_empty());
        assert!(config.prompts.is_empty());
    }

    #[test]
    fn read_config_loads_steps_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orksorksorks.toml");
        std::fs::write(
            &path,
            "version = \"0.1.0\"\n[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
        )
        .unwrap();
        let config = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap();
        assert_eq!(config.steps.len(), 1);
        assert_eq!(config.steps[0].name, "one");
        assert_eq!(config.steps[0].trigger_artifact, "a.txt");
        assert_eq!(config.steps[0].model, "small");
    }

    #[test]
    fn prompts_round_trip() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![Prompt {
                name: "questions".to_string(),
                content: "# Question — Decompose the Task\n\nSome body.\n".to_string(),
            }],
        };
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn read_config_loads_prompts_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orksorksorks.toml");
        std::fs::write(
            &path,
            concat!(
                "version = \"0.1.0\"\n",
                "[[prompts]]\n",
                "name = \"questions\"\n",
                "content = \"\"\"\n",
                "# Question — Decompose the Task\n",
                "\"\"\"\n",
            ),
        )
        .unwrap();
        let config = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap();
        assert_eq!(config.prompts.len(), 1);
        assert_eq!(config.prompts[0].name, "questions");
        assert!(
            config.prompts[0].content.contains("Decompose the Task"),
            "{}",
            config.prompts[0].content
        );
    }

    #[test]
    fn models_round_trip() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![Model {
                name: "small".to_string(),
                model: "openrouter/deepseek/flash".to_string(),
                thinking: "high".to_string(),
            }],
            prompts: vec![],
        };
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn show_frontmatter_false_round_trips() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: false,
            steps: vec![],
            models: vec![],
            prompts: vec![],
        };
        let serialized = toml::to_string(&config).unwrap();
        assert!(
            serialized.contains("show_frontmatter = false"),
            "{serialized}"
        );
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn read_config_missing_file_tags_io() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.toml");
        let err = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap_err();
        assert_eq!(err.source, "io");
        assert!(
            err.message.contains(&path.display().to_string()),
            "message should contain path: {}",
            err.message
        );
        assert!(
            err.message.contains("specified via --config"),
            "message should contain resolution: {}",
            err.message
        );
    }

    #[test]
    fn read_config_malformed_toml_tags_toml_de() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orksorksorks.toml");
        // Missing the required `name` field on the step
        std::fs::write(
            &path,
            "version = \"0.1.0\"\n[[steps]]\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
        )
        .unwrap();
        let err = read_config(&path, ConfigPathSource::XdgConfigHome).unwrap_err();
        assert_eq!(err.source, "toml::de");
    }

    #[test]
    fn read_config_io_error_contains_xdg_source() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orksorksorks.toml");
        let err = read_config(&path, ConfigPathSource::XdgConfigHome).unwrap_err();
        assert_eq!(err.source, "io");
        assert!(err.message.contains("resolved from XDG_CONFIG_HOME"));
        assert!(err.message.contains(&path.display().to_string()));
    }

    #[test]
    fn unknown_step_key_is_rejected_as_toml_de() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orksorksorks.toml");
        std::fs::write(
            &path,
            "version = \"0.1.0\"\n[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\nbogus = \"x\"\n",
        )
        .unwrap();
        let err = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap_err();
        assert_eq!(err.source, "toml::de");
    }
}
