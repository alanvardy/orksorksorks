use crate::config::{Config, Step};
use crate::errors::Error;
use crate::git;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

const NAME: &str = env!("CARGO_PKG_NAME");
const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "\nbuild target: ",
    env!("BUILD_TARGET"),
    "\nbuild profile: ",
    env!("BUILD_PROFILE"),
    "\nbuild timestamp: ",
    env!("BUILD_TIMESTAMP"),
);

/// CLI argument root.
#[derive(Parser, Clone)]
#[command(
    name = NAME,
    author = AUTHOR,
    version = LONG_VERSION,
    about = ABOUT,
    long_about = None
)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    /// Output results as JSON
    #[arg(short = 'j', long, global = true, default_value_t = false)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// (i) Create a new orksorksorks.toml file
    Init {
        /// Write to this path instead of the default config directory
        #[arg(short = 'c', long, value_parser = clap::value_parser!(PathBuf))]
        config: Option<PathBuf>,
    },

    /// Print the current git branch
    Branch,

    /// Print the artifact directory path (cwd/.pi/orksorksorks/<branch>/)
    #[command(name = "artifact_directory")]
    ArtifactDirectory,

    /// Determine the current step from present trigger artifacts
    Step {
        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,
    },

    /// Print the model for the current step (from `config.models`)
    Model {
        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,
    },

    /// Print the thinking budget for the current step (from `config.models`)
    Thinking {
        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,
    },

    /// Print the prompt for the current step (from `config.prompts`)
    Prompt {
        /// Step name override; defaults to the current step derived from
        /// present trigger artifacts
        #[arg(value_name = "STEP_NAME")]
        step_name: Option<String>,

        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,
    },
}

/// Route a parsed CLI to its handler, injecting `env` for config-dir resolution.
fn select_command_with_env(cli: &Cli, env: &crate::config_dir::ConfigEnv) -> Result<String, Error> {
    match &cli.command {
        Commands::Init { config } => {
            let (path, _) = crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            init_command(&path)
        }
        Commands::Branch => branch_command(),
        Commands::ArtifactDirectory => artifact_directory_command(),
        Commands::Step { config } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            step_command(&path, source)
        }
        Commands::Model { config } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            model_command(&path, source)
        }
        Commands::Thinking { config } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            thinking_command(&path, source)
        }
        Commands::Prompt { step_name, config } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            prompt_command(&path, source, step_name.clone())
        }
    }
}

/// Route a parsed CLI to its handler and return a success message or error.
pub fn select_command(cli: &Cli) -> Result<String, Error> {
    select_command_with_env(cli, &crate::config_dir::ConfigEnv::from_env())
}

/// Create a new `orksorksorks.toml` file with default configuration.
fn init_command(path: &std::path::Path) -> Result<String, Error> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let config = crate::config::Config::default();
    let toml_str = toml::to_string(&config)?;
    let mut file = std::fs::File::create(path)?;
    file.write_all(toml_str.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(crate::format::green_string(&format!(
        "✓ Created {}",
        path.display()
    )))
}

/// Compose the artifact-directory path from a cwd and a branch name.
///
/// Slashes in the branch name are normalized to hyphens so the branch is a
/// single path segment. Pure — no git, no I/O, no error path. The trailing
/// slash is part of the contract (see `task.md`).
fn artifact_dir_path(cwd: &std::path::Path, branch: &str) -> String {
    format!(
        "{}/.pi/orksorksorks/{}/",
        cwd.display(),
        branch.replace('/', "-")
    )
}

/// Handle the `branch` subcommand: return the current git branch, plain.
fn branch_command() -> Result<String, Error> {
    git::current_branch()
}

/// Handle the `artifact_directory` subcommand: return
/// `$PWD/.pi/orksorksorks/<branch>/`, plain (no directory creation).
fn artifact_directory_command() -> Result<String, Error> {
    let cwd = std::env::current_dir()?;
    Ok(artifact_dir_path(&cwd, &git::current_branch()?))
}

/// Reverse-iterate steps and return the *matched step* (not just its name),
/// so callers like `step` and `model` can read different fields (`name` vs
/// `model`). A step with an empty
/// `trigger_artifact` is a default: it never matches by existence, but is
/// returned instead of erroring when no other step's artifact exists (the
/// last such step in the list, in reverse priority, wins). `artifact_dir`
/// must end in a trailing slash — the same string-composition convention
/// as `artifact_dir_path`.
fn determine_step(config: &Config, artifact_dir: &str) -> Result<Step, Error> {
    let mut default = None;
    for step in config.steps.iter().rev() {
        if step.trigger_artifact.is_empty() {
            // Empty trigger = default step; never used while a real match
            // is possible, only as fallback. First seen in reverse = last
            // forward, which wins (reverse-priority convention).
            default.get_or_insert_with(|| step.clone());
            continue;
        }
        let path = format!("{artifact_dir}{}", step.trigger_artifact);
        if std::path::Path::new(&path).try_exists()? {
            return Ok(step.clone());
        }
    }
    if let Some(step) = default {
        return Ok(step);
    }
    Err(Error::new(
        "step",
        &format!("{artifact_dir}: no trigger artifact matched"),
    ))
}

/// Look up the named model (the current step's `model` reference) in
/// `config.models` and return the matching entry, so callers like `model`
/// and `thinking` can read different fields (`model` vs `thinking`).
fn resolve_model(config: &Config, name: &str) -> Result<crate::config::Model, Error> {
    for m in config.models.iter() {
        if m.name == name {
            return Ok(m.clone());
        }
    }
    Err(Error::new("model", &format!("no model named {name:?}")))
}

/// Handle the `step` subcommand: read the config, derive the artifact
/// directory from cwd + git branch, and return the current step name.
///
/// `path` is the already-resolved config location: either the explicit
/// `--config` argument or the config-directory default (`config_dir.rs`).
/// `read_config` runs before git resolution so a missing/unreadable config
/// deterministically fails with `"io"`.
fn step_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let cwd = std::env::current_dir()?;
    let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
    let step = determine_step(&cfg, &artifact_dir)?;
    Ok(step.name)
}

/// Handle the `model` subcommand: determine the current step, read the
/// model *name* it references, and resolve that name against `config.models`
/// to the concrete model string.
fn model_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let cwd = std::env::current_dir()?;
    let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
    let step = determine_step(&cfg, &artifact_dir)?;
    let model = resolve_model(&cfg, &step.model)?;
    Ok(model.model)
}

/// Handle the `thinking` subcommand: determine the current step, read the
/// model *name* it references, and resolve that name against `config.models`
/// to its thinking-budget value.
fn thinking_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let cwd = std::env::current_dir()?;
    let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
    let step = determine_step(&cfg, &artifact_dir)?;
    let model = resolve_model(&cfg, &step.model)?;
    Ok(model.thinking)
}

/// Look up the named prompt (a `config.prompts` key, usually a step name)
/// and return its content.
fn resolve_prompt(config: &Config, name: &str) -> Result<String, Error> {
    for p in config.prompts.iter() {
        if p.name == name {
            return Ok(p.content.clone());
        }
    }
    Err(Error::new("prompt", &format!("no prompt named {name:?}")))
}

/// Handle the `prompt` subcommand: read the config and return the prompt
/// content for the current step, or for the explicitly named step when
/// `step_name` is provided as an override. Output is prefixed with a
/// frontmatter block (important-variables header, step, branch, artifact
/// directory) above the prompt content unless `show_frontmatter = false`
/// in the config.
fn prompt_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step_name: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let name = if let Some(name) = step_name {
        name
    } else {
        // No explicit step: derive the current step from trigger artifacts,
        // exactly like `step`/`model`/`thinking`.
        let cwd = std::env::current_dir()?;
        let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
        determine_step(&cfg, &artifact_dir)?.name
    };
    let content = resolve_prompt(&cfg, &name)?;

    // Frontmatter carries the run context (step, branch, artifact dir) above
    // the prompt output unless disabled by `show_frontmatter = false`.
    // Branch/artifact resolution is deferred until after the prompt resolves
    // so an unknown prompt keeps failing with its `"prompt"` error instead
    // of a git error. `step` is the effective prompt name (the explicit
    // override when given, else the derived step).
    if cfg.show_frontmatter {
        let cwd = std::env::current_dir()?;
        let branch = git::current_branch()?;
        let artifact_dir = artifact_dir_path(&cwd, &branch);
        return Ok(format!(
            "## Important variables\nUse these everywhere you see $<variable>\nstep = {}\nbranch = {}\nartifact_directory = {}\n\n{}",
            name, branch, artifact_dir, content
        ));
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Step};
    use clap::CommandFactory;

    #[test]
    fn select_command_routes_init() {
        let temp = tempfile::tempdir().unwrap();
        let env = crate::config_dir::ConfigEnv {
            xdg_config_home: Some(temp.path().into()),
            ..Default::default()
        };
        let cli = Cli {
            json: false,
            command: Commands::Init { config: None },
        };
        let result = select_command_with_env(&cli, &env).unwrap();
        let expected_path = temp.path().join("orksorksorks.toml");
        assert!(expected_path.exists(), "config file not created");
        // Under cfg!(test) color is stripped
        assert_eq!(result, format!("✓ Created {}", expected_path.display()));
    }

    #[test]
    fn cli_try_parse_rejects_no_subcommand() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_try_parse_accepts_init() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "init"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_accepts_config_short() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "init", "-c", "/tmp/x.toml"]).unwrap();
        match cli.command {
            Commands::Init { config } => {
                assert_eq!(config, Some(PathBuf::from("/tmp/x.toml")))
            }
            _ => panic!("expected init"),
        }
    }

    #[test]
    fn cli_try_parse_accepts_config_long() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "init", "--config", "/tmp/x.toml"]).unwrap();
        match cli.command {
            Commands::Init { config } => {
                assert_eq!(config, Some(PathBuf::from("/tmp/x.toml")))
            }
            _ => panic!("expected init"),
        }
    }

    #[test]
    fn cli_try_parse_init_without_config() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "init"]).unwrap();
        match cli.command {
            Commands::Init { config } => assert_eq!(config, None),
            _ => panic!("expected init"),
        }
    }

    #[test]
    fn cli_command_debug_assert() {
        Cli::command().debug_assert();
    }

    #[test]
    fn cli_try_parse_accepts_branch() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "branch"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_accepts_artifact_directory() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "artifact_directory"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_rejects_kebab_case_artifact_directory() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "artifact-directory"]);
        assert!(result.is_err());
    }

    #[test]
    fn artifact_dir_path_appends_trailing_slash() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "main");
        assert_eq!(path, "/repo/.pi/orksorksorks/main/");
    }

    #[test]
    fn artifact_dir_path_is_plain() {
        let path = artifact_dir_path(std::path::Path::new("/repo"), "main");
        assert!(!path.contains('\x1b'), "{path}");
    }

    #[test]
    fn artifact_dir_path_handles_branch_with_slashes() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "feature/x");
        assert_eq!(path, "/repo/.pi/orksorksorks/feature-x/");
    }

    #[test]
    fn artifact_dir_path_replaces_all_slashes() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "a/b/c");
        assert_eq!(path, "/repo/.pi/orksorksorks/a-b-c/");
    }

    #[test]
    fn artifact_dir_path_leaves_slashless_branch_unchanged() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "main");
        assert_eq!(path, "/repo/.pi/orksorksorks/main/");
    }

    #[test]
    fn determine_step_returns_step_with_present_artifact() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("first.txt"), "").unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "first.txt".to_string(),
                    model: "small".to_string(),
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "second.txt".to_string(),
                    model: "high".to_string(),
                },
            ],
            models: vec![],
            prompts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "one");
    }

    #[test]
    fn determine_step_prefers_last_step_in_reverse() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("first.txt"), "").unwrap();
        std::fs::write(dir.path().join("second.txt"), "").unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "first.txt".to_string(),
                    model: "small".to_string(),
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "second.txt".to_string(),
                    model: "high".to_string(),
                },
            ],
            models: vec![],
            prompts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "two");
    }

    #[test]
    fn determine_step_empty_trigger_is_default_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "first.txt".to_string(),
                    model: "small".to_string(),
                },
                Step {
                    name: "default".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                },
            ],
            models: vec![],
            prompts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(
            determine_step(&config, &artifact_dir).unwrap().name,
            "default"
        );
    }

    #[test]
    fn determine_step_empty_trigger_does_not_shadow_real_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("first.txt"), "").unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "first.txt".to_string(),
                    model: "small".to_string(),
                },
                Step {
                    name: "default".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                },
            ],
            models: vec![],
            prompts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "one");
    }

    #[test]
    fn determine_step_empty_trigger_matches_before_later_artifact() {
        // The default is a last resort: it does not beat a later step's
        // real artifact, only the absence of any artifact.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("second.txt"), "").unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "default".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "second.txt".to_string(),
                    model: "high".to_string(),
                },
            ],
            models: vec![],
            prompts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "two");
    }

    #[test]
    fn determine_step_latest_empty_trigger_wins() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: String::new(),
                    model: "high".to_string(),
                },
            ],
            models: vec![],
            prompts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "two");
    }

    #[test]
    fn cli_try_parse_accepts_step() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "step"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_accepts_step_with_custom_config() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "step", "--config", "custom.toml"]).unwrap();
        match cli.command {
            Commands::Step { config } => {
                assert_eq!(config, Some(std::path::PathBuf::from("custom.toml")));
            }
            _ => panic!("expected Commands::Step"),
        }
    }

    #[test]
    fn cli_try_parse_step_without_config_is_none() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "step"]).unwrap();
        match cli.command {
            Commands::Step { config } => assert_eq!(config, None),
            _ => panic!("expected Commands::Step"),
        }
    }

    #[test]
    fn select_command_routes_step() {
        let cli = Cli {
            json: false,
            command: Commands::Step {
                config: Some(std::path::PathBuf::from(
                    "definitely-missing-config-file.toml",
                )),
            },
        };
        // step_command reads the (missing) config first → "io", proving the
        // arm dispatched to step_command (and not, e.g., branch/artifact_directory).
        let err = select_command(&cli).unwrap_err();
        assert_eq!(err.source, "io");
    }

    #[test]
    fn determine_step_no_match_errors_with_step_tag() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![Step {
                name: "one".to_string(),
                trigger_artifact: "first.txt".to_string(),
                model: "small".to_string(),
            }],
            models: vec![],
            prompts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        let err = determine_step(&config, &artifact_dir).unwrap_err();
        assert_eq!(err.source, "step");
        assert!(
            err.message.contains("no trigger artifact matched"),
            "{}",
            err.message
        );
    }

    #[test]
    fn resolve_model_returns_model_for_matching_name() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![crate::config::Model {
                name: "small".to_string(),
                model: "openrouter/deepseek/flash".to_string(),
                thinking: "high".to_string(),
            }],
            prompts: vec![],
        };
        assert_eq!(
            resolve_model(&config, "small").unwrap().model,
            "openrouter/deepseek/flash",
        );
        assert_eq!(resolve_model(&config, "small").unwrap().thinking, "high");
    }

    #[test]
    fn resolve_model_missing_name_errors_with_model_tag() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![crate::config::Model {
                name: "small".to_string(),
                model: "openrouter/deepseek/flash".to_string(),
                thinking: "high".to_string(),
            }],
            prompts: vec![],
        };
        let err = resolve_model(&config, "large").unwrap_err();
        assert_eq!(err.source, "model");
        assert!(err.message.contains("no model named"), "{}", err.message);
    }

    #[test]
    fn select_command_routes_model() {
        let cli = Cli {
            json: false,
            command: Commands::Model {
                config: Some(std::path::PathBuf::from(
                    "definitely-missing-config-file.toml",
                )),
            },
        };
        // model_command reads the (missing) config first → "io", proving the
        // arm dispatched to model_command.
        let err = select_command(&cli).unwrap_err();
        assert_eq!(err.source, "io");
    }

    #[test]
    fn select_command_routes_thinking() {
        let cli = Cli {
            json: false,
            command: Commands::Thinking {
                config: Some(std::path::PathBuf::from(
                    "definitely-missing-config-file.toml",
                )),
            },
        };
        // thinking_command reads the (missing) config first → "io", proving
        // the arm dispatched to thinking_command.
        let err = select_command(&cli).unwrap_err();
        assert_eq!(err.source, "io");
    }

    #[test]
    fn cli_try_parse_accepts_model() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "model"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_model_without_config_is_none() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "model"]).unwrap();
        match cli.command {
            Commands::Model { config } => assert_eq!(config, None),
            _ => panic!("expected Commands::Model"),
        }
    }

    #[test]
    fn cli_try_parse_model_with_custom_config() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["orksorksorks", "model", "--config", "custom.toml"]).unwrap();
        match cli.command {
            Commands::Model { config } => {
                assert_eq!(config, Some(std::path::PathBuf::from("custom.toml")));
            }
            _ => panic!("expected Commands::Model"),
        }
    }

    #[test]
    fn cli_try_parse_accepts_thinking() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "thinking"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_thinking_without_config_is_none() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "thinking"]).unwrap();
        match cli.command {
            Commands::Thinking { config } => assert_eq!(config, None),
            _ => panic!("expected Commands::Thinking"),
        }
    }

    #[test]
    fn cli_try_parse_thinking_with_custom_config() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["orksorksorks", "thinking", "--config", "custom.toml"]).unwrap();
        match cli.command {
            Commands::Thinking { config } => {
                assert_eq!(config, Some(std::path::PathBuf::from("custom.toml")));
            }
            _ => panic!("expected Commands::Thinking"),
        }
    }

    #[test]
    fn cli_try_parse_accepts_prompt() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "prompt", "questions"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_prompt_without_step_name_derives_automatically() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "prompt"]).unwrap();
        match cli.command {
            Commands::Prompt { step_name, config } => {
                assert_eq!(step_name, None);
                assert_eq!(config, None);
            }
            _ => panic!("expected Commands::Prompt"),
        }
    }

    #[test]
    fn cli_try_parse_prompt_reads_step_name() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "prompt", "design"]).unwrap();
        match cli.command {
            Commands::Prompt { step_name, config } => {
                assert_eq!(step_name, Some("design".to_string()));
                assert_eq!(config, None);
            }
            _ => panic!("expected Commands::Prompt"),
        }
    }

    #[test]
    fn cli_try_parse_prompt_with_custom_config() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "orksorksorks",
            "prompt",
            "questions",
            "--config",
            "custom.toml",
        ])
        .unwrap();
        match cli.command {
            Commands::Prompt { step_name, config } => {
                assert_eq!(step_name, Some("questions".to_string()));
                assert_eq!(config, Some(std::path::PathBuf::from("custom.toml")));
            }
            _ => panic!("expected Commands::Prompt"),
        }
    }

    #[test]
    fn select_command_routes_prompt() {
        // step_name None — the primary auto-derive mode: dispatch still
        // happens before any git/artifact logic, so the missing config
        // error proves the arm reached prompt_command.
        let cli = Cli {
            json: false,
            command: Commands::Prompt {
                step_name: None,
                config: Some(std::path::PathBuf::from(
                    "definitely-missing-config-file.toml",
                )),
            },
        };
        // prompt_command reads the (missing) config first → "io", proving
        // the arm dispatched to prompt_command.
        let err = select_command(&cli).unwrap_err();
        assert_eq!(err.source, "io");
    }

    #[test]
    fn resolve_prompt_returns_content_for_matching_name() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![crate::config::Prompt {
                name: "questions".to_string(),
                content: "# Question — Decompose the Task\n".to_string(),
            }],
        };
        assert_eq!(
            resolve_prompt(&config, "questions").unwrap(),
            "# Question — Decompose the Task\n",
        );
    }

    #[test]
    fn resolve_prompt_missing_name_errors_with_prompt_tag() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![crate::config::Prompt {
                name: "questions".to_string(),
                content: String::new(),
            }],
        };
        let err = resolve_prompt(&config, "research").unwrap_err();
        assert_eq!(err.source, "prompt");
        assert!(err.message.contains("no prompt named"), "{}", err.message);
    }
}
