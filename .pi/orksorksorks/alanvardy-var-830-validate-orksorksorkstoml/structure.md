# Structure Outline

## Approach

Add post-parse validation to `read_config` (`src/config.rs`), the single choke point every command (`step`/`model`/`thinking`/`prompt`) already goes through. After `toml::from_str` succeeds, a new `Config::validate()` runs in fixed order and rejects invalid configs with distinct `config:*` error tags before any git/artifact work. Serde-level `deny_unknown_fields` hardens the parse. All logic is pure decision code in `src/config.rs` (unit-testable, counts toward the ≥50% patch-coverage gate); CLI behavior is pinned via integration tests.

Horizontal, bottom-up: schema hardening → validation logic → chokepoint wiring (with fixture reconciliation) → CLI pinning. No cross-cutting change delays testability; the only wrinkle is existing partial test fixtures, reconciled in Stage 3.

---

## Stage 1: Schema hardening (serde parse layer)

Delivers parse-time rejection of unknown keys across the whole schema, ahead of all post-parse checks. Green tests prove every struct's key surface is locked.

**Files**: `src/config.rs`
**Key changes**:
- `#[serde(deny_unknown_fields)]` on `Step`, `Model`, `Prompt`, `Config` — all four structs (`src/config.rs:7-53`). No signature change; pure attribute addition.
- No `src/errors.rs` change — rejection flows through the existing `From<toml::de::Error>` (`src/errors.rs:57-64`) and emits the existing `"toml::de"` tag.

**Tests**: new unit tests in `src/config.rs` — unknown key in a `[[steps]]` table → `source == "toml::de"` (sad path); existing round-trips (`steps_round_trip`, `models_round_trip`, `prompts_round_trip`, `config_round_trip_serialize_deserialize`) stay green (happy path — no fixture carries unknown keys). Doc-comment all added items (`#![warn(missing_docs)]`).
**Verify**: `scripts/test.sh` green; faster loop `cargo nextest run --no-tests pass -E 'test(config)'`.

---

## Stage 2: Validation logic (`Config::validate()`)

Delivers the pure validation decision function — the heart of the work. Green unit tests prove each rule fires with the right tag, first-error-wins ordering is deterministic, and valid configs pass.

**Files**: `src/config.rs`
**Key changes**:
- `impl Config { fn validate(&self) -> Result<(), crate::errors::Error> }` — new, `pub(crate)`-visible to `read_config`, doc-commented. Returns `Ok(())` or the first `Error::new("<tag>", &format!(...))`.
- Private helpers as needed for duplicate detection (e.g. `HashSet<&str>` over names / triggers).

**Fixed check order** (each returns the first failure, `?`-chained):

| # | Tag | Rule |
|---|---|---|
| 1 | `config:version` | `version != "0.1.0"` |
| 2 | `config:duplicate-name` | same `name` twice within `steps` / `prompts` / `models` (each list independently) |
| 3 | `config:empty-name` | any `name` in all three lists is empty (`""`) |
| 4 | `config:empty-model` | any `step.model` is empty |
| 5 | `config:duplicate-trigger` | two steps share a non-empty `trigger_artifact` |
| 6 | `config:multiple-default` | more than one empty `trigger_artifact` |
| 7 | `config:missing-prompt` | a `step.name` has no matching `prompts[].name` |
| 8 | `config:missing-model` | a `step.model` has no matching `models[].name` |

`deny_unknown_fields` (Stage 1) fires before all of these at parse time; extra (unreferenced) prompts/models are allowed (membership-only, no length/order).

**Tests**: in-module `#[cfg(test)]` unit tests in `src/config.rs` — valid config passes; one invalid case per rule (8 sad paths, each `assert_eq!(err.source, "<tag>")`); plus an ordering-determinism test (a config violating two rules fails with the earlier tag every run). Build cases via `toml::from_str` inside the test module, no fixture files.
**Verify**: `scripts/test.sh` green; `cargo nextest run --no-tests pass -E 'test(config)'`. Validation is not yet wired, so the existing CLI suite remains untouched and green.

---

## Stage 3: Chokepoint wiring + fixture reconciliation

Delivers the fail-fast behavior: every command now validates config before git/artifact work. Green tests prove `read_config` returns `config:*` errors where it previously returned a parsed `Config`.

**Files**: `src/config.rs`, `tests/step.rs`, `tests/model.rs`, `tests/prompt.rs`
**Key changes**:
- `read_config` (`src/config.rs:71-88`) — after `toml::from_str` (`:86`) and before returning `:87`, insert `config.validate()?;`. Signature unchanged (still `Result<Config, Error>`).
- **Fixture reconciliation (real scope, design Open Risk)**: the three integration files write section-partial configs that Stage 2's coverage checks will now reject. Make each fully consistent — `tests/step.rs` gains `[[models]]`+`[[prompts]]` matching its `steps`; `tests/model.rs` gains `[[prompts]]`; `tests/prompt.rs` gains a `[[models]]` covering its dangling `step.model` refs. ~17 success-path assertions stay passing with only the inline `concat!` string literals edited. Invalid-override tests (`model_unknown_model_reference_fails`, `prompt_unknown_step_name_fails`) keep exercising a *valid* config + bad runtime lookup, not a bad config.

**Tests**: new `read_config_*` unit tests in `src/config.rs` — a representative invalid TOML string on disk → `read_config` returns `Err` with a `config:*` tag (e.g. `config:missing-model`, `config:duplicate-name`); existing `read_config_loads_steps_from_disk` / `_prompts_from_disk` / `_malformed_toml_tags_toml_de` / `_missing_file_tags_io` stay green.
**Verify**: full `scripts/test.sh` green — proves the reconciled fixtures + new wiring coexist. Manual smoke: `cargo run -- step --config <bad.toml>` exits 1 with the tag on stderr (no ANSI).

---

## Stage 4: CLI integration pinning (`config:*` surface)

Delivers end-to-end proof that invalid configs surface through the real binary with the project's exact error contract: tagged source, exit 1, correct JSON envelope, no ANSI.

**Files**: `tests/config_validation.rs` (new) — follows `assert_cmd::Command::cargo_bin("orksorksorks")` + `tempfile::tempdir()` + inline `std::fs::write` pattern.
**Key changes**: no production code.
**Tests**: one integration test per rule family (happy + sad where relevant), each tempdir gets a git repo and an inline invalid config: `config:version`, `config:duplicate-name`, `config:duplicate-trigger`, `config:multiple-default`, `config:empty-name`, `config:missing-prompt`, `config:missing-model`; and a fully-valid config asserting success. Every failure test pins: `.failure()` (exit 1), `!output.contains('\x1b')`, text-stderr phrase, JSON `error.source == "config:<tag>"` via `-j` (`{"error":{"message":…,"source":…}}` on stdout).
**Verify**: `scripts/test.sh` green. Live check per `live-testing` conventions is a CLI run, not HTTP: for a couple of tags, `cargo run -- <cmd> --config bad.toml` and inspect exit code + stderr.

---

## Testing Checkpoints

- After Stage 1: `cargo nextest run --no-tests pass -E 'test(config)'` — serde round-trips + new unknown-field tests green.
- After Stage 2: same filter green — `validate()` unit tests (8 tags + ordering) green; CLI suite still untouched.
- After Stage 3: full `scripts/test.sh` green — wiring + reconciled fixtures green.
- After Stage 4: full `scripts/test.sh` green — new `tests/config_validation.rs` pins the `config:*` contract.

## Notes

- **Not a vertical slice** — each layer ships code + tests together; Stages 1 and 2 are independently landable even if later stages stall.
- **Cross-cutting note**: once Stage 3 wires validation, `resolve_model`'s `"model"` tag (`src/commands/mod.rs:216`) and `resolve_prompt`'s `"prompt"` tag (`:275`) become CLI-unreachable for dangling refs (load-time validation pre-empts them). Functions and their unit tests remain; no stub needed, but Stage 4 should not add CLI tests expecting those tags for dangling refs.
- All work on the main ticket — no child tickets.
- If `scripts/test.sh` fails at any checkpoint, halt and fix that layer before advancing.