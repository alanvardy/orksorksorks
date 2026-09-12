use crate::errors::Error;
use crate::git;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod artifact_directory;
mod branch;
mod init;
mod model;
mod resolve;
mod step;
mod thinking;

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
            init::init_command(&path)
        }
        Commands::Branch => branch::branch_command(),
        Commands::ArtifactDirectory => artifact_directory::artifact_directory_command(),
        Commands::Step { config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            step::step_command(&path, source, step.clone())
        }
        Commands::Model { config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            model::model_command(&path, source, step.clone())
        }
        Commands::Thinking { config, step } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            thinking::thinking_command(&path, source, step.clone())
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
        resolve::resolve_step(&cfg, &name)?.name
    } else {
        // No explicit step: derive the current step from trigger artifacts,
        // exactly like `step`/`model`/`thinking`.
        let cwd = std::env::current_dir()?;
        let artifact_dir = resolve::artifact_dir_path(&cwd, &git::current_branch()?);
        resolve::determine_step(&cfg, &artifact_dir)?.name
    };
    let content = resolve::resolve_prompt(&cfg, &name)?;

    // Frontmatter carries the run context (step, branch, artifact dir) above
    // the prompt output unless disabled by `show_frontmatter = false`.
    // Branch/artifact resolution is deferred until after the prompt resolves
    // so an unknown prompt keeps failing with its `"prompt"` error instead
    // of a git error. `step` is the effective prompt name (the explicit
    // override when given, else the derived step).
    if cfg.show_frontmatter {
        let cwd = std::env::current_dir()?;
        let branch = git::current_branch()?;
        let artifact_dir = resolve::artifact_dir_path(&cwd, &branch);
        return Ok(format!(
            "## Important variables\nThese are literal text values, not shell or\nenvironment variables. Wherever a prompt writes $<variable> (or\n($variable)path), substitute the value shown below as plain text; never\nwrite $variable in a shell command.\nstep = {}\nbranch = {}\nartifact_directory = {}\n\n{}",
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
        resolve::resolve_step(&cfg, &name)?
    } else {
        // No explicit step: derive the current step from trigger artifacts,
        // exactly like `step`/`model`/`thinking`/`prompt`.
        let cwd = std::env::current_dir()?;
        let artifact_dir = resolve::artifact_dir_path(&cwd, &git::current_branch()?);
        resolve::determine_step(&cfg, &artifact_dir)?
    };
    let script_name = step.script.ok_or_else(|| {
        Error::new(
            "script",
            &format!("step {:?} has no script configured", step.name),
        )
    })?;
    resolve::resolve_script(&cfg, &script_name)
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
