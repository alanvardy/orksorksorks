use crate::config_dir::ConfigPathSource;
use crate::errors::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
    /// Optional name of a `[[scripts]]` entry this step references.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
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

/// A single named script, referenced by `Step::script` and returned by the
/// `script` subcommand.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Script {
    /// Lookup key referenced by `Step::script`.
    pub name: String,
    /// Raw script text (a TOML multi-line string).
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
    /// Named scripts referenced by `Step::script`, returned by the `script`
    /// subcommand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<Script>,
}

/// Serde default path for `Config::show_frontmatter`: frontmatter is
/// shown unless the config explicitly disables it.
fn default_true() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION.to_string(),
            show_frontmatter: true,
            steps: Vec::new(),
            models: Vec::new(),
            prompts: Vec::new(),
            scripts: Vec::new(),
        }
    }
}

/// Read, deserialize, and validate a `Config` from disk.
///
/// Missing/unreadable files map to `"io"` with the resolved path and its
/// resolution source embedded in the message; malformed TOML maps to
/// `"toml::de"` (via `From<toml::de::Error>`), which also covers unknown
/// keys rejected by `deny_unknown_fields`. A well-formed config that fails
/// [`Config::validate`] maps to a `"config:*"` tag instead.
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
    let config: Config = toml::from_str(&contents)?;
    config.validate()?;
    Ok(config)
}

/// The config format version this build reads and writes.
pub const CONFIG_VERSION: &str = "0.1.0";

impl Config {
    /// Validate this config, failing fast with a `config:*` tag on the first
    /// violation in a fixed deterministic order.
    ///
    /// Returns `Ok(())` when the config is consistent, otherwise the first
    /// error: `config:version`, `config:duplicate-name`, `config:empty-name`,
    /// `config:empty-model`, `config:duplicate-trigger`,
    /// `config:multiple-default`, `config:missing-prompt`, or
    /// `config:missing-model`.
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.version != CONFIG_VERSION {
            return Err(Error::new(
                "config:version",
                &format!(
                    "unsupported config version {:?} (expected {:?})",
                    self.version, CONFIG_VERSION
                ),
            ));
        }

        let step_names: Vec<&str> = self.steps.iter().map(|s| s.name.as_str()).collect();
        let model_names: Vec<&str> = self.models.iter().map(|m| m.name.as_str()).collect();
        let prompt_names: Vec<&str> = self.prompts.iter().map(|p| p.name.as_str()).collect();

        if let Some(msg) = duplicated_name(&step_names, "steps")
            .or_else(|| duplicated_name(&prompt_names, "prompts"))
            .or_else(|| duplicated_name(&model_names, "models"))
        {
            return Err(Error::new("config:duplicate-name", &msg));
        }

        if let Some(msg) = empty_name(&step_names, "steps")
            .or_else(|| empty_name(&prompt_names, "prompts"))
            .or_else(|| empty_name(&model_names, "models"))
        {
            return Err(Error::new("config:empty-name", &msg));
        }

        for step in &self.steps {
            if step.model.trim().is_empty() {
                return Err(Error::new(
                    "config:empty-model",
                    &format!("step {:?} has an empty model reference", step.name),
                ));
            }
        }

        let mut triggers: HashSet<&str> = HashSet::new();
        for step in &self.steps {
            if !step.trigger_artifact.is_empty() && !triggers.insert(step.trigger_artifact.as_str())
            {
                return Err(Error::new(
                    "config:duplicate-trigger",
                    &format!(
                        "trigger artifact {:?} is used by more than one step",
                        step.trigger_artifact
                    ),
                ));
            }
        }

        let default_names: Vec<&str> = self
            .steps
            .iter()
            .filter(|s| s.trigger_artifact.is_empty())
            .map(|s| s.name.as_str())
            .collect();
        if default_names.len() > 1 {
            return Err(Error::new(
                "config:multiple-default",
                &format!(
                    "{} steps have an empty trigger artifact (at most one default step is allowed): {}",
                    default_names.len(),
                    default_names.join(", "),
                ),
            ));
        }

        let prompt_set: HashSet<&str> = prompt_names.iter().copied().collect();
        for step in &self.steps {
            if !prompt_set.contains(step.name.as_str()) {
                return Err(Error::new(
                    "config:missing-prompt",
                    &format!("step {:?} has no matching prompt", step.name),
                ));
            }
        }

        let model_set: HashSet<&str> = model_names.iter().copied().collect();
        for step in &self.steps {
            if !model_set.contains(step.model.as_str()) {
                return Err(Error::new(
                    "config:missing-model",
                    &format!(
                        "step {:?} references unknown model {:?}",
                        step.name, step.model
                    ),
                ));
            }
        }

        Ok(())
    }
}

/// Return a message naming the duplicated value, if `names` has a duplicate.
fn duplicated_name(names: &[&str], section: &str) -> Option<String> {
    let mut seen = HashSet::new();
    for &name in names {
        if !seen.insert(name) {
            return Some(format!("duplicate name {name:?} in [{section}]"));
        }
    }
    None
}

/// Return a message identifying `section` if any listed name is blank.
fn empty_name(names: &[&str], section: &str) -> Option<String> {
    if names.iter().any(|name| name.trim().is_empty()) {
        return Some(format!("[{section}] contains an entry with an empty name"));
    }
    None
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
    fn template_deserializes_to_default() {
        let template = include_str!("../templates/default.toml");
        let parsed: Config = toml::from_str(template).unwrap();
        assert_eq!(parsed, Config::default());
    }

    #[test]
    fn template_commented_examples_validate() {
        // The exact TOML the commented examples in templates/default.toml
        // would produce when uncommented. Guards the documentation against
        // schema drift: a renamed field fails deserialization (via
        // deny_unknown_fields), and a broken cross-reference fails
        // validate().
        let uncommented = concat!(
            "version = \"0.1.0\"\n",
            "show_frontmatter = true\n",
            "[[steps]]\n",
            "name = \"example\"\n",
            "trigger_artifact = \"example.md\"\n",
            "model = \"small\"\n",
            "script = \"my_script\"\n",
            "[[models]]\n",
            "name = \"small\"\n",
            "model = \"openrouter/example/model\"\n",
            "thinking = \"high\"\n",
            "[[scripts]]\n",
            "name = \"my_script\"\n",
            "content = \"\"\"\necho \"hello\"\n\"\"\"\n",
            "[[prompts]]\n",
            "name = \"example\"\n",
            "content = \"\"\"\nYour prompt body goes here.\n\"\"\"\n",
        );
        let config: Config = toml::from_str(uncommented).unwrap();
        config.validate().unwrap();
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
                    script: None,
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "b.txt".to_string(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
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
        assert!(config.scripts.is_empty());
    }

    #[test]
    fn read_config_loads_steps_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orksorksorks.toml");
        std::fs::write(
            &path,
            concat!(
                "version = \"0.1.0\"\n",
                "[[steps]]\n",
                "name = \"one\"\n",
                "trigger_artifact = \"a.txt\"\n",
                "model = \"small\"\n",
                "[[models]]\n",
                "name = \"small\"\n",
                "model = \"openrouter/deepseek/flash\"\n",
                "thinking = \"high\"\n",
                "[[prompts]]\n",
                "name = \"one\"\n",
                "content = \"one\"\n",
            ),
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
            scripts: vec![],
        };
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn scripts_round_trip() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![],
            scripts: vec![Script {
                name: "run-one".to_string(),
                content: "#!/usr/bin/env bash\n\necho hello\n".to_string(),
            }],
        };
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn step_script_deserializes_to_none() {
        let config: Config = toml::from_str(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"a.txt\"\n",
            "model = \"small\"\n",
        ))
        .unwrap();
        assert_eq!(config.steps[0].script, None);
    }

    #[test]
    fn step_script_deserializes_to_some() {
        let config: Config = toml::from_str(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"a.txt\"\n",
            "model = \"small\"\n",
            "script = \"run-one\"\n",
        ))
        .unwrap();
        assert_eq!(config.steps[0].script.as_deref(), Some("run-one"));
    }

    #[test]
    fn step_script_empty_string_deserializes_to_some_empty() {
        let config: Config = toml::from_str(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"a.txt\"\n",
            "model = \"small\"\n",
            "script = \"\"\n",
        ))
        .unwrap();
        assert_eq!(config.steps[0].script, Some(String::new()));
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
            scripts: vec![],
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
            scripts: vec![],
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

    fn validate_toml(s: &str) -> Result<(), Error> {
        toml::from_str::<Config>(s).unwrap().validate()
    }

    /// A minimal consistent config used as the happy-path baseline.
    const VALID_TOML: &str = concat!(
        "version = \"0.1.0\"\n",
        "[[steps]]\n",
        "name = \"one\"\n",
        "trigger_artifact = \"a.txt\"\n",
        "model = \"small\"\n",
        "[[models]]\n",
        "name = \"small\"\n",
        "model = \"openrouter/deepseek/flash\"\n",
        "thinking = \"high\"\n",
        "[[prompts]]\n",
        "name = \"one\"\n",
        "content = \"one\"\n",
    );

    #[test]
    fn validate_accepts_consistent_config() {
        assert!(validate_toml(VALID_TOML).is_ok());
    }

    #[test]
    fn validate_accepts_unreferenced_prompts_and_models() {
        // Orphan entries are allowed by design (membership-only cross-check):
        // a model no step references and a prompt with no matching step name
        // must not fail validation (e.g. prompts used via `prompt <name>`).
        let result = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[models]]\nname = \"unused\"\nmodel = \"openrouter/deepseek/pro\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
            "[[prompts]]\nname = \"orphan\"\ncontent = \"orphan\"\n",
        ));
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn validate_rejects_wrong_version() {
        let err = validate_toml("version = \"9.9.9\"\n").unwrap_err();
        assert_eq!(err.source, "config:version");
    }

    #[test]
    fn validate_rejects_duplicate_step_name() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"b.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:duplicate-name");
    }

    #[test]
    fn validate_rejects_duplicate_prompt_name() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"two\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:duplicate-name");
    }

    #[test]
    fn validate_rejects_duplicate_model_name() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/pro\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:duplicate-name");
    }

    #[test]
    fn validate_rejects_empty_step_name() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:empty-name");
    }

    #[test]
    fn validate_rejects_whitespace_only_step_name() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"   \"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:empty-name");
    }

    #[test]
    fn validate_rejects_empty_model_reference() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:empty-model");
    }

    #[test]
    fn validate_rejects_whitespace_only_model_reference() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"   \"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:empty-model");
    }

    #[test]
    fn validate_rejects_duplicate_trigger() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[steps]]\nname = \"two\"\ntrigger_artifact = \"a.txt\"\nmodel = \"high\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[models]]\nname = \"high\"\nmodel = \"openrouter/deepseek/pro\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
            "[[prompts]]\nname = \"two\"\ncontent = \"two\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:duplicate-trigger");
    }

    #[test]
    fn validate_rejects_multiple_defaults() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"\"\nmodel = \"small\"\n",
            "[[steps]]\nname = \"two\"\ntrigger_artifact = \"\"\nmodel = \"high\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[models]]\nname = \"high\"\nmodel = \"openrouter/deepseek/pro\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
            "[[prompts]]\nname = \"two\"\ncontent = \"two\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:multiple-default");
    }

    #[test]
    fn validate_rejects_missing_prompt() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:missing-prompt");
    }

    #[test]
    fn validate_rejects_missing_model() {
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"nope\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:missing-model");
    }

    #[test]
    fn validate_errors_are_deterministic_first_error_wins() {
        // Violates BOTH duplicate-name and missing-prompt: the earlier rule
        // (duplicate-name) must win deterministically.
        let err = validate_toml(concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"b.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
        ))
        .unwrap_err();
        assert_eq!(err.source, "config:duplicate-name");
    }

    #[test]
    fn read_config_rejects_missing_prompt_with_config_tag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orksorksorks.toml");
        std::fs::write(
            &path,
            concat!(
                "version = \"0.1.0\"\n",
                "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
                "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            ),
        )
        .unwrap();
        let err = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap_err();
        assert_eq!(err.source, "config:missing-prompt");
    }

    #[test]
    fn read_config_rejects_missing_model_with_config_tag() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orksorksorks.toml");
        std::fs::write(
            &path,
            concat!(
                "version = \"0.1.0\"\n",
                "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"nope\"\n",
                "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
            ),
        )
        .unwrap();
        let err = read_config(&path, ConfigPathSource::ExplicitFlag).unwrap_err();
        assert_eq!(err.source, "config:missing-model");
    }
}
