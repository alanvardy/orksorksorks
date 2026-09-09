# Design Discussion

## Current State

The `orksorksorks` CLI reads a versioned `orksorksorks.toml` config modeled in
`src/config.rs`. Struct hierarchy (all `derive(Debug, Clone, Serialize,
Deserialize, PartialEq)`):

- `Step { name, trigger_artifact, model }` — config.rs:7-14. No serde defaults,
  no optional fields. `name` and `trigger_artifact` are `String` (required);
  `model` is a key into `Config::models`.
- `Model { name, model, thinking }` — config.rs:18-25.
- `Prompt { name, content }` — config.rs:29-34. The named-text-collection
  precedent: `name` is both step-name and lookup key, `content` is the blob
  text (TOML multi-line string).
- `Config { version, show_frontmatter, steps, models, prompts }` —
  config.rs:41-56.

Serde attributes live only on `Config` (config.rs:45-55): `show_frontmatter`
defaults true (config.rs:45); `steps`/`models`/`prompts` each carry
`#[serde(default, skip_serializing_if = "Vec::is_empty")]` (config.rs:48-55),
so missing keys deserialize to empty vecs and empty vecs are omitted on write.
That is the existing backward-compat mechanism (config.rs:148-153). `version`
has no serde attrs — required on deserialize, always serialized
(config.rs:42-43). The doc on `version` says it "tracks the config format
version so future migrations can detect and upgrade older files"
(config.rs:38-39), but today **no code reads it** — it is only written via
`Config::default()` (`"0.1.0"`, config.rs:67) and `init_command`
(commands/mod.rs:138-139). No migrations exist.

Each step-field is surfaced by a subcommand sharing one spine (research Q2):
`read_config` → `current_dir` + `git::current_branch()` → `artifact_dir_path` →
`determine_step` → resolve → print. `determine_step` (commands/mod.rs:183-205)
returns the **whole `Step`** so callers read any field, probing
`{artifact_dir}{trigger_artifact}` for presence, with a default-step fallback.
`resolve_prompt` (commands/mod.rs:269-276) does a linear first-match scan of
`config.prompts` and returns `Error::new("prompt", "no prompt named {name:?}")`
on miss. `prompt_command` (commands/mod.rs:284-325) resolves the name/branch/
artifact dir **after** the prompt lookups succeed, so an unknown prompt keeps
its `"prompt"` error instead of surfacing a git error. The `Prompt` command
also takes an optional `STEP_NAME` positional (commands/mod.rs:84-85).

The central error type is `Error { message, source }` (errors.rs:10-12) with
lowercase source tags; the exhaustive production tags include `"model"` and
`"prompt"` for name-lookup misses (commands/mod.rs:216, :275). Output rendering
(main.rs:27-50) prints text to stdout on success / colored stderr on error, or
a JSON envelope (`{"data": ...}` / `{"error": {"message", "source"}}`) to
stdout in `-j` mode.

## Desired End State

1. `orksorksorks.toml` gains a nullable `script` field on each `[[steps]]`
   entry and a new top-level `[[scripts]]` collection of `{ name, content }`
   entries — structure mirrors `[[prompts]]` exactly.
2. A new `script` CLI subcommand prints, to stdout, the `content` of the
   `[[scripts]]` entry whose `name` matches the current step's `script` field —
   raw text, no frontmatter. It accepts an optional `STEP_NAME` positional that
   overrides current-step detection, exactly like `prompt`.
3. Existing configs parse unchanged: `script` is absent ⇒ `None`
   (`#[serde(default)]`), and `scripts` is absent ⇒ empty vec. No version bump,
   no migration code.
4. Two error paths, both tagged `"script"`: (a) step references a script name
   with no matching `[[scripts]]` entry; (b) the resolved step has no `script`
   field at all (`None`). Both produce exit code 1, matching the
   `resolve_prompt` miss contract.

### How we verify it's correct

- `cargo fmt --all` clean; `cargo check` passes; `cargo clippy --tests -- -D
  warnings` passes; `cargo nextest run --no-tests pass` green; forbidden-string
  `rg` gate clean (scripts/test.sh:2-21).
- New unit tests in src/config.rs: `Script`/`Config` round-trip with scripts;
  missing `scripts` ⇒ empty vec; `Step` with no `script` deserializes to `None`;
  `Step` with `script` deserializes to `Some(name)`.
- New unit tests in src/commands/mod.rs: `resolve_script` hit/miss/tag; the two
  `"script"` error branches; dispatch arm for `script` (proven via `"io"` on a
  missing config, per the existing pattern at commands/mod.rs:320+).
- New integration test file `tests/script.rs` (harness copied, not imported):
  current-step script text; explicit `STEP_NAME` wins; JSON `data` envelope;
  unknown-script ⇒ `"script"`; no-script-step ⇒ `"script"`; `XDG_CONFIG_HOME`
  read; no ANSI in any output.

## Patterns to Follow

- **Struct + collection + resolver + command** end-to-end template: `Prompt`
  → `Config.prompts` → `resolve_prompt` → `prompt_command`. The new `Script`
  should be a near-verbatim twin (config.rs:29-34, :54-55; commands/mod.rs:
  269-276, :284-325).
- **Serde defaults for backward compat**: new `scripts` collection gets
  `#[serde(default, skip_serializing_if = "Vec::is_empty")]`; new `script` field
  on `Step` gets `#[serde(default)]` ⇒ `Option<String>` (config.rs:48-55).
- **`determine_step` returns the whole `Step`**, so the new `script` field is
  surfaced without touching step resolution (commands/mod.rs:183-205).
- **Resolve-then-step ordering**: mirror `prompt_command` by resolving the
  script name before any git/branch work, so unknown-script errors stay
  `"script"` rather than `"git"` (commands/mod.rs:284-325).
- **`resolve_*` returns the object/content, never mutates config**; error
  messages use `{name:?}` debug repr; tag is lowercase (commands/mod.rs:216,
  :269-276).
- **No ANSI in callers** — `Error` `Display` owns all coloring (errors.rs:26-34);
  the new command path must not pre-apply color.
- **Harness duplication over imports**: integration tests are black-box (binary
  crate, tests/init_creates_file.rs:9-11) — copy the `git init -b main` +
  `write_config` + `artifact_dir` + `assert_cmd` + JSON envelope helpers from
  prompt.rs into tests/script.rs; each test rejects `\x1b` output.
- **Patterns found that should NOT be followed**: none of consequence — no
  existing code reads `Config.version` for migration (config.rs:38-39), and we
  are deliberately *not* introducing a version check since serde defaults make
  the change invisible to old files.

## Design Decisions

1. **`[[scripts]]` is an array-of-tables of `{ name, content }`** — the
   `Prompt`/`[[prompts]]` twin (config.rs:29-34). Linear first-match name lookup
   preserves order determinism; array-of-tables is the only named-blob precedent
   in the schema.
2. **`Step.script` is `Option<String>` via `#[serde(default)]`** — nullable
   field on `Step`, matching the task's "nullable script field" wording; absent
   in old configs ⇒ `None` with no version bump or migration.
3. **`script` command prints raw `content`, no frontmatter** — the Script text
   is meant to be executed/piped; prepending the `## Important variables` block
   would corrupt it. It ignores `Config.show_frontmatter`.
4. **`script` takes an optional `STEP_NAME` positional** — mirrors `Prompt`
   (commands/mod.rs:84-85), enabling a named-step override; default is current
   step via `determine_step`.
5. **New `resolve_script(config, name) -> Result<String, Error>`** — linear
   scan of `config.scripts`, first `s.name == name` ⇒ `Ok(s.content.clone())`,
   else `Error::new("script", "no script named {name:?}")`, mirroring
   `resolve_prompt` (commands/mod.rs:269-276).
6. **Two `"script"` error branches in `script_command`** —
   (a) unknown reference: `resolve_script` miss (tag `"script"`);
   (b) resolved step's `script` is `None`: `Error::new("script", ...)` naming
   the step. Both exit 1. No silent-empty success path.
7. **Ordering mirrors `prompt_command`**: `read_config` first; then name
   selection (explicit `STEP_NAME` or `determine_step`); then resolve script;
   only then branch/artifact resolution — keeping `"script"` errors ahead of
   `"git"` errors (commands/mod.rs:289-306).

## What We're NOT Doing

- **No `version` bump or migration code.** Serde defaults keep old configs
  parsing identically; the format-version field is left untouched.
- **No frontmatter/templating for script output.** Raw text only;
  `show_frontmatter` remains a `prompt`-only concern.
- **No multi-script support.** One `script: String` field per step — a step that
  needs several scripts stays out of scope (and would be a format change).
- **No `[scripts]` inline-table or map alternative.** Array-of-tables only, to
  match `[[prompts]]`.
- **No changes to `step`/`model`/`thinking`/`prompt` behavior or output.**
- **No child tickets** — all work happens on the main task ticket.

## Open Risks

- **`init_command` and task-frontmatter templates** install a default config;
  should `init` emit an empty `scripts` (it is `skip_serializing_if`-empty, so
  it would simply be absent) — verify the default-serialization test at
  config.rs:106-112 still matches after adding the field.
- **`script` vs `prompt` positional disambiguation** — clap routing for the new
  `Script` variant must not collide with `Prompt`'s `STEP_NAME`; the dedicated
  `try_parse` tests at commands/mod.rs:320+ pin this.
- **Empty-string `script` field** (`script = ""`) deserializes as
  `Some("")`, not `None` — `resolve_script` would then miss and error
  `"script"`, which is acceptable behavior but worth a conscious test.
- **Trailing whitespace / multi-line `content`** in `[[scripts]]` round-trips —
  cover with a test mirroring `prompts_round_trip` (config.rs:173-187) to avoid
  TOML `"""` edge cases.