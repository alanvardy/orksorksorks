# Changelog

All notable changes to orksorksorks are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com), and
this project adheres to [Semantic Versioning](https://semver.org).

## [Unreleased]

Contributors add entries here for user-visible changes. This section covers
the 0.1.0 capability set; when 0.1.0 is tagged, the entries below move under
a `## [0.1.0] — <release date>` header and releases gain version/date
headers going forward.

### Added

- `init` — writes a commented `orksorksorks.toml` template and refuses to
  overwrite an existing file.
- `step`, `model`, `thinking`, `prompt`, `script`, `branch`, and
  `artifact_directory` subcommands driven by the TOML config.
- Typed errors (central `Error` type with lowercase source tags), colored
  output, and a `-j/--json` JSON envelope.
- Strict TOML config validation — unknown keys are rejected, the `version`
  must match, and cross-references between `steps`, `models`, and `prompts`
  are checked.