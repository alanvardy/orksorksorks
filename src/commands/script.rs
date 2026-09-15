use super::resolve;
use crate::errors::Error;

/// Handle the `script` subcommand: read the config and return the raw
/// `[[scripts]]` content referenced by the current step's `script` field,
/// or by the explicitly named step when `step_name` overrides detection.
/// No frontmatter is emitted (the content is meant to be run/piped).
pub(crate) fn script_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step_name: Option<String>,
) -> Result<String, Error> {
    let (cfg, step) = resolve::read_config_and_step(path, source, step_name)?;
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
