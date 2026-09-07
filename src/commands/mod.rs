use clap::{Parser, Subcommand};

use crate::errors::Error;

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
    Init,

    /// Print the current git branch
    Branch,

    /// Print the artifact directory path (cwd/.pi/orksorksorks/<branch>/)
    #[command(name = "artifact_directory")]
    ArtifactDirectory,
}

/// Route a parsed CLI to its handler and return a success message or error.
pub fn select_command(cli: &Cli) -> Result<String, Error> {
    match &cli.command {
        Commands::Init => init_command(),
        Commands::Branch => branch_command(),
        Commands::ArtifactDirectory => artifact_directory_command(),
    }
}

/// Create a new `orksorksorks.toml` file with default configuration.
fn init_command() -> Result<String, Error> {
    use std::io::Write;

    let config = crate::config::Config::default();
    let toml_str = toml::to_string(&config)?;
    let mut file = std::fs::File::create("orksorksorks.toml")?;
    file.write_all(toml_str.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(crate::format::green_string("✓ Created orksorksorks.toml"))
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

/// Resolve the current git branch by invoking `git branch --show-current` in
/// the process working directory.
///
/// A failed spawn (git not on PATH) rides the existing `From<std::io::Error>`
/// → `"io"` tag; git-level failures use `Error::new("git", ...)`.
fn current_branch() -> Result<String, Error> {
    current_branch_in(&std::env::current_dir()?)
}

/// Resolve the git branch by running `git branch --show-current` in `dir`.
///
/// Testable in isolation: callers pass an explicit directory so unit tests can
/// point at a disposable repo instead of the ambient checkout (CI checks out
/// in detached HEAD, where `--show-current` is empty).
fn current_branch_in(dir: &std::path::Path) -> Result<String, Error> {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_branch_output(output.status.success(), &stdout, &stderr)
}

/// Pure decision: map `git branch --show-current` output to a branch or error.
fn parse_branch_output(exit_ok: bool, stdout: &str, stderr: &str) -> Result<String, Error> {
    if !exit_ok {
        return Err(Error::new("git", stderr.trim()));
    }
    let stdout = stdout.trim();
    if stdout.is_empty() {
        return Err(Error::new("git", "not on a branch (detached HEAD)"));
    }
    Ok(stdout.to_string())
}

/// Handle the `branch` subcommand: return the current git branch, plain.
fn branch_command() -> Result<String, Error> {
    current_branch()
}

/// Handle the `artifact_directory` subcommand: return
/// `$PWD/.pi/orksorksorks/<branch>/`, plain (no directory creation).
fn artifact_directory_command() -> Result<String, Error> {
    let cwd = std::env::current_dir()?;
    Ok(artifact_dir_path(&cwd, &current_branch()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn select_command_routes_init() {
        let cli = Cli {
            json: false,
            command: Commands::Init,
        };
        let result = select_command(&cli).unwrap();
        // Under cfg!(test) color is stripped
        assert_eq!(result, "✓ Created orksorksorks.toml");
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

    /// Create a disposable git repo with one commit on `branch`.
    ///
    /// Keeps git-dependent tests hermetic: CI's `actions/checkout` leaves the
    /// tree in detached HEAD (empty `branch --show-current`), so the ambient
    /// checkout cannot be used as a source of truth.
    fn init_git_branch(dir: &std::path::Path, branch: &str) {
        let run = |args: &[&str]| {
            let out = std::process::Command::new("git")
                .args(args)
                .current_dir(dir)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?} failed");
        };
        run(&["init"]);
        run(&["checkout", "-b", branch]);
        std::fs::write(dir.join("README"), "x").unwrap();
        run(&["add", "."]);
        run(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-qm",
            "init",
        ]);
    }

    #[test]
    fn current_branch_in_returns_configured_branch() {
        use pretty_assertions::assert_eq;
        let temp = tempfile::tempdir().unwrap();
        init_git_branch(temp.path(), "feature/foo");
        assert_eq!(current_branch_in(temp.path()).unwrap(), "feature/foo");
    }

    #[test]
    fn current_branch_in_detached_head_errors() {
        let temp = tempfile::tempdir().unwrap();
        init_git_branch(temp.path(), "main");
        let detach = std::process::Command::new("git")
            .args(["checkout", "--detach"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(detach.status.success());
        let err = current_branch_in(temp.path()).unwrap_err();
        assert_eq!(err.source, "git");
        assert!(err.message.contains("detached HEAD"), "{}", err.message);
    }

    #[test]
    fn parse_branch_output_trims_trailing_newline() {
        use pretty_assertions::assert_eq;
        let branch = parse_branch_output(true, "main\n", "").unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn parse_branch_output_detached_head_errors() {
        let err = parse_branch_output(true, "", "").unwrap_err();
        assert_eq!(err.source, "git");
        assert!(err.message.contains("detached HEAD"), "{}", err.message);
    }

    #[test]
    fn parse_branch_output_nonzero_exit_uses_stderr() {
        use pretty_assertions::assert_eq;
        let err = parse_branch_output(false, "", "fatal: not a git repository\n").unwrap_err();
        assert_eq!(err.source, "git");
        assert_eq!(err.message, "fatal: not a git repository");
    }
}
