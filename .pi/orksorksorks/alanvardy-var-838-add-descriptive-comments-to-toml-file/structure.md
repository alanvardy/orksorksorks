# Structure Outline

## Approach
Replace the serializer-generated 42-byte default (`toml::to_string(Config::default())`) with a hand-authored commented template embedded via `include_str!` (`templates/default.toml`), make `init` refuse to overwrite an existing file (new `config-exists` tag), and apply matching full-line `#` comments to the user's live dotfiles config as a one-off manual edit. Commented example blocks stay comments, so the file still deserializes to `Config::default()` and `validate()` + `deny_unknown_fields` stay green.

## Stage 1: Template asset — single source of truth (data layer)
Delivers the commented template file and proves it is valid TOML that still equals the default config. Bottom-most layer: pure data, no behavior change.

**Files**: `templates/default.toml` (new), `src/config.rs` (unit test only)

**Key changes**:
- `templates/default.toml` — live keys `version = "0.1.0"` and `show_frontmatter = true`; `#`-commented example blocks for `[[steps]]` (`name`/`trigger_artifact`/`model`/`script`), `[[models]]` (`name`/`model`/`thinking`), `[[scripts]]` (`name`/`content`), `[[prompts]]` (`name`/`content`) — in section order steps → models → scripts → prompts (not serializer field order). Full-line `#` comments only, no trailing `key = value # note`, blank-line separation, `#`-dash banners, header comment pointing at `src/config.rs`.
- `#[cfg(test)]` module in `src/config.rs`: new `template_deserializes_to_default` — `toml::from_str::<Config>(include_str!("../templates/default.toml")).unwrap() == Config::default()`. (`Config` derives `PartialEq` + `Debug`, config.rs:7,58.)

**Tests**: one guard, both sad paths covered by it — an accidentally *uncommented* example line or added key changes the parsed value (`PartialEq` mismatch); an *unparseable* template fails `from_str`. Happy path = parses and equals default.
**Verify**: `cargo nextest run config` (targeted), then `./scripts/test.sh` green before advancing.

## Stage 2: Write-safety guard — `init` becomes non-clobbering (error/command layer)
Makes `init` refuse to truncate an existing populated file, surfacing a machine-readable tag. Independent of content, so it can land (and be proven) on its own.

**Files**: `src/errors.rs` (registry doc comment), `src/commands/mod.rs` (`init_command`)

**Key changes**:
- `src/errors.rs`: add `"config-exists"` to the tag registry doc comment (errors.rs:5-16). No new type — reuse `Error::new(source: &str, message: &str)` (errors.rs:19).
- `init_command(path: &std::path::Path) -> Result<String, Error>`: replace `std::fs::File::create(path)?` with `std::fs::OpenOptions::new().write(true).create_new(true).open(path)`, mapping `std::io::ErrorKind::AlreadyExists` → `Error::new("config-exists", &format!("{} already exists; not overwriting", path.display()))`. All other I/O errors fall through the existing `From<io::Error>` → `"io"`.

**Tests** (`tests/init_creates_file.rs`): new `init_refuses_to_overwrite_existing_file` — pre-write a file at the destination, run `init`, assert exit 1, JSON `"source":"config-exists"`, and the pre-existing content byte-unchanged (sad path). Existing 6 tests stay green (all target fresh destinations). Comment notes the symlink case: `create_new` follows the link, so `init` against `~/.config/orksorksorks.toml` still refuses — desired.
**Verify**: `cargo nextest run init_creates_file`, then `./scripts/test.sh` green.

## Stage 3: Template emission — `init` writes the commented template (content layer)
Switches the emitted bytes from the serializer to the embedded template, on top of the now-safe write path.

**Files**: `src/commands/mod.rs` (`init_command`), `tests/init_creates_file.rs`

**Key changes**:
- `init_command`: drop `let config = Config::default(); let toml_str = toml::to_string(&config)?;` and write `include_str!("../../templates/default.toml")` bytes instead. (Path is relative to `src/commands/mod.rs` — the design doc's `../templates/` is off by one level; correct depth is `../../`.)
- `tests/init_creates_file.rs`: replace both byte-exact literals (22-26, 104-108) with `assert_eq!(content, include_str!("../templates/default.toml"))` — source/test drift becomes impossible.

**Tests**: two updated byte-exact assertions (happy path: fresh `init` writes exactly the template). Sad paths already covered by Stage 1 (template validity) and Stage 2 (clobber refusal).
**Verify**: `cargo nextest run init_creates_file`, then `./scripts/test.sh` green (fmt / check / clippy `-D warnings` / nextest / forbidden-strings — the template is a `.toml` file, so the `rg -g '*.rs'` gate is untouched).

## Stage 4: Live config — one-off in-place comment edit (manual, out-of-repo)
Applies the same full-line `#` comments to the user's dotfiles file, preserving all 1269 lines of custom content. Not a code layer; the final user-facing deliverable.

**Files**: `/Users/vardy/dev/dotfiles/orksorksorks/orksorksorks.toml` (git-managed in the dotfiles repo — **not** this checkout)

**Key changes**: insert a header comment + per-section `#`-dash banners (steps → models → scripts → prompts), normalize the stray leading space on the 63/97 dividers; no line removed, no trailing comments, no content replaced. All existing steps/models/scripts/prompts untouched.
**Tests**: no repo code or tests involved.
**Verify**: manual — `python3 -c "import tomllib,sys; tomllib.load(open(sys.argv[1],'rb'))" /Users/vardy/dev/dotfiles/orksorksorks/orksorksorks.toml` parses, and `git diff` reviewed in the dotfiles repo (user drives the commit there).

## Testing Checkpoints
- After Stage 1: `cargo nextest run config` green — template parses and equals `Config::default()`.
- After Stage 2: `cargo nextest run init_creates_file` green — clobber test exits 1 / `config-exists`; old byte tests still pass.
- After Stage 3: `cargo nextest run init_creates_file` green — both byte assertions match `include_str!`; `./scripts/test.sh` fully green.
- After Stage 4: `tomllib.load` parses the live file; dotfiles `git diff` reviewed.

## Notes / cross-cutting
- The commented example blocks are hand-maintained and not compiled against the structs — a future field rename would fail no test. Accepted mitigation: keep examples minimal and put a template header comment pointing at `src/config.rs`. The Stage 1 guard pins only the *live* (uncommented) keys.
- `toml::to_string` stays at the other call sites (config.rs:262,271,299) — those are value-equality round-trips and remain correct.
- `init` is not made idempotent or given `--force` this ticket; it errors on an existing file.
