# Design Discussion

## Current State

The CLI resolves its config file path through `config_file_path(explicit: Option<&Path>)` (`src/config_dir.rs:16-20`): an explicit `--config` flag passes through directly, otherwise `resolve_config_dir()` (`src/config_dir.rs:24-41`) picks `XDG_CONFIG_HOME` (if absolute), `%APPDATA%` (Windows), or `$HOME/.config`, and appends `orksorksorks.toml`. Both dispatch arms — `init` and `step` — call this identically (`src/commands/mod.rs:69-70,75-76`).

When the config file is missing, `step_command` passes the resolved path to `read_config(path)` (`src/commands/mod.rs:164`), which calls `std::fs::read_to_string(path)?` (`src/config.rs:40`). The `?` converts the `io::Error` via `From<std::io::Error>` (`src/errors.rs:39-46`), which sets `source = "io"` and `message = e.to_string()` — the OS strerror text with no filename. This fork's stdlib lacks `io::Error::with_path` entirely, so the path is structurally absent from the stdlib error text.

The error then propagates through `step_command` → `select_command` → `run_command` (`src/main.rs:64`), reaching the single render chokepoint (`src/main.rs:62-77`). `output_result` (`src/main.rs:53-58`) branches to `output_text` (stderr, colored via `Display for Error` at `src/errors.rs:26-34`) or `output_json` (stdout, `{"error": {"message": ..., "source": ...}}` at `src/main.rs:46`). The chokepoint has access only to the fully-composed `Error` — it cannot add context.

**Result**: the user sees `No such file or directory (os error 2)` with no indication of which path was tried or how it was resolved. The path was known at `src/config.rs:40` and `src/commands/mod.rs:164` but never entered the error message.

The init success path already embeds the path: `Ok(format!("✓ Created {}", path.display()))` (`src/commands/mod.rs:94-96`), proving path rendering on this code path has precedent.

Tests for missing-config assert only `source == "io"` (`src/config.rs:111`, `src/commands/mod.rs:470`); no test pins the message text. The integration regression (`tests/step.rs:198-213`) asserts non-zero exit only. Message text changes on this path are unconstrained.

## Desired End State

When the config file cannot be read, both text and JSON error output include:

1. The **resolved config file path** (e.g. `/Users/vardy/.config/orksorksorks/orksorksorks.toml`)
2. **How it was resolved** — whether it came from an explicit `--config` flag, `XDG_CONFIG_HOME`, `HOME/.config`, or `%APPDATA%` on Windows
3. The **underlying OS error text** preserved

The error `source` tag remains `"io"`. No new fields are added to the `Error` struct. No JSON schema changes. All existing tests pass without modification.

Verification: a missing `--config /tmp/nope.toml` produces text-mode stderr containing `/tmp/nope.toml` and the phrase "specified via --config"; JSON stdout `{"error":{"message":...}}` contains the same path text. A missing config with `XDG_CONFIG_HOME=/bad/dir` mentions `/bad/dir/orksorksorks/orksorksorks.toml` and "resolved from XDG_CONFIG_HOME".

## Patterns to Follow

### Error construction conventions (`src/errors.rs:5-7,16-17`)
- `source`: lowercase tag (here `"io"` stays unchanged)
- `message`: human-readable, lowercase initial, no trailing punctuation
- Dynamic values via `&format!(...)`, interpolated bare — no quotes or brackets
- Error color applied at render time inside `Display` (`src/errors.rs:30-32`); stored strings are plain
- `Error::new("tag", &format!(...))` for any message with interpolated values

### Message format pattern (`src/commands/mod.rs:150-153`)
The only existing dynamic error message: `"{artifact_dir}: no trigger artifact matched"` — value emitted bare, colon-space before explanation clause. Follow this pattern: `"could not read config file at <path>: <io message>"`.

### Path rendering (`src/commands/mod.rs:94-96,108`, conventions.md)
Paths stringify exclusively via `.display()`. No quotes, no escaping. The init success message `"✓ Created {}"` shows the bare `path.display()` output.

### ANSI / test compatibility (`src/format.rs:7`, conventions.md)
`cfg!(test)` strips all ANSI escapes in `apply_color`. Any new message that flows through `Display for Error` will have color applied at render time but plain text in tests — safe to assert substrings.

### Message change is test-safe
Missing-config tests assert only `source == "io"` (`src/config.rs:111`, `src/commands/mod.rs:470`). No test asserts the message string. New wording won't break existing tests.

### Patterns to avoid
- **Do not** add fields to `Error` — the two-field struct is intentionally minimal; every `Error::new` site would need updating
- **Do not** change the `From<std::io::Error>` impl — it serves all io conversions; its contract (flatten to tag + strerror) is correct for other call sites like git spawn failures
- **Do not** pre-color the error message at construction — success path does this (`mod.rs:94`) but error convention is color-at-Display (`errors.rs:30-32`)
- **Do not** change resolution logic — `config_file_path` and `resolve_config_dir` behavior stays untouched; only the *recording* of how resolution happened is added
- **Do not** change the JSON envelope shape — `{"error": {"message", "source"}}` stays as-is; path lives in `message` text

## Design Decisions

1. **Attach path at `read_config`**: The function already owns the path as its parameter (`src/config.rs:40`). On `read_to_string` failure, catch the `io::Error` and construct `Error::new("io", &format!("could not read config file at {}: {}", path.display(), e))` instead of using `?`. This is the natural boundary — any future config-reading call site benefits without repeating the wrapping.

2. **Include resolution context**: `read_config` receives a second parameter indicating how the path was resolved. A small enum `ConfigPathSource { ExplicitFlag, XdgConfigHome, HomeDotConfig, AppData }` constructed at the `config_file_path` call sites (`src/commands/mod.rs:69-70,75-76`) and passed through. The error message appends a parenthetical: `"could not read config file at <path> (specified via --config): <io message>"` or `"could not read config file at <path> (resolved from XDG_CONFIG_HOME): <io message>"`.

3. **Everything in `message` text**: No new `Error` fields, no JSON schema changes. The path and resolution context are embedded in the `message` string, visible in both text and JSON output modes. The `source` tag stays `"io"`.

4. **No `Error` struct changes**: The two-field `Error { message, source }` (`src/errors.rs:10-12`) stays unchanged. All four non-test `Error::new` call sites remain untouched. The `From<io::Error>` impl (`src/errors.rs:39-46`) stays unchanged — it's correct for git spawn and other io conversions. Only `read_config` switches from `?`-propagation to explicit `Error::new` construction.

5. **Text-mode output format**: `Error from io:\ncould not read config file at <path> (<resolution>): <os error>` — the `source` line (yellow) stays `io`, the message line (red) gains the path and resolution. JSON output: `{"error": {"message": "could not read config file at <path> (<resolution>): <os error>", "source": "io"}}`.

## What We're NOT Doing

- **Not** adding structured path/resolution fields to `Error` or the JSON envelope
- **Not** changing resolution behavior — `config_file_path` / `resolve_config_dir` logic stays identical
- **Not** touching the `From<io::Error>` impl or any other `?`-based io error propagation site
- **Not** modifying the render chokepoint (`run_command` / `output_result`) — it stays agnostic to error internals
- **Not** adding new error source tags — `"io"` stays `"io"`
- **Not** changing the init path's error handling — init writes a config, it doesn't read one; only `step` hits this path
- **Not** adding Windows-specific error formatting — `%APPDATA%` resolution gets a `ConfigPathSource::AppData` tag and the same message pattern

## Open Risks

- **Fork stdlib `with_path` absence**: confirmed empirically (zero matches in the fork), but the fork divergence rationale is not documented. If a future toolchain update adds `with_path`, the `io::Error` text would gain a `": path"` suffix and we'd get a doubled path in the message. Mitigation: the `read_config` change catches and wraps the io error before propagation, so we control the text regardless.

- **Message length**: the combined message could be long for deeply nested paths or verbose OS errors. The codebase has no message-length policy; this is the first error message with two interpolated runtime values. Acceptable for a diagnostic error seen only on misconfiguration.

- **`ConfigPathSource` enum placement**: could live in `config_dir.rs` (near resolution logic) or `config.rs` (near `read_config`). Decision deferred to planning phase; both are reasonable and either keeps the changes local to 2-3 files.

- **Test coverage of new message content**: currently no test asserts missing-config message text. We will add a unit test in `src/config.rs` that asserts the path string appears in `err.message`, and an integration test that asserts stderr contains the resolved path. These are net-new assertions, not modifications.
