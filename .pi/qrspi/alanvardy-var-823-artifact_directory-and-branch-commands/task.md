# Task — artifact_directory and branch commands

Add two new subcommands to the `orksorksorks` CLI:

- `branch` — returns the current git branch as a string.
- `artifact_directory` — returns a string of the form `$PWD/.pi/orksorksorks/<branch>/`.

Both must behave consistently with the existing `init` command (dispatch, output formatting, JSON mode, error handling) and be covered by tests.