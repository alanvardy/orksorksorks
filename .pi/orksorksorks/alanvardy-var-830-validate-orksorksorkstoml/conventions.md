# Conventions — orksorksorks

Factual appendix for Structure/Plan. Dense anchors; do not re-open the source tree for these facts.

## Canonical commands

- **Local gate** — `scripts/test.sh` (`:1-21`, `set -euo pipefail`), in order:
  1. `cargo fmt --all` (`:5`)
  2. `cargo check` (`:8`)
  3. `cargo clippy --tests -- -D warnings` (`:11`)
  4. `cargo nextest run --no-tests pass` (`:14`) — nextest with the **default profile**; `--no-tests pass` keeps the binary-only crate green when there are no tests.
  5. Forbidden-strings gate: `rg -i -g '*.rs' 'TODO:|todo:|FIXME|fixme|dbg!|DEBUG:|FIXTURE:' .` → `exit 1` on any hit (`:16-19`).
- **CI** — `.github/workflows/ci-pr.yml:4-11` (PR→main) → reusable lint `_reusable-lint.yml` (`cargo check --locked --all-features`; `cargo fmt --all -- --check`; `cargo clippy --all-targets --all-features --locked -- -D warnings`; same forbidden-strings rg) and reusable test `_reusable-test.yml` on ubuntu-latest: installs `cargo-nextest,cargo-llvm-cov` via `taiki-e/install-action`, then `cargo nextest run --profile ci --all-features --no-tests pass`. Coverage only when `upload-coverage=true`: `cargo llvm-cov nextest --profile ci --all-features --lcov --output-path lcov.info`, uploaded with `codecov/codecov-action@v5`; JUnit via `codecov/test-results-action@v1` from `target/nextest/ci/junit.xml`.
- **Install** — `scripts/install.sh:5`: `cargo install --path . --locked`.
- **Nextest profile** — `.config/nextest.toml:1-9`: only `[profile.ci]` (`retries = 2`, `fail-fast = false`, `slow-timeout = { period = "60s" }`, `:1-5`) + `[profile.ci.junit]` (`path = "target/nextest/ci/junit.xml"`, store-success-output false, store-failure-output true, `:6-9`). **No `[profile.default]`** — local runs use nextest defaults.
- **Coverage** — `codecov.yml:1-12`: `ignore: ["src/main.rs"]` (`:2`); project status `target: auto`, `threshold: 10%` (`:5-9`); **patch status `target: 50%`** (`:12`) — changed/new lines must be ≥50% covered.
- Toolchain `rust-toolchain.toml` (channel/components only). Crate is **edition 2024, binary-only** (no `[lib]`): `Cargo.toml:1-8`.

## Test suite inventory

### Unit tests (in-module `#[cfg(test)] mod tests`)
| File | Test | Covers | Anchor |
|---|---|---|---|
| `src/config.rs` | default_config_serializes_to_expected_toml | default serializes `version`/`"0.1.0"` | `:96` |
| `src/config.rs` | config_round_trip_serialize_deserialize | serde round-trip of default | `:104` |
| `src/config.rs` | steps_round_trip | hand-built `Config`/`Step` round-trip (now incl. `model`) | `:112` |
| `src/config.rs` | missing_steps_deserializes_to_empty_vec | `#[serde(default)]` leniency — steps/models/prompts all empty | `:136` |
| `src/config.rs` | read_config_loads_steps_from_disk | file → parse → fields incl. `steps[0].model` | `:144` |
| `src/config.rs` | prompts_round_trip | `Prompt` round-trip | `:160` |
| `src/config.rs` | read_config_loads_prompts_from_disk | `[[prompts]]` multi-line `content` | `:176` |
| `src/config.rs` | models_round_trip | `Model` round-trip | `:202` |
| `src/config.rs` | read_config_missing_file_tags_io | `source == "io"` + path + `"specified via --config"` | `:219` |
| `src/config.rs` | read_config_malformed_toml_tags_toml_de | missing required `name` → `"toml::de"` | `:237` |
| `src/config.rs` | read_config_io_error_contains_xdg_source | `"resolved from XDG_CONFIG_HOME"` in message | `:251` |
| `src/config_dir.rs` | absolute_xdg_is_used | XDG absolute wins | `:121` |
| `src/config_dir.rs` | unset_xdg_falls_back_to_home_dot_config | unset → `$HOME/.config` | `:132` |
| `src/config_dir.rs` | empty_xdg_falls_back_to_home | empty → fallback | `:143` |
| `src/config_dir.rs` | relative_xdg_falls_back_to_home | relative → fallback | `:155` |
| `src/config_dir.rs` | explicit_path_passthrough | `--config` passthrough + ExplicitFlag | `:167` |
| `src/config_dir.rs` | appdata_used_on_windows | APPDATA raw, no join | `:175` |
| `src/config_dir.rs` | unresolved_home_yields_config_dir_error | `"config-dir"` tag | `:190` |
| `src/config_dir.rs` | config_path_source_display_pins_phrases | 4 Display phrases verbatim | `:196` |
| `src/commands/mod.rs` | cli_try_parse_* (~20 tests) | clap parse matrix (no subcommand; init -c/--config; branch; artifact_directory incl. kebab rejection; step/model/thinking/prompt ± --config; prompt step-name) | `:324-427`, `:732-830` |
| `src/commands/mod.rs` | select_command_routes_init | env-injected init writes file | `:306` |
| `src/commands/mod.rs` | select_command_routes_step | missing config → `"io"` proving step arm | `:615` |
| `src/commands/mod.rs` | select_command_routes_model / _thinking / _prompt | missing config → `"io"` proving each arm | `:686`, `:702`, `:835` |
| `src/commands/mod.rs` | artifact_dir_path_* (5 tests) | trailing slash, `/`→`-`, no ANSI | `:398-425` |
| `src/commands/mod.rs` | determine_step_* (6 tests) | reverse priority, default semantics | `:432-582` |
| `src/commands/mod.rs` | determine_step_no_match_errors_with_step_tag | `"step"` + `"no trigger artifact matched"` | `:631` |
| `src/commands/mod.rs` | resolve_model_returns_model_for_matching_name / resolve_model_missing_name_errors_with_model_tag | name→`Model` lookup + `"model"` tag | `:654`, `:673` |
| `src/commands/mod.rs` | resolve_prompt_returns_content_for_matching_name / resolve_prompt_missing_name_errors_with_prompt_tag | name→content lookup + `"prompt"` tag | `:855`, `:872` |
| `src/errors.rs` | display_includes_source_and_message | Display shape | `:72` |
| `src/errors.rs` | from_io_error_tags_io / from_toml_ser_error_tags_toml_ser / from_toml_de_error_tags_toml_de | From-impl tag mapping | `:81`,`:89`,`:103` |
| `src/errors.rs` | serialize_round_trip | JSON keys `"message"`/`"source"` literal | `:117` |
| `src/errors.rs` | partial_eq / error_trait_impl | derive contracts | `:125`,`:134` |
| `src/git.rs` | current_branch_in_* / parse_branch_output_* | hermetic git repo, detached HEAD, stderr-as-message | `:73`,`:81`,`:96`,`:103`,`:110` |
| `src/format.rs` | green/red/yellow_string_* + all_helpers_strip_ansi_under_test | color stripped under `cfg!(test)` | `:32-52` |

### CLI integration tests (`tests/`, drive compiled binary via `assert_cmd::Command::cargo_bin("orksorksorks")`)
| File | Test | Covers | Anchor |
|---|---|---|---|
| `tests/step.rs` | step_prints_name_of_step_with_present_artifact | present artifact, stdout==`"two"`, no ANSI | `:41` |
| `tests/step.rs` | step_json_returns_valid_json_with_no_ansi | `-j` → `{"data": "two"}` | `:64` |
| `tests/step.rs` | step_no_artifacts_present_fails | valid config, no artifacts → exit 1 | `:86` |
| `tests/step.rs` | step_with_default_returns_default_when_no_artifacts | empty trigger default | `:99` |
| `tests/step.rs` | step_real_artifact_beats_default_step | default never beats real match | `:133` |
| `tests/step.rs` | step_without_flag_reads_config_dir | XDG_CONFIG_HOME env resolution | `:169` |
| `tests/step.rs` | step_without_flag_ignores_cwd_config | **regression guard** — CWD config must be ignored | `:206` |
| `tests/step.rs` | step_with_missing_config_shows_path_in_stderr | text stderr: verbatim path + `"specified via --config"`, no ANSI | `:224` |
| `tests/step.rs` | step_with_missing_xdg_config_shows_path_in_stderr | text stderr: resolved path + `"resolved from XDG_CONFIG_HOME"` | `:256` |
| `tests/step.rs` | step_with_missing_config_json_contains_path_in_message | JSON: `error.source=="io"`, phrases in `error.message` | `:289` |
| `tests/model.rs` | model_prints_model_for_current_step / model_uses_manifest_trigger_step / model_json_returns_valid_json_with_no_ansi | `[[models]]` resolution for current step, no ANSI | `:51`,`:75`,`:99` |
| `tests/model.rs` | model_no_artifacts_present_fails / model_unknown_model_reference_fails | exit 1 on no artifacts / dangling `step.model` | `:121`,`:134` |
| `tests/model.rs` | model_without_flag_reads_config_dir | XDG resolution | `:163` |
| `tests/model.rs` | thinking_prints_/thinking_uses_manifest_trigger_step/thinking_json_*/thinking_no_artifacts_*/thinking_unknown_model_reference_fails/thinking_without_flag_reads_config_dir | `[[models]].thinking` resolution | `:204`,`:225`,`:245`,`:267`,`:280`,`:306` |
| `tests/prompt.rs` | prompt_without_step_name_infers_current_step | derive step → `[[prompts]]` content, no ANSI | `:53` |
| `tests/prompt.rs` | prompt_without_step_name_no_artifacts_fails | no artifacts → exit 1 | `:74` |
| `tests/prompt.rs` | prompt_with_explicit_step_name_overrides_step | `prompt <name>` wins over derived step | `:87` |
| `tests/prompt.rs` | prompt_json_returns_valid_json_with_data_field | `-j` → `{"data": "<content>"}` | `:109` |
| `tests/prompt.rs` | prompt_unknown_step_name_fails | `prompt <unknown>` → exit 1 | `:132` |
| `tests/prompt.rs` | prompt_without_flag_reads_config_dir | XDG resolution | `:158` |
| `tests/init_creates_file.rs` | init_creates_orksorksorks_toml_with_default_content | file content exactly `version = "0.1.0"\n` | `:11` |
| `tests/init_creates_file.rs` | init_returns_success_message | stdout `✓ Created <path>` | `:30` |
| `tests/init_creates_file.rs` | init_in_readonly_dir_returns_error_exit_code | readonly flip (`fs::metadata` permissions, `:49-67`) | `:49` |
| `tests/init_creates_file.rs` | init_json_error_in_readonly_dir_shows_source_io | JSON stdout literal `"source":"io"` | `:69` |
| `tests/init_creates_file.rs` | init_with_config_flag_writes_to_given_path | `--config` wins over hostile env | `:93` |
| `tests/init_creates_file.rs` | init_without_home_fails_with_config_dir_tag | JSON literal `"source":"config-dir"` | `:112` |
| `tests/json_output.rs` | init_json_returns_valid_json_with_data_field / init_no_json_prints_plain_text | envelope vs plain text, no ANSI | `:4`,`:25` |
| `tests/branch.rs` | branch_prints_current_branch / branch_json_returns_valid_json_with_no_ansi / branch_outside_repo_fails | git branch output | `:36`,`:49`,`:64` |
| `tests/artifact_directory.rs` | artifact_directory_prints_composed_path / _json_… / _outside_repo_fails | path composition (macOS `/var`→`/private/var` canonicalization, `:34-39`) | `:46`,`:60`,`:76` |

## Platform/tooling gating

- **No `#if os(...)` gating** in tests; the only cfg gates are `cfg!(test)` (ANSI strip, `src/format.rs:8-12`) and `cfg!(windows)` (env collection, `src/config_dir.rs:29-38`). Windows-specific behavior (APPDATA) is unit-tested on any host via direct `ConfigEnv` construction (`src/config_dir.rs:175-189`).
- `#![allow(clippy::permissions_set_readonly_false)]` at `tests/init_creates_file.rs:5` (only integration test that touches file permissions).
- `#![warn(missing_docs)]` at `src/main.rs:6` — every `pub` item needs a doc comment (clippy/check will not fail, but missing docs are deliberate warnings; comment on all added items regardless).

## Build/verify gotchas

- **One test process at a time**: integration tests create disposable git repos + configs in fresh `tempfile::tempdir()` dirs and mutate only the injected env (`XDG_CONFIG_HOME` via `.env(...)`, `HOME`/`XDG_CONFIG_HOME` via `.env_remove(...)`, `init_creates_file.rs:15,34,58,78,114-115`) — the binary must never touch the ambient checkout or real config dir (CI leaves detached HEAD: `tests/branch.rs:3-7`, `tests/artifact_directory.rs:3-7`).
- **`--no-tests pass` is required** in test.sh (`:14`) and CI because the crate is binary-only; dropping it fails the gate.
- **`cargo test` is not used** — nextest only (`cargo nextest run`); `assert_cmd::Command::cargo_bin` compiles the binary on first use.
- **Destination-pinning gotchas**: the binary echoes `--config` paths verbatim (no canonicalization, `tests/step.rs:224-230`) but resolves real cwd for artifact dir (canonicalize expected values: `tests/artifact_directory.rs:40-43, 34-39`); `artifact_dir_path` requires a trailing slash and `/`→`-` branch normalization (`src/commands/mod.rs:155-162`).
- **`tempfile::tempdir()` cleanup breaks if flags change permissions**: `init_creates_file.rs:49-67` restores permissions before the tempdir is cleaned.
- **Coverage**: `src/main.rs` is codecov-ignored (`codecov.yml:2`); patch coverage must stay ≥50% — new validation logic tested by unit tests in `src/*.rs` counts toward src patch coverage; integration `tests/` do not.
- **rg gate** rejects `TODO:`/`FIXME`/`dbg!` in any `*.rs` (`scripts/test.sh:16-19`, CI `_reusable-lint.yml`) — do not leave such markers.
- **`.ignore:1-2` hides `.pi/orksworksorks` from grep/find tooling** (files remain tracked) — use explicit paths when searching there.