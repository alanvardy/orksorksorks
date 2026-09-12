use crate::errors::Error;
use crate::git;

/// Handle the `branch` subcommand: return the current git branch, plain.
pub(crate) fn branch_command() -> Result<String, Error> {
    git::current_branch()
}
