# Task — VAR-824: Determine step

The orksorksorks CLI needs to determine which step a project is currently on by
reading which artifacts are present. A new `[[steps]]` array-of-tables is added
to the TOML config, each entry holding a step `name` and a `trigger_artifact`
filename. A new `step` command takes a `config` argument (path to the TOML),
reads the steps from config, iterates them in reverse, and returns the name of
the first step whose `trigger_artifact` exists at the configured
`artifact_directory`; if none exist it moves to the next step.