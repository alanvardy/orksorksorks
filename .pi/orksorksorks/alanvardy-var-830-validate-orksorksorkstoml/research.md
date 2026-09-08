# Research Findings

> Post-rebase. Branch now sits on `main` @ `86e6483` (the `models`/`model`/`thinking`/`prompt` work merged in).

## Q1: Config schema — what `orksorksorks.toml` supports

### Findings
- **Four serde structs** define the schema, all in `src/config.rs` (binary-only crate; no `[lib]` target, `Cargo.toml:1-8`).
- **`Step`** (`src/config.rs:7-14`): `name` (`:9`, "Human-readable name returned by the `step` subcommand"), `trigger_artifact` (`:11`, "Filename whose presence at the artifact directory marks this step"), and `model` (`:13`, "Name of the model (a key into `Config::models`) used at this step"). All three plain required `String`s.
- **`Model`** (`src/config.rs:18-25`): `name` (`:20`, "Key that `Step::model` references"), `model` (`:22`, "Concrete model identifier, e.g. `openrouter/...`"), `thinking` (`:24`, "Reasoning-budget hint").
- **`Prompt`** (`src/config.rs:29-34`): `name` (`:31`, "Step name this prompt belongs to; also the lookup key") and `content` (`:33`, "Full prompt text (a TOML multi-line string)").
- **`Config`** (`src/config.rs:41-53`): `version: String` (`:43`, **required**, no serde default) plus three optional vecs — `steps: Vec<Step>` (`:46`), `models: Vec<Model>` (`:49`), `prompts: Vec<Prompt>` (`:52`) — each optional via `#[serde(default, skip_serializing_if = "Vec::is_empty")]`; a missing key deserializes to `Vec::new()`.
- **No `Option<T>` fields, no `#[serde(rename/alias/...)]`** anywhere; TOML keys equal Rust field names: top-level `version`, `[[steps]]`, `[[models]]`, `[[prompts]]` arrays-of-tables.
- **Two name-keyed cross-references, both linear lookups** (see Q3): `step.model` → `models[].name` (`resolve_model` `src/commands/mod.rs:210-217`); `step.name` → `prompts[].name` (`resolve_prompt` `src/commands/mod.rs:269-276`, doc "keyed by step name"). `steps[].name` also drives step selection (Q3). No name→step map exists; steps are matched by `trigger_artifact` presence.
- Empty `trigger_artifact` = the `""`-means-default convention (`src/commands/mod.rs:186-192`).
- `Config::default()` (`src/config.rs:55-64`) = `version: "0.1.0"`, empty steps/models/prompts — the `init` template. Because of `skip_serializing_if`, `init` still writes **exactly `version = "0.1.0"\n`** (`init_command` `src/commands/mod.rs:132-151`; pinned at `tests/init_creates_file.rs:11`, `:102-108`).
- Dependencies: `serde` (derive), `toml`, `serde_json` (`Cargo.toml:13,15,17`). Deserialize via `toml::from_str` (`src/config.rs:86`), serialize via `toml::to_string` (`src/commands/mod.rs:140`).

## Q2: Validation surface beyond serde parse-time

### Findings
- **There is still no post-parse validation.** Validation = serde derive-based type enforcement inside `toml::from_str` only (`src/config.rs:86`).
- No `impl Deserialize` blocks, no `deserialize_with`, no `deny_unknown_fields`, no `validate()` method, no `TryFrom`/`FromStr` for `Config`.
- **No duplicate-name, duplicate-artifact, ordering, or cross-section checks.** No `HashMap`/`HashSet` anywhere in `src/`.
- Ordering is *semantically loaded but never validated*: `determine_step` reverse-iterates (`src/commands/mod.rs:183-203`) so the last list entry wins on artifact overlap — pinned as intentional by `determine_step_prefers_last_step_in_reverse` (`src/commands/mod.rs:457`).
- Empty `trigger_artifact` = default step is behavior, not validation (`src/commands/mod.rs:186-192`).
- `version` is never checked against `"0.1.0"` or any format after parse (`src/config.rs:43`); it is compared only in round-trip tests.
- Cross-reference failures surface **at command time, not load time**: `resolve_model` → `Error::new("model", …)` (`src/commands/mod.rs:216`), `resolve_prompt` → `Error::new("prompt", …)` (`:275`). A dangling `step.model` only fails `model`/`thinking` (`:240-267`); a missing prompt only fails `prompt` (`:281-298`); `step` (`:226-237`) touches neither section and never notices.
- No tests assert rejection of semantically-problematic configs; the only failure-path tests are parse-time: `read_config_malformed_toml_tags_toml_de` (`src/config.rs:237`) and `read_config_missing_file_tags_io` (`:219`).
- Where post-parse checks would naturally live if added: no existing hook — `read_config` returns the parsed `Config` directly (`src/config.rs:71-88`) and the four command fns consume it immediately (`src/commands/mod.rs:226`, `:240`, `:255`, `:281`). The only semantic logic currently running post-parse is `determine_step` and the two `resolve_*` lookups.

## Q3: Config load + use flow

### Findings
- **Entry**: `#[tokio::main] async fn main()` → `commands::Cli::parse()` → `run_command(cli)` (`src/main.rs:62-79`).
- **Parser**: clap derive. `Cli` root (`src/commands/mod.rs:30-35`) carries global `-j/--json` (`:31`) and the `Commands` subcommand enum (`:41-84`): `Init { -c/--config config }` (`:43-48`), `Branch` (`:50`), `ArtifactDirectory` (`:54`), `Step { --config }` (`:57-63`), `Model { --config }` (`:65-71`), `Thinking { --config }` (`:73-79`), `Prompt { step_name: Option<String>, --config }` (`:81-84` — the step-name override is the only positional arg).
- **Dispatch**: `select_command` (`:127`) → `select_command_with_env(cli, &ConfigEnv::from_env())` (`:95-124`). Config-reading arms resolve the path (`config_file_path_with_env`) then call `step_command` (`:103`), `model_command` (`:108`), `thinking_command` (`:113`), or `prompt_command` (`:118`, forwarding `step_name.clone()`). `Init` discards the resolution source (`:97-100`); `Branch`/`ArtifactDirectory` never touch config.
- **Env capture** (`src/config_dir.rs`): `ConfigEnv` (`:16-21`) with `from_env()` (`:26-40`) reading `XDG_CONFIG_HOME` (all platforms), `HOME` (non-Windows), `APPDATA` (Windows only). Injected so tests construct `ConfigEnv` directly.
- **Path resolution** — `config_file_path_with_env` (`src/config_dir.rs:105-113`): explicit `--config` passes through unchanged → `ConfigPathSource::ExplicitFlag`; otherwise `resolve_config_dir_with_env` (`:71-98`): `XDG_CONFIG_HOME` only when absolute (`:76-80`), then `APPDATA` as-is, then `$HOME/.config`, else `Error::new("config-dir", …)`. Filename `FILE_NAME = "orksorksorks.toml"` (`:9`).
- **`read_config`** (`src/config.rs:71-88`): `std::fs::read_to_string` (`:72`); failure → `Error::new("io", "could not read config file at {path} ({source}): {err}")` (`:75-84`); `toml::from_str::<Config>(&contents)` (`:86`), errors mapped via `From<toml::de::Error>` (`src/errors.rs:57-64`).
- **Consumption** — all four commands call `read_config(path, source)?` first, before any git/artifact work (doc comment `src/commands/mod.rs:226-232`), so a missing/unreadable config fails deterministically with `"io"`. Then:
  - `step_command` (`:226-237`) → `determine_step(&cfg, &artifact_dir)?.name`.
  - `model_command` (`:240-250`) → `determine_step(...)` → `resolve_model(&cfg, &step.model)?.model`.
  - `thinking_command` (`:255-267`) → `determine_step(...)` → `resolve_model(...)?.thinking`.
  - `prompt_command` (`:281-298`) → explicit `step_name` if given, else `determine_step(...)?.name`; then `resolve_prompt(&cfg, &name)`.
- `determine_step` (`:183-203`) reverse-iterates `steps`; empty `trigger_artifact` → default fallback, first-seen-in-reverse wins (`:186-192`); else composes `"{artifact_dir}{trigger_artifact}"` and returns the first whose path `try_exists()` (`:193-199`); no match + no default → `Error::new("step", "{artifact_dir}: no trigger artifact matched")` (`:200-204`). It returns the **`Step`** (not just name) so callers can read `name` vs `model`.
- Artifact dir = `"{cwd}/.pi/orksorksorks/{branch-with-/→-}/"` (`artifact_dir_path` `:155-162`; trailing slash is contractual).
- **`version` is never consumed** at runtime — only written by `init` (`:132-151`) and round-tripped in unit tests. `init` never calls `read_config`.

## Q4: Error handling conventions

### Findings
- **Single error type** `crate::errors::Error` — `src/errors.rs:10-13`: `{ message: String, source: String }`. Derived `Debug, Clone, PartialEq, Eq, Serialize` (`:9`).
- **`source` is a free-form lowercase tag** (string, not enum) — doc at `src/errors.rs:4-8`. Existing tags: `"io"` (`From<std::io::Error>` `:39-46`; manual with path at `src/config.rs:75-84`), `"toml::ser"` (`:48-55`), `"toml::de"` (`:57-64`), `"step"` (`src/commands/mod.rs:201-204`), `"git"` (`src/git.rs:30,34`), `"config-dir"` (`src/config_dir.rs`), `"model"` (`src/commands/mod.rs:216`), `"prompt"` (`src/commands/mod.rs:275`).
- **Constructor** `Error::new(source, message)` (`src/errors.rs:18-23`); convention is `&format!(...)` for parameterized messages (`:16-17`).
- **Text display** — `Display` impl (`src/errors.rs:26-35`) renders `"Error from {yellow(source)}:\n{red(message)}"`; **all** ANSI coloring lives in `Display`, routed through the single chokepoint `src/format.rs` (`apply_color` `:6-13`, helpers `:14-23`), which strips color under `cfg!(test)`. Callers must not pre-apply ANSI.
- **Output routing** (`src/main.rs`): text errors → `eprintln!("\n\n{e}")` **stderr** with two leading blank lines (`:33`); text success → stdout (`:30`). JSON errors → **stdout** as `{"error": {"message": ..., "source": ...}}` (`:47`); JSON success → `{"data": ...}` (`:43`). Dispatch on the global `-j` flag (`output_result` `:53-59`).
- **Exit code**: exactly one — `std::process::exit(1)` on any `Err` (`src/main.rs:77`). No other exit paths.
- **Path/resolution context** is embedded in the *message string*, not a context type: config read errors use `"could not read config file at {path} ({ConfigPathSource display}): {io_err}"` (`src/config.rs:75-84`); resolution-source phrases pinned by `ConfigPathSource`'s `Display`: `"specified via --config"`, `"resolved from XDG_CONFIG_HOME"`, `"resolved from HOME/.config"`, `"resolved from %APPDATA%"`. The step no-match error uses a `"{artifact_dir}: ..."` prefix.
- A new config-validation error would need, to conform: `Error::new(<lowercase-tag>, ...)` (custom tag string is the norm), `Display`/JSON envelope/exit-1 all follow automatically from returning `Err(Error)` (`src/main.rs:33,47,77`), and path-bearing messages should embed `path.display()` + resolution phrase per precedent.
- `#![warn(missing_docs)]` at `src/main.rs:6` — public items need doc comments.

## Q5: Testing conventions

### Findings
- **Two layers, cleanly split.**
  1. **In-module unit tests** — `#[cfg(test)] mod tests` blocks in `src/config.rs`, `src/config_dir.rs`, `src/commands/mod.rs`, `src/errors.rs`, `src/git.rs`, `src/format.rs`. Config-parse tests in `src/config.rs`: default serialization (`:96`), round-trips (`:104`, `:112`, `:160`, `:202`), missing-section leniency (`:136` asserts steps/models/prompts all empty), disk load marker fields (`:144` asserts `steps[0].model`), prompts disk load (`:176`), models round-trip (`:202`), io/toml-de error tagging (`:219`, `:237`, `:251`). Config-dir resolution tests at `src/config_dir.rs:121-211` (injected `ConfigEnv`, no env mutation).
  2. **CLI integration tests** in `tests/` — **seven** files, all driving the compiled binary via `assert_cmd::Command::cargo_bin("orksorksorks")` inside disposable `tempfile::tempdir()` git repos: `step.rs`, `model.rs`, `prompt.rs`, `init_creates_file.rs`, `json_output.rs`, `branch.rs`, `artifact_directory.rs`. The crate is binary-only, so integration tests **cannot import crate types**.
- **TOML samples: inline `std::fs::write` string literals — no fixture files.** Unit: `std::fs::write(&path, "...")` or direct `toml::from_str`. Integration: `concat!` string-literal helpers — `tests/step.rs:18-35` (steps only), `tests/model.rs:20-44` (steps+models), `tests/prompt.rs:24-51` (steps+prompts). **Crucially, these fixtures are *partial*** — each writes only the sections its command touches; none is a complete steps+models+prompts config. The only exact-file-content assertion is `init` output `"version = \"0.1.0\"\n"` (`tests/init_creates_file.rs:11`, `:108`).
- **Hermetic git**: each integration test makes its own repo (`git init -b` at `tests/step.rs:7-16`, `tests/model.rs:7-15`, `tests/prompt.rs:7-13`; `git_repo_on_branch` helpers in `tests/branch.rs`/`tests/artifact_directory.rs`) because CI `actions/checkout` leaves detached HEAD. `tests/artifact_directory.rs:40-43` canonicalizes the tempdir (macOS `/var` → `/private/var`).
- **Runner/CI**: `.config/nextest.toml:1-9` — `[profile.ci]` (`retries = 2`, `fail-fast = false`, `slow-timeout = {period = "60s"}`, JUnit at `target/nextest/ci/junit.xml`); no default profile. Local gate `scripts/test.sh`: `cargo fmt --all`, `cargo check`, `cargo clippy --tests -- -D warnings`, `cargo nextest run --no-tests pass` (binary-only crate), and a forbidden-strings `rg 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:'` gate. CI `.github/workflows/ci-pr.yml:4-11` → reusable lint + test (ubuntu-latest, `cargo nextest run --profile ci --all-features --no-tests pass`). Coverage `codecov.yml:1-12`: `src/main.rs` ignored, **patch `target: 50%`** — new lines need ≥50% coverage.
- **Error-message pinning (three granularities)**: (1) `source` tag equality — `assert_eq!(err.source, "io")` (`src/config.rs`), `"toml::de"`, `"config-dir"`, `"step"` (`src/commands/mod.rs:644`), `"git"`, `"model"` (`src/commands/mod.rs:685`), `"prompt"` (`src/commands/mod.rs:884`); literal JSON fragments `r#""source":"io""#` (`tests/init_creates_file.rs:84`). (2) Exact phrase pinning where static — `config_path_source_display_pins_phrases` pins all four resolution phrases verbatim; `contains` on `"no trigger artifact matched"`, `"no model named"` (`src/commands/mod.rs:686`), `"no prompt named"` (`:885`), `"detached HEAD"`. (3) CLI-level double-pinning of path + resolution phrase through stdout/stderr/JSON (`tests/step.rs`, `tests/init_creates_file.rs`).
- **Universal regression guards**: no-ANSI (`!stdout.contains('\x1b')`) in every integration test; `.assert().success()/.failure()` on exit codes; stdout equality after `trim_end()`; `step_without_flag_ignores_cwd_config` guard (`tests/step.rs:206`).

## Q6: Schema references outside the crate

### Findings
- **No README, no `docs/`, no example/sample `orksorksorks.toml`** exists anywhere — in the working tree, any branch, or git history. `find . -name '*.toml'` yields only `Cargo.toml`, `rust-toolchain.toml`, `.config/nextest.toml`. `linear-project.md:1` holds only the project name.
- CI workflows, `scripts/`, and the manifest carry no schema: `Cargo.toml:1-8` (`[package]` has no `description`), `.github/workflows/*` lint/test only.
- **The schema's cross-references are defined only in code** (no external doc): `Prompt` is "keyed by step name" (`src/config.rs:29-34`), `Step.model` is "a key into `Config::models`" (`:12-13`), enforced by the two linear lookups `resolve_model` (`src/commands/mod.rs:210-217`) and `resolve_prompt` (`:269-276`). Neither lookup enforces uniqueness, length, or order — they just fail the specific command when a name is absent.
- **The only statement of intended validation is the ticket docs** committed in this artifact directory (`.pi/orksorksorks/alanvardy-var-830-validate-orksorksorkstoml/`): `steps` names must correspond to `prompts` names, duplicates rejected — refined in `design.md` to membership-only (no length/order), plus model-ref coverage and trigger/name hygiene.

## Cross-Cutting Observations
- **Config knowledge is split across three modules**: `config.rs` (schema + file I/O), `config_dir.rs` (path/`ConfigPathSource` resolution), `commands/mod.rs` (consumption in `determine_step`/`resolve_model`/`resolve_prompt`). Nothing outside `src/commands/mod.rs` reads config values.
- **Two name-keyed linear lookups** are the only cross-section enforcement today: `resolve_model` (`:210-217`) and `resolve_prompt` (`:269-276`), each `O(n)` over its list, each `Error::new` on miss.
- **All config failures funnel through one chokepoint**: `Error { message, source }` (`src/errors.rs:10-13`), surfaced by the single output layer (`src/main.rs`) with exit code 1. New failure modes need only a new lowercase `source` tag.
- **Path context is message-embedded, not structured** (resolution phrase + `path.display()` in the string).
- **`cfg!(test)` gates observability**: ANSI stripping (`src/format.rs:6-13`) and `cfg!(windows)` env collection are the only test/platform conditionals.
- **Ordering is load-bearing**: `[[steps]]` order + reverse iteration is the selection semantics — last entry wins; empty `trigger_artifact` is a default (never validated as unique).
- **Testing separation is strict**: pure/decision functions get in-module unit tests; anything crossing a process boundary (CLI, git, fs) is tested by driving the binary in a fresh tempdir repo; env is injected (`ConfigEnv`) rather than mutated.

## Open Areas
- **No validation entry point exists** — semantically there is no `validate()`/hook; the parse-return-consume path is `read_config` (`src/config.rs:71-88`) → four command fns (`src/commands/mod.rs:226/240/255/281`).
- **`version` semantics undefined**: nothing enforces `"0.1.0"` or any format (`src/config.rs:43`).
- **Duplicate tolerance today**: duplicate step/prompt/model names and duplicate `trigger_artifact`s are accepted; duplicates resolve by reverse priority in `determine_step` (`:183-203`) and first-match in `resolve_*`. Integration tests never assert duplicate configs fail.
- **Partial fixtures everywhere**: `tests/step.rs`/`tests/model.rs`/`tests/prompt.rs` write section-partial configs that any strict cross-section validation would reject — flagged in `design.md` Open Risks as required fixture churn.