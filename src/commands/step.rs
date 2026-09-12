use super::resolve;
use crate::errors::Error;
use crate::git;

/// Handle the `step` subcommand: read the config and return the current step
/// name. With `--step <NAME>`, the name replaces artifact-based derivation
/// (no git is consulted); otherwise the step is derived from cwd + git
/// branch + trigger artifacts.
///
/// `path` is the already-resolved config location: either the explicit
/// `--config` argument or the config-directory default (`config_dir.rs`).
/// `read_config` runs before git resolution so a missing/unreadable config
/// deterministically fails with `"io"`.
pub(crate) fn step_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let cfg = crate::config::read_config(path, source)?;
    let step = if let Some(name) = step {
        resolve::resolve_step(&cfg, &name)?
    } else {
        let cwd = std::env::current_dir()?;
        let artifact_dir = resolve::artifact_dir_path(&cwd, &git::current_branch()?);
        resolve::determine_step(&cfg, &artifact_dir)?
    };
    Ok(step.name)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
