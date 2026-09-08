# Implementation Summary

Post-parse validation for `orksorksorks.toml`, wired into `read_config` so every command (`step`/`model`/`thinking`/`prompt`) fails fast on an invalid config before any git/artifact work. Implemented in 4 phases, one commit each, per `plan.md`.

## Commits

| Phase | Commit | Description |
|-------|--------|-------------|
| 1     | 352fba5 | Phase 1: Schema hardening (serde parse layer) |
| 2     | 62f5225 | Phase 2: Validation logic (Config::validate()) |
| 3     | 22de18d | Phase 3: Chokepoint wiring + fixture reconciliation |
| 4     | 6c6efc0 | Phase 4: CLI integration pinning (config:* surface) |

## Automated Checks

- [x] Phase 1: `cargo nextest run --no-tests pass -E 'test(config)'` — `unknown_step_key_is_rejected_as_toml_de` green; existing round-trips green
- [x] Phase 1: `scripts/test.sh` green
- [x] Phase 2: `cargo nextest run --no-tests pass -E 'test(config)'` — 1 valid + 10 sad + 1 ordering `validate` tests green
- [x] Phase 2: `scripts/test.sh` green
- [x] Phase 2: `cargo clippy --tests -- -D warnings` clean (doc comments on `CONFIG_VERSION`/`validate`; automated via the gate)
- [x] Phase 3: `scripts/test.sh` green — reconciled fixtures + new wiring coexist; existing read_config_* unit tests stay green
- [x] Phase 4: `scripts/test.sh` green — new `tests/config_validation.rs` pins the `config:*` contract end-to-end (145/145 nextest)
- [x] Testing checkpoints: after each phase, in order, all red-free (4/4)

## Manual Verification Items (from the plan)

- [ ] `cargo run -- step --config <tmp.toml>` with an extra key in `[[steps]]` → exit 1, stderr contains the `toml::de` phrasing, no ANSI
- [ ] `cargo run -- step --config <bad.toml>` (step with no prompt) exits 1 with `config:missing-prompt` on stderr, no ANSI
- [ ] `cargo run -- step --config <good.toml>` still prints the winning step name
- [ ] For two tags, `cargo run -- step --config bad.toml` → exit 1 + tag on stderr; `cargo run -- step --config bad.toml -j` → `{"error":{"message":…,"source":"config:<tag>"}}` on stdout

## Deviations / Observations

- **Phase 3 adaptation (plan predates rebase):** `tests/prompt.rs::write_config_hidden` (added by the merged `show_frontmatter` feature on main) also references models `small`/`high` with no `[[models]]` entries; it received the same `[[models]]` block as `write_config`, consistent with the plan's stated intent that every partial fixture be fully consistent. No assertion behavior affected.
- **Phase 2 count:** the plan's "1 valid + 8 sad" text undercounts its own enumeration — the full sad-path set is 10 tests (three duplicate-name variants for steps/prompts/models, empty-name, empty-model, duplicate-trigger, multiple-default, missing-prompt, missing-model) + 1 valid + 1 ordering. All enumerated tests exist and are green.
- **Phases 3/4 deviation (sanctioned in plan):** deleted `model_unknown_model_reference_fails` / `thinking_unknown_model_reference_fails` — `resolve_model`'s runtime `"model"` tag is CLI-unreachable once validation guarantees every `step.model` resolves. Coverage restored end-to-end by `config_missing_model_fails` in Phase 4.
- **Phase 3 detail:** `read_config`'s binding needed an explicit `: Config` annotation once `validate()` sits between parse and return (dialect type-inference limitation).
- **nextest filter note:** this nextest version's `-E 'test(config)'` matches test names, not binaries; `-E 'binary(config_validation)'` selects the new file's 9 tests. The plan's checkpoint commands still ran the suite green.