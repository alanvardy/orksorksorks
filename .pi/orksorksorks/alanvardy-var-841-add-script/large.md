# Task

Add a nullable `script` field to each entry in the `[[steps]]` table of the
`orksorksorks.toml` config: when a step names a script, that name is looked up
under a new `scripts` section (matched on its `name` field) and the matched
script text is surfaced via a new `script` CLI subcommand that outputs the
script text for the current step (mirroring how `prompt` resolves the current
step and prints its content).

## Why LARGE

SCHEMA + CROSS_CUTTING (+ CONVENTION_RISK): this changes the persisted
`orksorksorks.toml` data model — a new `script` field on `Step` in
`src/config.rs` and a new top-level `scripts` collection (the config is
format-versioned "so future migrations can detect and upgrade older files") —
and simultaneously adds a new CLI interface surface (`script` command with its
own output contract in `src/commands/mod.rs`), requiring backward-compatible
parsing of existing configs (serde defaults) across the shared config format.