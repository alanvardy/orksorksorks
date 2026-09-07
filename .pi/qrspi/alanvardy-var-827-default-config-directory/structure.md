# Structure Outline

## Approach

Relocate the `init` write from the CWD to a per-user config directory, resolved
hand-rolled (no new deps): `--config/-c <PATH>` override, else `$XDG_CONFIG_HOME`
(absolute only) → `$HOME/.config` on Unix, `%APPDATA%` on Windows; auto-create the
parent; report the resolved path; fail with `source: "config-dir"` when the directory
is unresolvable. `src/main.rs` is untouched (error envelope, exit codes stay stable).

---

## Layer 1: Config-directory resolution (`src/config_dir.rs`)

Pure resolution logic — the foundation every layer above consumes. One public
function resolves the target file path (explicit override passthrough, or default
dir + filename), returning the existing flat `Error` with the new `"config-dir"`
tag when no home is resolvable.

**Files**: `src/config_dir.rs` (new), `src/main.rs` (new `mod config_dir;`)

**Key changes**:
- `pub fn config_file_path(explicit: Option<&Path>) -> Result<PathBuf, Error>` — new
- `fn resolve_config_dir() -> Result<PathBuf, Error>` — private; the env-read + fallback logic
- `const FILE_NAME: &str = "orksorksorks.toml"` — the filename, joined under the dir
- Resolution contract: `XDG_CONFIG_HOME` set **and** absolute → use it; unset/empty/**relative**
  → `$HOME/.config` (all Unix incl. macOS); Windows → `%APPDATA%`; neither var →
  `Error::new("config-dir", "<clear message>")`. Explicit `Some(p)` returns `p` unchanged.
- No `src/errors.rs` code change: `Error::new` (errors.rs:18-23) already exists; the tag
  is a free string, so this module is the only producer of `"config-dir"`.

**Tests** (in-module `#[cfg(test)]`, `use super::*` — can reach `resolve_config_dir`):
- happy: absolute `XDG_CONFIG_HOME` → that dir + `FILE_NAME`
- happy: `XDG_CONFIG_HOME` unset + `HOME` set → `$HOME/.config/...`
- happy: empty `XDG_CONFIG_HOME` → fallback to `$HOME/.config`
- happy: relative `XDG_CONFIG_HOME` → ignored → `$HOME/.config`
- happy: `config_file_path(Some(p))` passthrough (default not consulted)
- sad: `HOME` unset **and** `XDG_CONFIG_HOME` unset → `Err` with `.source == "config-dir"`
- Note: env mutation is `unsafe` on 1.98 — `unsafe { std::env::set_var/remove_var }` scoped
  per test; safe because nextest runs each test in its own process. (Alternative: a private
  `resolve_from(env: impl Fn(&str) -> Option<String>, …)` thunk if `unsafe` is undesired.)

**Verify**: `./scripts/test.sh` green (fmt → check → clippy `-D warnings` → nextest → forbidden-string gate).

---

## Layer 2: CLI surface — `--config/-c` flag (`src/commands/mod.rs`)

Add the override input port as a per-variant arg on `Init`. No behavior change yet —
the flag is parsed but not consumed, so the suite stays green on top of Layer 1.

**Files**: `src/commands/mod.rs`

**Key changes**:
- `Commands::Init` unit variant → variant with fields:
  `Init { #[arg(short = 'c', long, value_parser = clap::value_parser!(PathBuf))] config: Option<PathBuf> }`
- `select_command` match arm updated to destructure: `Commands::Init { config }` (value ignored until Layer 3)

**Tests** (extend existing `cli_try_parse_*`, mod.rs:80-92):
- `cli_try_parse_accepts_config_short` — `["init", "-c", "/tmp/x.toml"]` → `Some(PathBuf)`
- `cli_try_parse_accepts_config_long` — `["init", "--config", "/tmp/x.toml"]` → `Some`
- `cli_try_parse_init_without_config` — `["init"]` → `None`

**Verify**: `./scripts/test.sh` green (Layer 1's tests still green beneath it).

---

## Layer 3: Handler — resolve, create parent, report resolved path (`src/commands/mod.rs` + tests)

The behavior land: `select_command` resolves (Layer 1) and hands the concrete path to
`init_command`, which creates the parent, truncate-writes, and reports the resolved path.
This **changes the default location**, so the four CWD-coupled integration tests are
repointed in this same stage (tests ship alongside the code that breaks them).

**Files**: `src/commands/mod.rs`, `tests/init_creates_file.rs`

**Key changes**:
- `select_command`: `Commands::Init { config } => { let path = config_file_path(config.as_deref())?; init_command(&path) }`
- `fn init_command(path: &Path) -> Result<String, Error>` — signature change (was `fn init_command()`, mod.rs:52)
  - `if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }`
  - `File::create(path)?` + `write_all`/`flush`/`sync_all` (unchanged, still `From<std::io::Error>`)
  - `Ok(green_string(&format!("✓ Created {}", path.display())))` — resolved path in the message

**Tests** (repointed alongside; `cmd.env("XDG_CONFIG_HOME", <tempdir subdir>)` — no new mechanism):
- rewrite `select_command_routes_init` (mod.rs:70-78): tempdir-scoped, no runner-CWD write
- `init_creates_orksorksorks_toml_with_default_content` (:11-26): asserts content at `<$XDG>/orksorksorks.toml`
- `init_returns_success_message` (:29-38): contains `✓ Created <resolved tempdir path>`
- `init_in_readonly_dir_returns_error_exit_code` (:41-56) and `init_json_error_in_readonly_dir_shows_source_io` (:59-78): readonly applied to the **target parent** (not the CWD), still asserting `.failure()` / `"source":"io"`

**Verify**: `./scripts/test.sh` green; spot-check `XDG_CONFIG_HOME=/tmp/x cargo run -- init` prints the `/tmp/x` path.

---

## Layer 4: Integration hardening — override, unresolvable home, regression

Additive coverage pinning the two behaviors the middle layers didn't yet assert
end-to-end: the `--config` override and the `"config-dir"` failure. Regression-confirms
the untouched JSON/text envelopes.

**Files**: `tests/init_creates_file.rs` (new cases), `tests/json_output.rs` (unchanged — regression)

**Key changes**: no production code — pure test-layer additions.

**Tests**:
- `init_with_config_flag_writes_to_given_path` — `--config <tempdir>/x.toml` with a *hostile*
  `XDG_CONFIG_HOME` set → file lands exactly at `x.toml`, default dir not consulted
- `init_without_home_fails_with_config_dir_tag` — child spawned with `env_remove("HOME")` +
  `env_remove("XDG_CONFIG_HOME")` → exit 1; `-j` stdout contains `"source":"config-dir"`; text
  output is ANSI-free (color choke still applies)
- `init_json_returns_valid_json_with_data_field` / `init_no_json_prints_plain_text` (json_output.rs) — unchanged, still green

**Verify**: `./scripts/test.sh` green + manual `cargo run -- init --config /tmp/foo.toml`
and `cargo run -- init -j`, then `cat /tmp/foo.toml`.

---

## Testing Checkpoints

- After **Layer 1**: `src/config_dir.rs` unit tests green; resolution + `config-dir` tag proven.
- After **Layer 2**: clap parse unit tests green; `-c`/`--config`/absent all parse correctly.
- After **Layer 3**: handler + repointed integration tests green; default resolves under `XDG_CONFIG_HOME`, parent auto-created, message shows the resolved path.
- After **Layer 4**: override + unresolvable-home + JSON/text regression green; `./scripts/test.sh` fully passes.

## Cross-cutting notes

- The error-tag `"config-dir"` requires **no** `src/errors.rs` change — `Error::new` and both
  render paths handle it automatically (research Q4); it is only *produced* in Layer 1 and
  *asserted* in Layer 4.
- Only cross-layer coupling is the success-message string (Layer 3 produces, tests pin it);
  keep it stable or update Layer 4's assertions in the same change.
- Committed repo-root `orksorksorks.toml` becomes orphaned by this change (design "Open Risks").
  Out of scope here — flag for a follow-up decision, not a Stage.