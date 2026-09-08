# Task: Remove process-env mutation from tests to fix flaky config_dir tests

Unit tests in `src/config_dir.rs` (`config_dir::tests`) and `src/commands/mod.rs`
(`select_command_routes_init`) mutate the process-global environment
(`HOME`, `XDG_CONFIG_HOME`) and mostly never restore it; because nextest runs all
unit tests in-process in parallel, these mutations race with sibling tests and
intermittently cause false failures (e.g. `unresolved_home_yields_config_dir_error`
panicking on an `Ok` value). This task makes the suite deterministic by removing or
isolating process-env mutation in tests — preferred directions are dependency-injecting
the env into the path-resolution functions so tests need no `set_var` at all, or
isolating env-dependent logic per process; a serialization mutex is acceptable only as a
stopgap.