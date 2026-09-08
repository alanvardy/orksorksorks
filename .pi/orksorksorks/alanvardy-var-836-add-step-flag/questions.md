# Research Questions

## Context

The orksorksorks CLI is a small Rust binary whose command types, dispatch, handlers, and step-determination logic all live in `src/commands/mod.rs`, with config in `src/config.rs`, config-dir resolution in `src/config_dir.rs`, and entry in `src/main.rs`. Step-derivation code and its callers exist on the current branch and on the unmerged `add-model-command` branch, which adds model/thinking/prompt commands. Integration tests under `tests/` exercise each command.

## Questions

1. How does command dispatch flow from CLI parse through to handler output in `src/commands/mod.rs`? Trace the full path for each command that auto-derives its step — `step` on the current branch, and `model`, `thinking`, and `prompt` on the `add-model-command` branch — and note what each handler does with the derived step and how config and env reach the handlers.

2. How are CLI arguments declared and parsed with clap in this codebase? Compare the global `-j/--json` flag with per-subcommand options such as `--config` and `prompt`'s positional `STEP_NAME` argument, and trace how a declared argument flows from a clap derive struct through dispatch into a command handler.

3. What is the config schema (`src/config.rs`) on each branch, and how are step names consumed after determination? How do the model/prompt lookups (`resolve_model`, `resolve_prompt`) validate names and report unknown-name errors, and what happens when `determine_step` finds no matching artifact or no default step?

4. How do the current branch, `main`, and the unmerged `add-model-command` branch diverge in the step-derivation code? Compare `determine_step`'s signature, the config-dir plumbing (`config_file_path` vs `config_file_path_with_env`, the `ConfigEnv`/`ConfigPathSource` types), and which commands exist on each branch.

5. How is the artifact-directory path composed and used? Trace `artifact_dir_path` from cwd and `git::current_branch`, and describe how `determine_step` reads the filesystem — the trigger-artifact existence checks, the empty-trigger default-step behavior, and the failure modes when nothing matches.

6. How do the integration tests exercise the step-related commands? Describe the shared setup pattern (temp git repo, fixture config, artifact dir plus trigger files), the CLI invocation style, and the assertion conventions for text, JSON, and error output across `tests/step.rs`, `tests/model.rs`, and `tests/prompt.rs`.
