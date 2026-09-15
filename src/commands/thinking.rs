use super::resolve;
use crate::errors::Error;

/// Handle the `thinking` subcommand: determine the current step (via
/// `--step <NAME>` when given, else artifact derivation), read the model
/// *name* it references, and resolve that name against `config.models` to
/// its thinking-budget value.
pub(crate) fn thinking_command(
    path: &std::path::Path,
    source: crate::config_dir::ConfigPathSource,
    step: Option<String>,
) -> Result<String, Error> {
    let (cfg, step) = resolve::read_config_and_step(path, source, step)?;
    let model = resolve::resolve_model(&cfg, &step.model)?;
    Ok(model.thinking)
}
