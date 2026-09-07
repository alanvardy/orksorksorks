# Structure Outline

## Approach

Add a `step` subcommand to the binary-only `orksorksorks` CLI: it reads a `[[steps]]` array-of-tables from config, derives the artifact directory from cwd + git branch, and returns the first (reverse-iteration) step whose `trigger_artifact` exists — every failure through the central `Error` type, output layer (`src/main.rs`) untouched.

Built bottom-up in five horizontal layers, each fully tested before the next starts. The "schema" here is the serde config shape (no DB); the "data-access" layer is the first production TOML read; the "service" layer is the pure-ish determine logic; the "transport" layer is the clap/handler wiring.

---

## Stage 1: Error foundation — `From<toml::de::Error>`

Delivers the deserialization error path the rest of the feature depends on. Green tests prove a `toml::de::Error` maps to source tag `"toml::de"` in the central type — the crate's first production TOML read error path.

**Files**: `src/errors.rs`

**Key changes**:
- `impl From<toml::de::Error> for Error` — new; source `"toml::de"`, message `e.to_string()` (mirrors the existing `From<toml::ser::Error>` at `src/errors.rs:48-55`)

**Tests**: extend `src/errors.rs` unit block — `from_toml_de_error_maps_to_tag` (force a parse failure, assert `source == "toml::de"`). No change to existing `From<io::Error>`/`From<toml::ser::Error>` tests.

**Verify**: `./scripts/test.sh` green.

---

## Stage 2: Config schema — `steps: Vec<Step>`

Delivers the `[[steps]]` array-of-tables shape without disturbing the pinned `init` bytes. Green tests prove fresh configs still serialize byte-identically (`version = "0.1.0"\n`) and `[[steps]]` round-trips.

**Files**: `src/config.rs`

**Key changes**:
- `Step { name: String, trigger_artifact: String }` — new; derive `Debug, Clone, Serialize, Deserialize, PartialEq` (match `Config`'s derives)
- `Config.steps: Vec<Step>` — new field, declared `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
- `impl Default for Config` — gains `steps: Vec::new()`

**Tests** (`src/config.rs`):
- existing `default_serializes_to_exact_version` must stay green (pins `tests/init_creates_file.rs:22-25`)
- new happy: `steps_round_trip` (multi-step config → `toml::to_string` → `toml::from_str` → equality)
- new sad-equiv: `missing_steps_deserializes_to_empty` (`version = "0.1.0"` with no `[[steps]]` → `steps.is_empty()`)

**Verify**: `./scripts/test.sh` green; confirm `init` bytes unchanged via existing `tests/init_creates_file.rs`.

---

## Stage 3: Config data-access — `read_config`

Delivers the first production disk-read + deserialize path. Green tests prove a `[[steps]]` file loads into `Config`, and both failure modes (missing file → `"io"`, malformed TOML → `"toml::de"`) ride the central error type.

**Files**: `src/config.rs`

**Key changes**:
- `pub fn read_config(path: &std::path::Path) -> Result<Config, Error>` — new; `std::fs::read_to_string(path)?` (→ `"io"`) then `toml::from_str(&contents)?` (→ `"toml::de"` via Stage 1)

**Tests** (`src/config.rs`, `tempfile::tempdir` sandbox — runtime dep used test-side only):
- happy: tempfile with a two-step `[[steps]]` TOML → `read_config` returns populated `Vec<Step>`
- sad: nonexistent path → `Error.source == "io"`; malformed TOML (`[[steps]` missing `name`) → `Error.source == "toml::de"`

**Verify**: `./scripts/test.sh` green.

---

## Stage 4: Determine-step logic — `determine_step`

Delivers the business rule in isolation: reverse-iterate steps, string-compose the trigger path onto the artifact dir, first `try_exists() == Ok(true)` wins. Green tests prove correctness without any CLI wiring.

**Files**: `src/commands/mod.rs`

**Key changes**:
- `fn determine_step(config: &Config, artifact_dir: &str) -> Result<String, Error>` — new; `for step in config.steps.iter().rev() { format!("{}{}", artifact_dir, step.trigger_artifact) }` (matches the string-composition convention at `src/commands/mod.rs:79-83`), `std::path::Path::try_exists()?` (`Err` → `"io"`); on no match `Err(Error::new("step", format!("{artifact_dir}: no trigger artifact matched")))`

**Tests** (`src/commands/mod.rs`, tempdir with a real `<artifact_dir>/<file>`):
- happy: one artifact present → that step's `name`
- happy: two steps, both artifacts present → last-forward step's `name` (reverse priority)
- sad: no artifact → `Error.source == "step"`

**Verify**: `./scripts/test.sh` green.

---

## Stage 5: CLI transport — `step` command

Delivers the user-facing command and end-to-end behavior. Green tests prove clap parses `step [--config]`, `select_command` routes it, the handler composes Stages 3–4, and the real binary returns the right name / exit 1 with no ANSI.

**Files**: `src/commands/mod.rs`, `tests/step.rs` (new)

**Key changes**:
- `Commands::Step { #[arg(long, value_name = "CONFIG", default_value = "orksorksorks.toml")] config: PathBuf }` — new variant (first value-carrying one)
- `select_command` — new match arm `Commands::Step { config } => step_command(config)`
- `pub fn step_command(config: PathBuf) -> Result<String, Error>` — new handler; resolves the config path against `std::env::current_dir()?`, calls `read_config`, then `determine_step(&cfg, &artifact_dir_path(&cwd, &current_branch()?))`; returns plain `name` (no ANSI — consistent with `branch`/`artifact_directory`)

**Tests**:
- unit (`src/commands/mod.rs`): `Cli::try_parse_from` parses `step` (default config) and `step --config custom.toml`; `select_command` routes `Step`
- integration (`tests/step.rs`, tier 2, spawn real binary): `git init -b <name>` tempdir, write `orksorksorks.toml` + `.pi/orksorksorks/<branch>/<artifact>`, `current_dir(temp.path())`; happy → correct `name`; none present → `.assert().failure()` (exit 1); `-j` → `{"data": "<name>"}` with no `\x1b`

**Verify**: `./scripts/test.sh` green; manual check: seed a git-repo CWD and run `cargo run -- step` / `cargo run -- step -j`.

---

## Testing Checkpoints

- After Stage 1: `cargo test src::errors` — `From<toml::de::Error>` → `"toml::de"` green.
- After Stage 2: `./scripts/test.sh` — `init` bytes still `version = "0.1.0"\n`; `[[steps]]` round-trips.
- After Stage 3: `./scripts/test.sh` — file loads to `Vec<Step>`; `"io"` and `"toml::de"` sad paths green.
- After Stage 4: `./scripts/test.sh` — reverse priority + no-match `"step"` error green.
- After Stage 5: `./scripts/test.sh` — clap parse, routing, and end-to-end integration green.

## Cross-cutting notes

- The reverse-iteration semantics assume **ordered progression** (last artifact = current step). If a later step's artifact can exist without earlier ones, the interpretation shifts silently — flagged in design Open Risks; the Stage 4 test asserting last-forward-wins documents the chosen reading. No code change to stage.
- No change to the output layer (`src/main.rs`), `Display`, or bell/exit-code policy — the JSON `{"error":{...}}` envelope stays unit-covered only (`src/errors.rs:91-98`); the Stage 5 no-match test asserts `.failure()` but not the envelope body (known gap, out of scope here).
- Config read is added in `src/config.rs` (data-access) rather than inside the handler so Stage 3 can be unit-tested before any CLI wiring exists.