use super::resolve;
use crate::errors::Error;
use crate::git;

/// Handle the `model` subcommand: determine the current step (via
/// `--step <NAME>` when given, else artifact derivation), read the model
/// *name* it references, and resolve that name against `config.models` to
/// the concrete model string.
pub(crate) fn model_command(
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
    let model = resolve::resolve_model(&cfg, &step.model)?;
    Ok(model.model)
}
