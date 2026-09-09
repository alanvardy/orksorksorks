# Done

- **Branch / head SHA**: `alanvardy-var-838-add-descriptive-comments-to-toml-file` / `b6542d6`
- **Mechanical checks**: `./scripts/test.sh` — fmt, check, clippy `-D warnings` (plus `--all-targets --all-features --locked`), nextest **195/195**, forbidden-strings gate all pass.
- **Review outcome**: 5 parallel reviewers found zero blockers. Review-identified fixes applied:
  - Template trailing newline added (matches plan spec; all byte-exact tests still green via `include_str!`)
  - Steps comment now documents the default-step (`trigger_artifact = ""`) fallback and the at-most-one-default rule
  - `errors.rs` tag registry now includes the pre-existing `"script"` tag
  - Symlink note corrected (`O_EXCL` refuses symlinks without following, not "follows the link")
  - Clap `init` help text now mentions non-clobber behavior
  - `src/config.rs` reference in template header scoped for end-user audience
  - New `template_commented_examples_validate` test guards commented examples against schema drift
  - New `init_init_config_flag_refuses_existing_file` test covers `--config` + existing-file clobber path
- **Remaining manual items**:
  - Phase 4 (live config edit in `/Users/vardy/dev/dotfiles/orksorksorks/orksorksorks.toml`) — user-driven, out of this repo
  - The template's header still points at a source-file path; if this binary is distributed outside the repo, inline the validation rules instead
  - No `--force` flag (explicitly out of scope per plan)