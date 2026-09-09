# Task

Add rust_arkitect tests to verify layering/architecture constraints across the 9 modules in the ~5,000 line Rust project. The tests should assert that:
- `commands` imports from `config`, `config_dir`, `errors`, `format`, `git` (downward dependencies only)
- No cycles exist between modules
- No upward imports (modules can't import from modules above them in the dependency hierarchy)

Wire the tests into the test workflow.

## Why SMALL

Single module (test code), localized changes to test files only, clear approach (rust_arkitect), no schema/API/UI/platform changes, no design sign-off needed, tests are few and local to the change.

## Key files (if the recon found any)

- `src/commands/mod.rs`
- `config.rs`, `config_dir.rs`, `errors.rs`, `format.rs`, `git.rs`, `main.rs`
- Test module file(s) to add rust_arkitect tests
- Test workflow configuration
