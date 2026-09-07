# Research Questions

## Context

Focus on the `orksorksorks` Rust CLI (edition 2024, binary-only crate): the clap
derive argument surface and command routing (`src/commands/mod.rs`), the Config
type and serialization (`src/config.rs`), the central error type and its `From`
conversions (`src/errors.rs`), the main entry/output layer (`src/main.rs`),
and the integration tests in `tests/`. Note that no home- or config-directory
resolution exists anywhere in the codebase today, and dependencies are limited
to clap 4.6, colored, serde/serde_json, toml, tokio, tempfile, assert_cmd,
predicates, and pretty_assertions.

## Questions

1. How does argument parsing flow from `Cli::parse()` through the `Cli` struct,
   the `Commands` subcommand enum, and `select_command` routing
   (`src/commands/mod.rs`)? What clap attribute patterns are already in use
   (shorts, `global`, `default_value_t`), and how do clap v4 derive options —
   including path-style argument types and default values evaluated at runtime —
   behave inside subcommands?

2. What exactly does the `init` command do today (`src/commands/mod.rs:48-61`)?
   Trace its config serialization, the hardcoded relative write path
   (`File::create("orksorksorks.toml")`), overwrite/truncation behavior, `sync_all`
   usage, and how each `?` propagates into the central `Error` type — which
   `From` impls (`src/errors.rs`) cover each failure mode, and which std error
   kinds (e.g. path/not-found, permission, invalid-UTF8 in path strings) lack
   a conversion?

3. What are the established Rust ecosystem conventions and crates (e.g.
   `dirs-rs`, `etcetera`, `home`, `std::env`) for resolving a per-user default
   config directory cross-platform, including `XDG_CONFIG_HOME`/`APPDATA`-style
   environment overrides on Linux, macOS, and Windows, and their behavior when
   the variable is unset?

4. How is the central `Error` type (`src/errors.rs`) structured — message/source
   fields, `Display`, `Serialization`, the `From<std::io::Error>` and
   `From<toml::ser::Error>` conversions — and how are errors rendered in both
   text (`src/main.rs` output_text, stderr + bell) and JSON (`output_json`,
   stdout) output modes? What would a new conversion (e.g. for path/UTF-8
   errors) need to integrate with?

5. How are the integration tests (`tests/init_creates_file.rs`,
   `tests/json_output.rs`) and the in-module unit tests structured — the
   `tempfile::tempdir` + `current_dir` fixture strategy, assertions about exact
   file content and location, the readonly-directory failure cases, and how the
   binary-only crate (no `lib.rs`/`lib` target) constrains what tests can
   import? Which tests would need to change if the default write location moved
   away from the CWD, and what infrastructure exists (or is missing) for
   overriding the target directory in tests?

6. Where does the codebase currently read or write files and resolve paths
   besides `init` (committed `orksorksorks.toml` at repo root, `scripts/install.sh`,
   `.gitignore`), and are there any existing helpers or conventions for
   creating parent directories, handling symlinks/HOME, or normalizing paths
   that a config-dir write would need to account for?