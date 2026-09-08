# Structure Outline

## Approach

When the config file can't be read, embed the resolved path **and** how it was resolved into the `"io"` error's `message` text (no `Error` struct fields, no JSON schema change, `source` stays `"io"`). Tag the resolution branch with a new `ConfigPathSource` enum, and replace `read_config`'s `?`-propagation with an explicit `Error::new("io", …)` that composes path + resolution phrase + OS error.

**Decomposition note.** This design has no migration, store, or transport surface — it's a 3-file diagnostic-message change. Three horizontal layers is the honest count, not five. No cross-cutting change needs a top-layer stub: every layer below is independently testable and green before the next starts. No child tickets; all work on the main ticket.

---

## Stage 1: `ConfigPathSource` type + resolution-source tracking (bottom)

Delivers the source-tagging type and proves the resolution logic can identify *which* branch produced the path — the risky "which branch" logic, isolated and unit-tested before any message text depends on it. `config_file_path` starts returning `(path, source)`; the two dispatch call sites discard it with `_` (behavior unchanged).

**Files**: `src/config_dir.rs` (enum + signature), `src/commands/mod.rs` (2-line destructure change)

**Key changes**:
- `pub enum ConfigPathSource { ExplicitFlag, XdgConfigHome, HomeDotConfig, AppData }` — new
- `impl std::fmt::Display for ConfigPathSource` — new; renders `"specified via --config"` / `"resolved from XDG_CONFIG_HOME"` / `"resolved from HOME/.config"` / `"resolved from %APPDATA%"`
- `fn resolve_config_dir() -> Result<(PathBuf, ConfigPathSource), Error>` — modified (was `Result<PathBuf, Error>`)
- `pub fn config_file_path(explicit: Option<&Path>) -> Result<(PathBuf, ConfigPathSource), Error>` — modified (was `Result<PathBuf, Error>`)
- `select_command` (`mod.rs:69-76`): init arm `let (path, _) = config_file_path(config.as_deref())?;`, step arm `let (path, _) = …?;` — source dropped at this stage

**Tests**: update `src/config_dir.rs` unit tests to assert the source variant alongside the path — XDG-absolute → `XdgConfigHome` (`:55-95`), Windows APPDATA → `AppData`, `$HOME/.config` → `HomeDotConfig`, explicit passthrough → `ExplicitFlag` (`:102`), unresolved-home still errors `"config-dir"` (`:107-112`). New: a `Display` test pinning all four phrase literals. `src/commands/mod.rs` routing tests (`:457-471`, `:186-190`) stay green unchanged.

**Verify**: `scripts/test.sh` green (canonical gate); scoped: `cargo nextest run config_dir::`

---

## Stage 2: Enriched `"io"` error in `read_config` (logic)

Delivers the actual message change: `read_config` catches the io error and composes `"could not read config file at <path> (<phrase>): <os error>"`, preserving the OS strerror and keeping `source = "io"`. Source is threaded from `config_file_path` through `step_command` to `read_config`.

**Files**: `src/commands/mod.rs` (capture + thread source), `src/config.rs` (error composition)

**Key changes**:
- `fn step_command(path: &Path, source: ConfigPathSource) -> Result<String, Error>` — modified (was `step_command(path: &Path)`)
- `pub fn read_config(path: &Path, source: ConfigPathSource) -> Result<Config, Error>` — modified; the `read_to_string(path)?` becomes a `match` whose `Err(e)` arm returns `Error::new("io", &format!("could not read config file at {} ({}): {}", path.display(), source, e))`; the `toml::from_str(&contents)?` line is untouched (malformed TOML still `"toml::de"`)
- `select_command` step arm (`mod.rs:75-76`): `let (path, source) = config_file_path(config.as_deref())?;` then `step_command(&path, source)` — init arm stays `_` (init writes, never reads)

**Tests**: update `read_config_missing_file_tags_io` (`src/config.rs:107-112`) to also assert `err.message` contains the resolved path string and `"specified via --config"`; add a second `read_config` test with an `XdgConfigHome` source asserting the path + `"resolved from XDG_CONFIG_HOME"`. Existing `select_command_routes_step` (`mod.rs:457-471`) still asserts only `source == "io"` — green unchanged.

**Verify**: `scripts/test.sh` green; scoped: `cargo nextest run config::`

---

## Stage 3: End-to-end output verification (presentation)

Delivers proof that the path + resolution text actually reaches the user through both renderers (text stderr via `Display`, JSON stdout via the raw `message` field), with no ANSI and no JSON-shape change.

**Files**: `tests/step.rs`, `tests/json_output.rs`

**Key changes**: none to `src/` — this layer is tests only.

**Tests** (net-new, via `assert_cmd::Command::cargo_bin("orksorksorks")`):
- text mode: `step --config /tmp/nope.toml` → stderr contains `/tmp/nope.toml` and `"specified via --config"`
- env mode: `step` with `XDG_CONFIG_HOME=/bad/dir` (config absent) → stderr contains `/bad/dir/orksorksorks/orksorksorks.toml` and `"resolved from XDG_CONFIG_HOME"`
- JSON mode: `step --config /tmp/nope.toml -j` → stdout reparses as `{"error":{"message":…,"source":"io"}}` with the path text inside `message`, no `'\x1b'`
- Existing `step_without_flag_ignores_cwd_config` (`tests/step.rs:198-213`) stays green (still `.failure()`)

**Verify**: full `scripts/test.sh` green (final gate — fmt/check/clippy `-D warnings`/nextest/forbidden-strings). Manual: `cargo run -- step --config /tmp/nope.toml` and `XDG_CONFIG_HOME=/bad/dir cargo run -- step` show the enriched message in both text and `-j` modes.

---

## Testing Checkpoints

Resume points if context resets — the listed command must be green before advancing:

1. After Stage 1: `scripts/test.sh` green — `ConfigPathSource` exists, resolution tests assert correct source per branch.
2. After Stage 2: `scripts/test.sh` green — `read_config` error `message` contains path + resolution phrase, `source == "io"`.
3. After Stage 3: `scripts/test.sh` green — integration stderr/JSON assertions pass, no ANSI, no JSON-shape change.

## Open risks carried from design (no structural impact)

- **Fork stdlib lacks `with_path`** — mitigated: `read_config` wraps the io error before propagation, so we control the text regardless of future toolchain changes.
- **Enum placement** — placed in `config_dir.rs` (near resolution logic); `config.rs` imports it. Both were viable; this keeps `read_config` a thin importer.
- **Message length** — first message with two runtime interpolations; acceptable for a misconfiguration-only diagnostic, no length policy exists.
