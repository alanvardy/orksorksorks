# ADR 0002 — Typed errors over bare status codes (retrospective)

Status: Accepted (recorded 2026-09-09; the decision predates this record)

Context: The CLI spans config parsing, git resolution, and per-command
lookups, and needs a uniform, testable, machine-readable failure mode.

Decision: All failures flow through the central `Error` type in
`src/errors.rs` — a lowercase `source` tag (`io`, `toml::ser`, `toml::de`,
`config:*`, `config-dir`, `config-exists`, `step`, `model`, `prompt`,
`script`, `git`) plus a human-readable `message`. Display is owned by `Error`
and colored only via the `src/format.rs` chokepoint; `-j/--json` renders the
same failure as `{"error": {"message", "source"}}`, and commands exit with
status 1. Source files never emit bare status codes or ad-hoc error output.

Consequences: Error handling is uniform and testable (tags are asserted in
unit and integration tests), JSON output stays machine-friendly, and the tag
taxonomy gives users a stable key for automation. The cost is that every
error path must route through the central type.