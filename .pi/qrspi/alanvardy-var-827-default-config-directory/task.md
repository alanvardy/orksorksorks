# Task: Default config directory

When no explicit path is passed to the CLI, the tool's config file should default
to `~/.config/orksorksorks.toml` rather than a file in the current working
directory. Today the `init` command writes `orksorksorks.toml` into the CWD; this
task changes the default location to a per-user config directory.