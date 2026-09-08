# Task

Validate the `orksorksorks.toml` configuration file. The names in `steps`
must match the names in `prompts` exactly — same length, same members, and
duplicates must error. The ticket also asks to survey what else in the
config can be validated and bring findings back to the user for sign-off.

## Why LARGE

Matched triggers: **DESIGN_SIGN-OFF**, **CONVENTION_RISK**, **UNKNOWNS**.
The ticket explicitly requires an open-ended exploration of additional
validations ("check what else can be validated") to be brought back for a
human decision, and it tightens validation of the shared config/persistence
format that every run depends on — a backward-compatibility-sensitive
surface with unknown scope.