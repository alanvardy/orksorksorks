# Task

Add a nullable `script` field to each entry in the `[[steps]]` table of the
`orksorksorks.toml` config: when a step names a script, that name is looked up
under a new `scripts` section (matched on its `name` field) and the matched
script text is surfaced via a new `script` CLI subcommand that outputs the
script text for the current step (mirroring how `prompt` resolves the current
step and prints its content). Existing configs must keep parsing unchanged
(serde defaults) since the config is format-versioned to permit future
migrations.