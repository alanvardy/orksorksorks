# Task

Add descriptive comments to the `orksorksorks.toml` config file so that AI or humans can understand and edit it. The repo generates this file at runtime via `init_command` (`src/commands/mod.rs` ~160–176): replace `toml::to_string(&Config::default())` — the `toml` crate serializer cannot emit comments — with a hand-authored commented template string, and update the two byte-exact assertions in `tests/init_creates_file.rs` (~22–25 and ~108).

The user additionally requires the comments to land in their **live** hand-authored config at `~/.config/orksorksorks.toml` (currently 1269 lines: `version`/`show_frontmatter`, 10+ `[[steps]]`, `[[models]]`, `[[scripts]]`, and 9 `[[prompts]]` including the full QRSPI prompt set). All existing custom content in that file must be preserved — the change adds comments, it does not replace content.

## Why LARGE

CONVENTION_RISK — touches the tool's live persistence config (user-owned data outside version control; regenerating the existing populated file would irreversibly destroy its content, and the commented template must stay parseable by `validate()`, `deny_unknown_fields`) + DESIGN_SIGN-OFF — several viable approaches for applying comments to an existing populated file (in-place one-off edit preserving content vs. template-only with user merge vs. making `init` non-clobbering), and a human decision on how the shipped template should relate to the user's real file.