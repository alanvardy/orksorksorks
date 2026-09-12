use super::resolve;
use crate::errors::Error;
use crate::git;

/// Handle the `artifact_directory` subcommand: return
/// `$PWD/.pi/orksorksorks/<branch>/`, plain (no directory creation).
pub(crate) fn artifact_directory_command() -> Result<String, Error> {
    let cwd = std::env::current_dir()?;
    Ok(resolve::artifact_dir_path(&cwd, &git::current_branch()?))
}
