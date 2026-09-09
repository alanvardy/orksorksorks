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
    /// (i) Create a new orksorksorks.toml file; refuses if it already exists
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

        /// Override the current step (from `config.steps`) by name
        #[arg(long, value_name = "STEP")]
        step: Option<String>,
    },

    /// Print the model for the current step (from `config.models`)
    Model {
        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,

        /// Override the current step (from `config.steps`) by name
        #[arg(long, value_name = "STEP")]
        step: Option<String>,
    },

    /// Print the thinking budget for the current step (from `config.models`)
    Thinking {
        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,

        /// Override the current step (from `config.steps`) by name
        #[arg(long, value_name = "STEP")]
        step: Option<String>,
    },

    /// Print the prompt for the current step (from `config.prompts`)
    Prompt {
        /// Path to the TOML config file; defaults to the config directory
        /// (the same resolution as `init`)
        #[arg(long, value_name = "CONFIG")]
        config: Option<PathBuf>,

        /// Override the current step (from `config.steps`) by name
        #[arg(long, value_name = "STEP")]
        step: Option<String>,
    },

    /// Print the raw script for the current step (from `config.scripts`)
    Script {
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
        Commands::Step { config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            step_command(&path, source, step.clone())
        }
        Commands::Model { config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            model_command(&path, source, step.clone())
        }
        Commands::Thinking { config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            thinking_command(&path, source, step.clone())
        }
        Commands::Prompt { step, config } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            prompt_command(&path, source, step.clone())
        }
        Commands::Script { step_name, config } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            script_command(&path, source, step_name.clone())
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
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Error::new(
                    "config-exists",
                    &format!("{} already exists; not overwriting", path.display()),
                )
            } else {
                Error::from(e)
            }
        })?;
    file.write_all(include_str!("../../templates/default.toml").as_bytes())?;
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
/// `model`). A step with an empty `trigger_artifact` is a default: it never
/// matches by existence, but is returned instead of erroring when no other
/// step's artifact exists. Configs loaded through `read_config` have at most
/// one default (enforced by `config:multiple-default`); this function still
/// tolerates several and the last such step in the list wins.
/// `artifact_dir` must end in a trailing slash — the same string-composition
/// convention as `artifact_dir_path`.
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

/// Look up the named step in `config.steps` and return the matching entry.
/// First match wins; an unknown name errors with the `"step"` tag (the same
/// tag `determine_step` uses, so callers discriminate by message).
fn resolve_step(config: &Config, name: &str) -> Result<Step, Error> {
    for s in config.steps.iter() {
        if s.name == name {
            return Ok(s.clone());
        }
    }
    Err(Error::new("step", &format!("no step named {name:?}")))
}

/// Handle the `step` subcommand: read the config and return the current step
/// name. With `--step <NAME>`, the name replaces artifact-based derivation
/// (no git is consulted); otherwise the step is derived from cwd + git
/// branch + trigger artifacts.
///
/// `path` is the already-resolved config location: either the explicit
/// `--config` argument or the config-directory default (`config_dir.rs`).
/// `read_config` runs before git resolution so a missing/unreadable config
/// deterministically fails with `"io"`.
fn step_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let step = if let Some(name) = step {
        resolve_step(&cfg, &name)?
    } else {
        let cwd = std::env::current_dir()?;
        let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
        determine_step(&cfg, &artifact_dir)?
    };
    Ok(step.name)
}

/// Handle the `model` subcommand: determine the current step (via
/// `--step <NAME>` when given, else artifact derivation), read the model
/// *name* it references, and resolve that name against `config.models` to
/// the concrete model string.
fn model_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let step = if let Some(name) = step {
        resolve_step(&cfg, &name)?
    } else {
        let cwd = std::env::current_dir()?;
        let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
        determine_step(&cfg, &artifact_dir)?
    };
    let model = resolve_model(&cfg, &step.model)?;
    Ok(model.model)
}

/// Handle the `thinking` subcommand: determine the current step (via
/// `--step <NAME>` when given, else artifact derivation), read the model
/// *name* it references, and resolve that name against `config.models` to
/// its thinking-budget value.
fn thinking_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let step = if let Some(name) = step {
        resolve_step(&cfg, &name)?
    } else {
        let cwd = std::env::current_dir()?;
        let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
        determine_step(&cfg, &artifact_dir)?
    };
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

/// Look up the named script (a `config.scripts` key referenced by a step's
/// `script` field) and return its raw content.
fn resolve_script(config: &Config, name: &str) -> Result<String, Error> {
    for s in config.scripts.iter() {
        if s.name == name {
            return Ok(s.content.clone());
        }
    }
    Err(Error::new("script", &format!("no script named {name:?}")))
}

/// Handle the `prompt` subcommand: read the config and return the prompt
/// content for the current step, or for the explicitly named step when
/// `--step <NAME>` is provided as an override. Output is prefixed with a
/// frontmatter block (important-variables header, step, branch, artifact
/// directory) above the prompt content unless `show_frontmatter = false`
/// in the config.
///
/// `--step` skips only *step derivation*: the frontmatter block still
/// resolves the git branch, so with the default `show_frontmatter = true`,
/// `prompt --step <NAME>` needs a git checkout (unlike `step`/`model`/
/// `thinking`, which consult git only to derive the step).
fn prompt_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let name = if let Some(name) = step {
        resolve_step(&cfg, &name)?.name
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

/// Handle the `script` subcommand: read the config and return the raw
/// `[[scripts]]` content referenced by the current step's `script` field,
/// or by the explicitly named step when `step_name` overrides detection.
/// No frontmatter is emitted (the content is meant to be run/piped).
fn script_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step_name: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let step = if let Some(name) = step_name {
        // Explicit name: resolve by name (no git/artifact work), so unknown
        // script-name / no-script errors surface as `"script"`, never `"git"`.
        resolve_step(&cfg, &name)?
    } else {
        // No explicit step: derive the current step from trigger artifacts,
        // exactly like `step`/`model`/`thinking`/`prompt`.
        let cwd = std::env::current_dir()?;
        let artifact_dir = artifact_dir_path(&cwd, &git::current_branch()?);
        determine_step(&cfg, &artifact_dir)?
    };
    let script_name = step.script.ok_or_else(|| {
        Error::new(
            "script",
            &format!("step {:?} has no script configured", step.name),
        )
    })?;
    resolve_script(&cfg, &script_name)
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
                    script: None,
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "second.txt".to_string(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
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
                    script: None,
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "second.txt".to_string(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
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
                    script: None,
                },
                Step {
                    name: "default".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
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
                    script: None,
                },
                Step {
                    name: "default".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
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
                    script: None,
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "second.txt".to_string(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
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
                    script: None,
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: String::new(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
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
            Commands::Step { config, .. } => {
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
            Commands::Step { config, .. } => assert_eq!(config, None),
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
                step: None,
            },
        };
        // step_command reads the (missing) config first → "io", proving the
        // arm dispatched to step_command (and not, e.g., branch/artifact_directory).
        let err = select_command(&cli).unwrap_err();
        assert_eq!(err.source, "io");
    }

    #[test]
    fn cli_try_parse_step_flag_binds_value() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "step", "--step", "one"]).unwrap();
        match cli.command {
            Commands::Step { step, .. } => assert_eq!(step, Some("one".to_string())),
            _ => panic!("expected Commands::Step"),
        }
    }

    #[test]
    fn cli_try_parse_step_flag_absent_is_none() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "step"]).unwrap();
        match cli.command {
            Commands::Step { step, .. } => assert_eq!(step, None),
            _ => panic!("expected Commands::Step"),
        }
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
                script: None,
            }],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
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
            scripts: vec![],
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
            scripts: vec![],
        };
        let err = resolve_model(&config, "large").unwrap_err();
        assert_eq!(err.source, "model");
        assert!(err.message.contains("no model named"), "{}", err.message);
    }

    #[test]
    fn resolve_step_returns_matching_step() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![Step {
                name: "one".to_string(),
                trigger_artifact: "first.txt".to_string(),
                model: "small".to_string(),
                script: None,
            }],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let step = resolve_step(&config, "one").unwrap();
        assert_eq!(step.name, "one");
        assert_eq!(step.trigger_artifact, "first.txt");
        assert_eq!(step.model, "small");
    }

    #[test]
    fn resolve_step_unknown_name_tags_step_error() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![Step {
                name: "one".to_string(),
                trigger_artifact: String::new(),
                model: "small".to_string(),
                script: None,
            }],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let err = resolve_step(&config, "nope").unwrap_err();
        assert_eq!(err.source, "step");
        assert!(
            err.message.contains("no step named \"nope\""),
            "{}",
            err.message
        );
    }

    #[test]
    fn resolve_step_first_match_wins() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "dup".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                    script: None,
                },
                Step {
                    name: "dup".to_string(),
                    trigger_artifact: String::new(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let step = resolve_step(&config, "dup").unwrap();
        assert_eq!(step.model, "small");
    }

    #[test]
    fn select_command_routes_model() {
        let cli = Cli {
            json: false,
            command: Commands::Model {
                config: Some(std::path::PathBuf::from(
                    "definitely-missing-config-file.toml",
                )),
                step: None,
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
                step: None,
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
            Commands::Model { config, .. } => assert_eq!(config, None),
            _ => panic!("expected Commands::Model"),
        }
    }

    #[test]
    fn cli_try_parse_model_with_custom_config() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["orksorksorks", "model", "--config", "custom.toml"]).unwrap();
        match cli.command {
            Commands::Model { config, .. } => {
                assert_eq!(config, Some(std::path::PathBuf::from("custom.toml")));
            }
            _ => panic!("expected Commands::Model"),
        }
    }

    #[test]
    fn cli_try_parse_model_step_flag_binds_value() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "model", "--step", "one"]).unwrap();
        match cli.command {
            Commands::Model { step, .. } => assert_eq!(step, Some("one".to_string())),
            _ => panic!("expected Commands::Model"),
        }
    }

    #[test]
    fn cli_try_parse_model_step_flag_absent_is_none() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "model"]).unwrap();
        match cli.command {
            Commands::Model { step, .. } => assert_eq!(step, None),
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
            Commands::Thinking { config, .. } => assert_eq!(config, None),
            _ => panic!("expected Commands::Thinking"),
        }
    }

    #[test]
    fn cli_try_parse_thinking_with_custom_config() {
        use clap::Parser;
        let cli =
            Cli::try_parse_from(["orksorksorks", "thinking", "--config", "custom.toml"]).unwrap();
        match cli.command {
            Commands::Thinking { config, .. } => {
                assert_eq!(config, Some(std::path::PathBuf::from("custom.toml")));
            }
            _ => panic!("expected Commands::Thinking"),
        }
    }

    #[test]
    fn cli_try_parse_thinking_step_flag_binds_value() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "thinking", "--step", "one"]).unwrap();
        match cli.command {
            Commands::Thinking { step, .. } => assert_eq!(step, Some("one".to_string())),
            _ => panic!("expected Commands::Thinking"),
        }
    }

    #[test]
    fn cli_try_parse_thinking_step_flag_absent_is_none() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "thinking"]).unwrap();
        match cli.command {
            Commands::Thinking { step, .. } => assert_eq!(step, None),
            _ => panic!("expected Commands::Thinking"),
        }
    }

    #[test]
    fn cli_try_parse_rejects_prompt_positional() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "prompt", "questions"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_try_parse_prompt_without_step_name_derives_automatically() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "prompt"]).unwrap();
        match cli.command {
            Commands::Prompt { step, config } => {
                assert_eq!(step, None);
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
            "--step",
            "questions",
            "--config",
            "custom.toml",
        ])
        .unwrap();
        match cli.command {
            Commands::Prompt { step, config } => {
                assert_eq!(step, Some("questions".to_string()));
                assert_eq!(config, Some(std::path::PathBuf::from("custom.toml")));
            }
            _ => panic!("expected Commands::Prompt"),
        }
    }

    #[test]
    fn cli_try_parse_prompt_step_flag_binds_value() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "prompt", "--step", "one"]).unwrap();
        match cli.command {
            Commands::Prompt { step, .. } => assert_eq!(step, Some("one".to_string())),
            _ => panic!("expected Commands::Prompt"),
        }
    }

    #[test]
    fn cli_try_parse_prompt_step_flag_absent_is_none() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "prompt"]).unwrap();
        match cli.command {
            Commands::Prompt { step, .. } => assert_eq!(step, None),
            _ => panic!("expected Commands::Prompt"),
        }
    }

    #[test]
    fn cli_try_parse_accepts_script() {
        use clap::Parser;
        let result = Cli::try_parse_from(["orksorksorks", "script", "run-one"]);
        assert!(result.is_ok());
    }

    #[test]
    fn cli_try_parse_script_without_step_name_is_none() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "script"]).unwrap();
        match cli.command {
            Commands::Script { step_name, config } => {
                assert_eq!(step_name, None);
                assert_eq!(config, None);
            }
            _ => panic!("expected Commands::Script"),
        }
    }

    #[test]
    fn cli_try_parse_script_reads_step_name() {
        use clap::Parser;
        let cli = Cli::try_parse_from(["orksorksorks", "script", "run-one"]).unwrap();
        match cli.command {
            Commands::Script { step_name, .. } => {
                assert_eq!(step_name, Some("run-one".to_string()))
            }
            _ => panic!("expected Commands::Script"),
        }
    }

    #[test]
    fn cli_try_parse_script_with_custom_config() {
        use clap::Parser;
        let cli = Cli::try_parse_from([
            "orksorksorks",
            "script",
            "run-one",
            "--config",
            "custom.toml",
        ])
        .unwrap();
        match cli.command {
            Commands::Script { step_name, config } => {
                assert_eq!(step_name, Some("run-one".to_string()));
                assert_eq!(config, Some(std::path::PathBuf::from("custom.toml")),);
            }
            _ => panic!("expected Commands::Script"),
        }
    }

    #[test]
    fn select_command_routes_prompt() {
        // No explicit step — the primary auto-derive mode: dispatch still
        // happens before any git/artifact logic, so the missing config
        // error proves the arm reached prompt_command.
        let cli = Cli {
            json: false,
            command: Commands::Prompt {
                step: None,
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
    fn select_command_routes_script() {
        // No explicit step — the primary auto-derive mode: dispatch still
        // happens before any git/artifact logic, so the missing config
        // error proves the arm reached script_command.
        let cli = Cli {
            json: false,
            command: Commands::Script {
                step_name: None,
                config: Some(std::path::PathBuf::from(
                    "definitely-missing-config-file.toml",
                )),
            },
        };
        // script_command reads the (missing) config first → "io", proving
        // the arm dispatched to script_command.
        let err = select_command(&cli).unwrap_err();
        assert_eq!(err.source, "io");
    }

    #[test]
    fn step_command_flag_succeeds_in_non_git_dir() {
        // A tempdir with NO git repo: bare step would fail in git::current_branch,
        // so success proves `--step` never shells out to git or reads cwd.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("orksorksorks.toml"),
            concat!(
                "version = \"0.1.0\"\n",
                "[[steps]]\nname = \"one\"\ntrigger_artifact = \"first.txt\"\nmodel = \"small\"\n",
                "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
                "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
            ),
        )
        .unwrap();
        let out = step_command(
            &dir.path().join("orksorksorks.toml"),
            crate::config_dir::ConfigPathSource::ExplicitFlag,
            Some("one".to_string()),
        )
        .unwrap();
        assert_eq!(out, "one");
    }

    #[test]
    fn step_command_flag_unknown_name_tags_step() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("orksorksorks.toml"),
            "version = \"0.1.0\"\n",
        )
        .unwrap();
        let err = step_command(
            &dir.path().join("orksorksorks.toml"),
            crate::config_dir::ConfigPathSource::ExplicitFlag,
            Some("nope".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.source, "step");
        assert!(
            err.message.contains("no step named \"nope\""),
            "{}",
            err.message
        );
    }

    #[test]
    fn prompt_command_step_flag_unknown_name_tags_step() {
        // `--step nope` with no [[steps]]: fails in resolve_step with the
        // `"step"` tag — before resolve_prompt ever runs.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("orksorksorks.toml"),
            "version = \"0.1.0\"\n",
        )
        .unwrap();
        let err = prompt_command(
            &dir.path().join("orksorksorks.toml"),
            crate::config_dir::ConfigPathSource::ExplicitFlag,
            Some("nope".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.source, "step");
        assert!(
            err.message.contains("no step named \"nope\""),
            "{}",
            err.message
        );
    }

    #[test]
    fn prompt_command_step_flag_known_step_missing_prompt_tags_validation() {
        // `--step one` where step `one` exists but has no [[prompts]] entry:
        // the config is invalid, so `read_config` fails fast with
        // `config:missing-prompt` before resolve_step/resolve_prompt run. The
        // runtime `"prompt"` tag is unreachable through the validated
        // `--step` path, since every step must have a matching prompt.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("orksorksorks.toml"),
            "version = \"0.1.0\"\n[[steps]]\nname = \"one\"\ntrigger_artifact = \"first.txt\"\nmodel = \"small\"\n",
        )
        .unwrap();
        let err = prompt_command(
            &dir.path().join("orksorksorks.toml"),
            crate::config_dir::ConfigPathSource::ExplicitFlag,
            Some("one".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.source, "config:missing-prompt");
        assert!(
            err.message.contains("has no matching prompt"),
            "{}",
            err.message
        );
    }

    #[test]
    fn script_command_step_without_script_errors() {
        // Step `one` exists (validated config with matching prompt/model) but
        // has no `script` key: `read_config` succeeds, then the explicit-name
        // path resolves the step and fails on the missing script reference
        // with the `"script"` tag — before resolve_script ever runs. The
        // explicit-name path avoids git, so no repo is needed.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("orksorksorks.toml"),
            concat!(
                "version = \"0.1.0\"\n",
                "[[steps]]\nname = \"one\"\ntrigger_artifact = \"first.txt\"\nmodel = \"small\"\n",
                "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
                "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
            ),
        )
        .unwrap();
        let err = script_command(
            &dir.path().join("orksorksorks.toml"),
            crate::config_dir::ConfigPathSource::ExplicitFlag,
            Some("one".to_string()),
        )
        .unwrap_err();
        assert_eq!(err.source, "script");
        assert!(
            err.message.contains("has no script configured"),
            "{}",
            err.message
        );
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
            scripts: vec![],
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
            scripts: vec![],
        };
        let err = resolve_prompt(&config, "research").unwrap_err();
        assert_eq!(err.source, "prompt");
        assert!(err.message.contains("no prompt named"), "{}", err.message);
    }

    #[test]
    fn resolve_script_hit_returns_content() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![],
            scripts: vec![crate::config::Script {
                name: "run-one".to_string(),
                content: "#!/bin/bash\n".to_string(),
            }],
        };
        assert_eq!(resolve_script(&config, "run-one").unwrap(), "#!/bin/bash\n",);
    }

    #[test]
    fn resolve_script_miss_returns_script_tag() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![],
            scripts: vec![crate::config::Script {
                name: "run-one".to_string(),
                content: String::new(),
            }],
        };
        let err = resolve_script(&config, "nope").unwrap_err();
        assert_eq!(err.source, "script");
        assert!(err.message.contains("no script named"), "{}", err.message);
    }

    #[test]
    fn resolve_script_first_match_wins() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![],
            scripts: vec![
                crate::config::Script {
                    name: "dup".to_string(),
                    content: "first\n".to_string(),
                },
                crate::config::Script {
                    name: "dup".to_string(),
                    content: "second\n".to_string(),
                },
            ],
        };
        assert_eq!(resolve_script(&config, "dup").unwrap(), "first\n",);
    }
}
