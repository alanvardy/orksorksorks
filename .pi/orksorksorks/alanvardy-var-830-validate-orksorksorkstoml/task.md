# Task

Add validation for the `orksorksorks.toml` configuration file. The names
listed in the config's `steps` must match the names in the `prompt` section
exactly — same length, same members, in the same order — and duplicate
names must be rejected with an error. Beyond this core cross-validation
check, survey what else in the config could reasonably be validated and
bring those findings back to the user for sign-off before implementing.

## Why LARGE

The ticket explicitly requires an open-ended exploration of additional
validations ("check what else can be validated") to be brought back for a
human decision, and it tightens validation of the shared config format that
every run depends on — a backward-compatibility-sensitive surface with
unknown scope. Note: neither a `prompts` nor a `prompt` section exists in
the codebase today (the config only has `version` and `steps`), so the
cross-target the validation refers to is also being defined here.
