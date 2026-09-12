use crate::errors::Error;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod artifact_directory;
mod branch;
mod init;
mod model;
mod prompt;
mod resolve;
mod script;
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
            prompt::prompt_command(&path, source, step.clone())
        }
        Commands::Script { step_name, config } => {
            let (path, source) =
                crate::config_dir::config_file_path_with_env(config.as_deref(), env)?;
            script::script_command(&path, source, step_name.clone())
        }
    }
}

/// Route a parsed CLI to its handler and return a success message or error.
pub fn select_command(cli: &Cli) -> Result<String, Error> {
    select_command_with_env(cli, &crate::config_dir::ConfigEnv::from_env())
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
}
