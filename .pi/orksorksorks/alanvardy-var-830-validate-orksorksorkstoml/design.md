# Design Discussion

## Current State

Post-rebase (branch now atop `main` @ `86e6483`), the config schema has three name-keyed collections:

- `Step { name, trigger_artifact, model }` — `src/config.rs:7-14`. `model` is a required `String` that names a `models[].name`.
- `Model { name, model, thinking }` — `src/config.rs:18-25`.
- `Prompt { name, content }` — `src/config.rs:29-34`.
- `Config { version, steps, models, prompts }` — `src/config.rs:41-53`; each vec optional via `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.

Load path: `read_config` (`src/config.rs:71-88`) runs `toml::from_str::<Config>` (`:86`) and returns the parsed value directly (`:87`). **There is no post-parse validation** — no `validate()`, no duplicate or cross-section checks anywhere.

Names are consumed as flat linear lookups, none of which reject bad references early:

- `determine_step` (`src/commands/mod.rs:183-203`) reverse-iterates `steps`; an empty `trigger_artifact` marks the default step (`:186-192`); returns the winning `Step`.
- `resolve_model` (`src/commands/mod.rs:210-217`) scans `models` for `name == step.model`; else `Error::new("model", …)` (`:216`).
- `resolve_prompt` (`src/commands/mod.rs:269-276`) scans `prompts` for `name == step.name`; else `Error::new("prompt", …)` (`:275`).
- Four commands all call `read_config` first — `step` (`:226`), `model` (`:240`), `thinking` (`:255`), `prompt` (`:281`) — so a single choke point already exists, currently unused for validation.

Error convention: one `Error { message, source }` (`src/errors.rs:9-13`); `source` is a free-form lowercase tag (`:6`), constructed with `Error::new(tag, &format!(...))` (`:18`). Existing tags: `"io"`, `"toml::ser"`, `"toml::de"`, `"step"`, `"git"`, `"config-dir"`, `"model"` (`src/commands/mod.rs:216`), `"prompt"` (`:275`). Errors surface through the single output layer — text stderr with two leading newlines (`src/main.rs:33`), JSON `{"error":{"message","source"}}` on stdout (`:46`), exit code 1 (`:71-73`).

## Desired End State

After `toml::from_str` succeeds, `read_config` validates the parsed config and rejects any invalid config before git/artifact work happens. Every command (`step`/`model`/`thinking`/`prompt`) fails fast on an invalid config. Validation, in fixed order:

1. **`version`** must equal `"0.1.0"` (the string `init` writes and `Config::default` holds).
2. **`deny_unknown_fields`** on all four structs — unknown keys become parse-time `"toml::de"` (serde-level, ahead of all post-parse checks).
3. **Duplicate names** rejected within `steps`, `prompts`, and `models` (each list independently).
4. **Non-empty names**: every `name` in all three lists and every `step.model` must be non-empty.
5. **Trigger hygiene**: no two `steps` may share a `trigger_artifact`; at most one empty (default) `trigger_artifact`.
6. **Cross-section coverage (membership only — no length, no order)**:
   - every `step.name` must have a matching `prompts[].name`;
   - every `step.model` must have a matching `models[].name`.
   - Extra (unreferenced) prompts/models are allowed.

Verification: in-module unit tests in `src/config.rs` for each rule (valid + invalid); CLI integration tests in `tests/` pinning the new `config:*` tags, exit code 1, and no-ANSI output; the full `scripts/test.sh` gate must be green. New validation logic lives in `src/config.rs` (non-`main.rs`, so it counts toward the ≥50% patch-coverage gate).

## Patterns to Follow

- **Fail-fast in `read_config`** — mirrors how the missing/unreadable config already fails deterministically in `read_config` before git resolution (`src/commands/mod.rs:225-233` doc + `:233`).
- **Error shape** — `Error::new("<tag>", &format!(...))` with a parameterized message (`src/errors.rs:18-23`); returning `Err(Error)` automatically gets Display/JSON/exit-1 for free (`src/main.rs:33,46,71-73`).
- **Grouped tag style** — `"toml::de"` already uses `::`; the new `config:*` namespace follows that precedent rather than the flat `"step"`/`"git"` style.
- **Test layering** — pure decision logic gets in-module `#[cfg(test)]` tests; anything crossing the process boundary gets `assert_cmd::Command::cargo_bin` integration tests in a temp git repo (`tests/model.rs:8-15`, `tests/prompt.rs:7-13`).
- **Inline TOML fixtures** — no fixture files; inline `std::fs::write` / `concat!` string literals (`tests/model.rs:16-39`, `tests/prompt.rs:22-51`).
- **Pinned error output** — `assert_eq!(err.source, "<tag>")` at unit level plus `contains` phrase checks and literal JSON `"source"` fragments at CLI level (see conventions.md), and no-ANSI guards in every integration test.

Patterns **not** to follow:

- Do **not** reuse the `"model"` / `"prompt"` runtime tags for validation — validation errors get their own `config:*` tags, and those two runtime errors become largely unreachable through the CLI (see Open Risks).
- Do **not** thread validation per-command — a single choke point in `read_config`, not four call sites.

## Design Decisions

1. **Fail-fast in `read_config`** — validate immediately after `toml::from_str` (`src/config.rs:86`), before returning, so all four commands inherit it.
2. **Membership-only cross-reference, no length/order** — each step name resolves to a prompt and each `step.model` to a model; duplicates within a list rejected; extra entries allowed. This keeps the door open for "many steps → one prompt" without re-doing validation.
3. **Full hygiene sweep** — `version` check, `deny_unknown_fields`, duplicate names, duplicate triggers, single default, empty names/model.
4. **Distinct `config:*` tags, first-error-only, fixed order** — one tag per rule, message disambiguates list/value. Tag table:

   | Tag | Check |
   |---|---|
   | `config:version` | `version != "0.1.0"` |
   | `config:duplicate-name` | same name twice in `steps`/`prompts`/`models` |
   | `config:empty-name` | empty `name` in any list |
   | `config:empty-model` | a step's `model` reference is empty |
   | `config:duplicate-trigger` | two steps share `trigger_artifact` |
   | `config:multiple-default` | more than one empty `trigger_artifact` |
   | `config:missing-prompt` | a step name has no matching prompt |
   | `config:missing-model` | a step `model` has no matching model |

5. **`deny_unknown_fields` is serde-level** (parse-time `"toml::de"`), separate from the post-parse checks above.
6. **`version` is exact-match `"0.1.0"`** — reject anything else as unsupported.

## What We're NOT Doing

- **Not** adding a `prompt` field to `Step` — the future "shared prompt" schema change (explicit step→prompt pointer) is out of scope; today's name-keyed `prompts` association is preserved.
- **Not** rejecting orphan prompts/models (unreferenced entries are allowed, enabling shared/unused prompts).
- **Not** migrating or auto-upgrading older `version` values — only reject `!= "0.1.0"`.
- **Not** aggregating multiple failures into one message (first-error-only).
- **Not** changing `determine_step` reverse-priority semantics (`src/commands/mod.rs:183-203`).
- **Not** touching `models`/`prompts` serialization or the `init` output (`version = "0.1.0"\n` stays untouched).

## Open Risks

- **Fixture rewrite is real scope**: `tests/step.rs` (steps-only, no models/prompts), `tests/model.rs` (steps+models, no prompts), and `tests/prompt.rs` (steps+prompts, dangling `model` refs) are all partial configs that fail under coverage checks. ~17 success-path assertions across those files must be made fully-consistent. Not doing so leaves the gate red.
- **`resolve_model`'s `"model"` tag becomes CLI-unreachable** — load-time validation pre-empts the dangling ref; the function and its unit test remain but the error branch is dead through the CLI. `resolve_prompt`'s `"prompt"` tag survives only via the `prompt <name>` override for a non-step name.
- **`version` rejection may need relaxing** when real migrations arrive (accept-and-upgrade older versions rather than reject).
- **`deny_unknown_fields` is backwards-incompatible** for any config carrying typo/extra keys — no such configs exist in-tree today, but real user configs are unknown.
- **Check ordering must be pinned by tests** so first-error determinism holds (e.g. a config with both a duplicate name and a missing prompt must fail with the same tag every run).