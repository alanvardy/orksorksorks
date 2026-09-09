# Design Discussion

## Current State

`init` generates the config file in `init_command` (src/commands/mod.rs:159-175):
`Config::default()` → `toml::to_string` → `File::create` → `write_all` → `flush`
→ `sync_all`, then prints `✓ Created <path>`. The emitted bytes are exactly
`version = "0.1.0"\nshow_frontmatter = true\n` (42 bytes) because the four vec
fields carry `skip_serializing_if = "Vec::is_empty"` (config.rs:67,70,73,77).

The resolved `toml` crate (1.1.5) **cannot emit comments** — zero comment
support in its serializer (research §Q3; crate README explicitly cedes
format control to `toml_edit`, which is absent from Cargo.lock). So today any
commented file is hand-authored and sits outside the tool's writer.

Two integration tests pin the 42 bytes **byte-exact by hand**
(tests/init_creates_file.rs:22-26 and 104-108) because the crate is binary-only
(no lib target; tests cannot import `Config`). `File::create` truncates, so
running `init` against a populated file silently destroys it (no clobber guard,
no test coverage — research §Q5/Open Areas).

The user's live config is a symlink
(`~/.config/orksorksorks.toml` → `/Users/vardy/dev/dotfiles/orksorksorks/orksorksorks.toml`),
1269 lines: `version` + `show_frontmatter`, then 9 `[[steps]]`, 2 `[[models]]`,
2 `[[scripts]]`, 9 `[[prompts]]` in section order steps → models → scripts →
prompts (research §Q4). It already uses the only TOML-comment precedent in the
project — three full-line `#` dash dividers at lines 51/63/97, two of which have
a stray leading space (63, 97).

## Desired End State

`init` writes a **hand-authored commented template** — a single source of truth
at `templates/default.toml` — that documents the two live keys and shows
commented-out examples of all four sections. `init` **refuses to overwrite an
existing file** (new `config-exists` error tag, exit 1). Both byte-exact tests
reference the template via `include_str!`, eliminating source/test drift.

The user's live config receives **in-place descriptive comments** preserving all
1269 lines of content.

**Verify by:**
- `./scripts/test.sh` green (fmt / check / clippy `-D warnings` / nextest /
  forbidden-strings). The template's `#` comments are in a `.toml` file, not
  `.rs`, so the `rg -g '*.rs'` gate is untouched.
- New unit test: the template deserializes to `Config::default()` (guards
  against an accidental uncommented line or an unparseable template).
- New integration test: `init` on an existing file exits 1 with
  `"source":"config-exists"` and leaves file content unchanged.
- Live file still parses after editing: `python3 -c "import tomllib,sys;
  tomllib.load(open(sys.argv[1],'rb'))" /Users/vardy/dev/dotfiles/orksorksorks/orksorksorks.toml`.

## Patterns to Follow

- **Central error type** — every error flows through `Error{message, source}`
  (errors.rs:17-23); construct new tags with `Error::new(source, message)`
  (errors.rs:19-25). `Display` owns all coloring (errors.rs:33-43). The tag
  registry is the doc comment at errors.rs:5-16 — `config-exists` must be added
  there.
- **Parse-before-acting contract** — every command loads via `read_config`
  (config.rs:107-124) + `validate()` (config.rs:139-226); `deny_unknown_fields`
  on all five types (config.rs:8,23,35,46,59). The template must stay
  parseable **and**, when uncommented content is counted, still equal
  `Config::default()`.
- **Test style** — `Command::cargo_bin("orksorksorks")`, destinations driven by
  `XDG_CONFIG_HOME` / `--config` (init_creates_file.rs:12-15, 95-100); byte-exact
  content assertions (22-26, 104-108); JSON `v["error"]["source"]` tag checks
  (init_creates_file.rs:81-84, config_validation.rs:8-77). No `cfg!(windows)`
  gates; macOS `/var`→`/private/var` via `canonicalize` (artifact_directory.rs:39-43).
- **Comment style** — full-line `#` comments only, **no trailing**
  `key = value # note` comments, blank-line-separated sections, `#`-dash divider
  banners. This is the sole existing style precedent (live file 51/63/97).
- **Section order** — document sections as steps → models → scripts → prompts,
  matching the user's live file (research §Q4), not the serializer's struct-field
  order (config.rs:68-78). All `[[x]]` entries must stay contiguous (TOML forbids
  table redefinition — research §Q2).
- **Do NOT follow**: `toml::to_string` for the default file (cannot comment).
  Keep it at the other call sites (config.rs:262,271,299) — those are
  value-equality round-trips and remain correct.

## Design Decisions

1. **Template scope = commented schema example.** Ship `version` +
   `show_frontmatter` live, plus `#`-commented example blocks for `[[steps]]`,
   `[[models]]`, `[[scripts]]`, `[[prompts]]` showing real field names
   (Step: `name/trigger_artifact/model/script`; Model: `name/model/thinking`;
   Prompt: `name/content`; Script: `name/content` — config.rs:7-56). Actual
   content is unchanged, so `validate()` stays green.

2. **Single source of truth via `include_str!`.** New asset `templates/default.toml`
   (repo's first non-Rust asset). `src/commands/mod.rs` embeds it with
   `include_str!("../templates/default.toml")`; the two tests change from the
   42-byte literal to `assert_eq!(content, include_str!("../templates/default.toml"))`.
   Byte-exact is preserved and drift is impossible.

3. **Live config = one-off in-place edit.** Apply full-line comments directly to
   the dotfiles file, preserving every existing line; normalize the stray leading
   space on the 63/97 dividers. Show the diff for review; the file is git-managed
   in the dotfiles repo, so it is reviewable and revertible. No repo code involved.

4. **`init` becomes non-clobbering.** Replace `File::create` (mod.rs:172) with
   `OpenOptions::new().write(true).create_new(true).open(path)`; map
   `ErrorKind::AlreadyExists` to `Error::new("config-exists", &format!("{} already
   exists; not overwriting", path.display()))`, other I/O through the existing
   `?` (`"io"`). `create_new` is atomic (no check-then-create race). Existing
   tests are unaffected — all target fresh destinations (research §Q5).

5. **Comment style = full-line dividers, normalized.** No trailing comments;
   header explains the file and the four sections; each section's example block
   is preceded by a `#`-dash banner. Matches and normalizes the live-file style.

## What We're NOT Doing

- **Not changing the default data**: the generated file still contains only
  `version` + `show_frontmatter` as live keys; examples stay commented.
- **Not adding a dependency** (`toml_edit` or a comment crate) — the template is
  a plain string literal.
- **Not making `init` skip silently or idempotent** — it errors on an existing
  file; no `--force` flag this ticket.
- **Not writing any migration/rewrite code** to annotate existing files on disk
  programmatically; the live-file edit is a manual one-off in the dotfiles repo.
- **Not touching** `toml::to_string`'s other call sites, `Config` defaults,
  validation, path resolution, or the `Display`/JSON error shape beyond adding
  one tag.

## Open Risks

- `config-exists` is a new machine-readable `source` tag; any downstream tooling
  keying on tags must learn it. Documented in errors.rs:5-16.
- Symlink semantics: the live config is a symlink; `create_new` follows the
  link, so `init` against `~/.config/orksorksorks.toml` still refuses (AlreadyExists)
  — desired, but worth an explicit note in the test.
- The commented example blocks are hand-maintained and **not compiled** against
  the structs; a future field rename would not fail any test. Mitigation: keep
  the example minimal and add a template comment pointing at `src/config.rs`.
- New asset file packaging: `include_str!` bakes it at compile time so
  `cargo install --path . --locked` is safe; ensure the file is tracked/reviewed
  in the PR.
- Live-file edit lives in the dotfiles repo, outside this checkout — committing
  it is a separate step the user drives.