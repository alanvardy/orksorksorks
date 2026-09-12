use super::resolve;
use crate::errors::Error;
use crate::git;

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
pub(crate) fn prompt_command(
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
