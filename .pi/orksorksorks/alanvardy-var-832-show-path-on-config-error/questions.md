# Research Questions

## Context

The CLI discovers and loads its TOML configuration from a resolved path
(either an explicit `--config` flag or a config directory derived from
environment variables), then surfaces any failure through a central error
type rendered to text or JSON output. Focus areas: config discovery and
loading (`src/config_dir.rs`, `src/config.rs`, `src/commands/mod.rs`), the
error and output layers (`src/errors.rs`, `src/main.rs`), and the tests
covering these paths (`src/*_tests`, `tests/`).

## Questions

1. How does the CLI resolve the config file location (explicit `--config`
   flag vs. environment-driven config-dir resolution), and through which
   call sites in the command dispatch flow do both the resolved path and the
   subsequent read attempt pass? Where in that flow is the resolved path
   known but not used?

2. How do errors flow from the point where they are raised to the point
   where they are rendered to the user? Where is the single chokepoint where
   error messages are formatted (text vs. JSON envelope, ANSI coloring, exit
   codes), and what does that chokepoint have access to at the time?

3. How does the `std::io::Error` → central `Error` conversion work, and
   what path information (if any) does the original std io error carry or
   lose in that conversion?

4. What conventions exist for error message content — source tag naming,
   lowercasing, quoting of or interpolating dynamic values like file paths
   — and which existing error messages already interpolate dynamic paths or
   other runtime values? Where is coloring applied relative to message
   construction?

5. How are config-not-found and error-output behaviors covered by tests:
   which unit and integration tests assert on error source tags, message
   content, JSON shape, or exit codes, and what do the existing assertions
   for the missing-config-file path require exactly?