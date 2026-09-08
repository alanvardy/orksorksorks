# Research Findings

## Q1: How is the config file location resolved (explicit `--config` vs. environment), and through which call sites do the resolved path and the read attempt pass? Where is the resolved path known but not used?

### Findings
- **Dispatch spine.** `main()` parses CLI args and calls `run_command(cli)` (`src/main.rs:82-84`). `run_command` makes the single dispatch call `commands::select_command(&cli)` (`src/main.rs:64`) and only ever sees the resulting `Result<String, Error>`.
- **Single resolution function for both cases.** `config_file_path(explicit: Option<&Path>)` (`src/config_dir.rs:16-20`): explicit `Some(path)` is passed straight through (`config_dir.rs:18`); `None` resolves the agent dir and appends `FILE_NAME` = `"orksorksorks.toml"` (`config_dir.rs:8`, `:19`).
- **Environment-driven resolution.** `resolve_config_dir()` (`src/config_dir.rs:24-41`): `XDG_CONFIG_HOME` wins only when absolute (`config_dir.rs:27-31`); Windows uses `%APPDATA%` under `cfg!(windows)` (`config_dir.rs:32-35`); otherwise `$HOME/.config` (`config_dir.rs:36-39`); else `Err(Error::new("config-dir", "could not determine a config directory: set XDG_CONFIG_HOME or HOME"))` (`config_dir.rs:40-45`).
- **Both dispatch arms call it identically.** Init arm: `config_file_path(config.as_deref())?` → `init_command(&path)` (`src/commands/mod.rs:69-70`). Step arm: same call → `step_command(&path)` (`src/commands/mod.rs:75-76`). These are the only two non-test call sites of `config_file_path`; all other occurrences (`config_dir.rs:55,66,79,92,102,111`) are inside `#[cfg(test)] mod tests`.
- **Read attempt.** `step_command` → `read_config(path)` (`src/commands/mod.rs:164`) → `std::fs::read_to_string(path)?` (`src/config.rs:40`). The `?` converts the `io::Error` via `From<std::io::Error>` (`src/errors.rs:39-46`).
- **Where the resolved path is known but not used.**
  - `select_command` returns only `Result<String, Error>` (`src/commands/mod.rs:66`); the path never leaves it. `run_command` (`src/main.rs:64`) and both renderers (`src/main.rs:27-58`) never receive it.
  - Inside `step_command`, `path` is used only as the `read_config` argument (`src/commands/mod.rs:164`); on failure the `?` propagates the flattened `io` error upward with no path annotation.
  - `read_config` never folds `path` into the error it propagates (`src/config.rs:39-43`).
- **Contrast: init success does embed the path** — `Ok(crate::format::green_string(&format!("✓ Created {}", path.display())))` (`src/commands/mod.rs:94-96`).

## Q2: How do errors flow from raise to render? Where is the single chokepoint (text vs. JSON, ANSI, exit code), and what does it have access to?

### Findings
- **Raise sites** (only four `Error::new` sites in non-test code): `"config-dir"` (`src/config_dir.rs:42-45`), `"step"` no-trigger-match (`src/commands/mod.rs:150-153`), `"git"` stderr pass-through (`src/git.rs:30`), `"git"` detached HEAD (`src/git.rs:34`).
- **Propagation** is by `?` through `Result`; every unrelated error type is flattened by a `From` impl at conversion time: `io` (`src/errors.rs:39-46`), `toml::ser` (`src/errors.rs:48-55`), `toml::de` (`src/errors.rs:57-64`). By the time an error reaches rendering, its original type is gone — only `Error { message: String, source: String }` (`src/errors.rs:10-12`) survives.
- **Single chokepoint.** `run_command` (`src/main.rs:62-77`) gathers everything: `let json = cli.json` (`src/main.rs:63`), `let result = select_command(&cli)` (`src/main.rs:64`), wraps in `CommandResult { result, json }` (`src/main.rs:15-18`, `:65-74`), calls `output_result(&cr)` (`src/main.rs:75`), then exits `1` on any `Err` (`src/main.rs:76-77`). Exit code ignores the JSON flag — any error exits 1.
- **Text vs. JSON branch.** `output_result` (`src/main.rs:53-58`) → `output_json` (`src/main.rs:39-51`) or `output_text` (`src/main.rs:27-37`).
- **Text + ANSI chokepoint.** `output_text` prints `eprintln!("\n\n{e}")` to stderr (`src/main.rs:33`); `{e}` invokes `fmt::Display for Error` (`src/errors.rs:26-34`), the single text formatter: `Error from {source}:\n{message}` with source yellow, message red (`src/errors.rs:30-32`). The `\n\n` prefix is added at the call site, not in `Display`.
- **ANSI chokepoint.** All color helpers route through `format::apply_color` (`src/format.rs:6-11`), which strips ANSI entirely under the compile-time `cfg!(test)` (`src/format.rs:7`); `green_string`/`red_string`/`yellow_string` (`src/format.rs:14-23`) all call it. `src/format.rs:48` documents the strip behavior in tests.
- **JSON bypasses Display/ANSI entirely.** `output_json` reads the two public fields directly: `serde_json::json!({"error": {"message": e.message, "source": e.source}})` (`src/main.rs:46`); success is `{"data": data}` (`src/main.rs:42`).
- **What the chokepoint has access to at `main.rs:75`:** only `cr.result` (either a pre-colored success string or an `Error` whose message is already fully composed text) and `cr.json`. It has **no** access to the config path, config contents, std io/toml/git internals, env, or cwd — those are burned into `message`/`source` at the raising frames.

## Q3: How does the `std::io::Error` → central `Error` conversion work, and what path info does it carry or lose?

### Findings
- **Single conversion impl.** `impl From<std::io::Error> for Error` (`src/errors.rs:39-46`): `source` hard-coded to `"io"`, `message` = `e.to_string()` (the std Display text, verbatim).
- **How `to_string()` behaves (fork stdlib, toolchain 1.98.1 per `rust-toolchain.toml:3`):** `e.to_string()` delegates to `Display for io::Error`, which dispatches on the internal variant — `ErrorData::Os(code)` → `"<strerror text> (os error <code>)"`; `Custom` → inner error's Display; `Simple`/`SimpleMessage` → kind/message text.
- **`io::Error` in this fork carries no path.** The struct is `repr: Repr` (`…/library/core/src/io/error.rs:118-128`) with variants `Os`/`Custom`/`Simple`/`SimpleMessage` (`:146-152`); a repo-wide search for `with_path` returns **zero matches** — upstream Rust's `io::Error::with_path` (which appends `": path"` to fs errors) does not exist in this fork. System-call errors are produced errno-only by `cvt`/`cvt_r` (`…/library/std/src/sys/pal/unix/mod.rs:240-252`); path-bearing entry points (`File::open_c` `…/library/std/src/sys/fs/unix.rs:1377-1398`) never re-attach the path.
- **Consequence:** a missing config file yields `message = "No such file or directory (os error 2)"` with **no filename/path text anywhere**. The config path is available at the call site (`src/config.rs:40` passes it to `read_to_string`) but never enters the message.
- **Field survival through the conversion:** OS strerror text → survives (as `message` text); numeric errno → survives only as the `(os error N)` suffix text; typed `ErrorKind`/`RawOsError` → lost; inner custom cause chain → lost (only its Display text flattened); source-tag → not derived, hard-coded `"io"` (`src/errors.rs:42`).
- **All `io` → `Error` conversion sites** (all via `?`/`FromResidual`): `src/config.rs:40` (read), `src/commands/mod.rs:86,91,93` (init write/sync), `src/commands/mod.rs:121,165` and `src/git.rs:9` (`current_dir`), `src/commands/mod.rs:143` (`try_exists`), `src/git.rs:21` (process spawn). `src/config_dir.rs` has no io conversion — its only error path constructs `Error::new("config-dir", …)` directly (`src/config_dir.rs:42-45`).
- **Tests pin the behavior loosely:** `from_io_error_tags_io` (`src/errors.rs:80-86`) constructs an `io::Error` with an explicit message and asserts `source == "io"` + message substring — no path involved.

## Q4: What conventions exist for error message content, and where is coloring applied?

### Findings
- **Documented conventions** (`src/errors.rs:5-7`): `source` is a lowercase tag (e.g. `"io"`, `"toml::ser"`), `message` is the human-readable description, and `Display` owns all coloring — callers must not pre-apply ANSI. `Error::new`'s doc mandates `&format!(...)` for interpolated messages (`src/errors.rs:16-17`).
- **Source-tag naming:** all lowercase. Single word: `"io"` (`src/errors.rs:42`), `"step"` (`src/commands/mod.rs:151`), `"git"` (`src/git.rs:30,34`); kebab-case `"config-dir"` (`src/config_dir.rs:43`); crate-namespaced `"toml::ser"`/`"toml::de"` (`src/errors.rs:51,60`) mirroring the external crate module path.
- **All authored messages** start lowercase, end with no terminal punctuation:
  - Static: `"could not determine a config directory: set XDG_CONFIG_HOME or HOME"` (`src/config_dir.rs:44`); `"not on a branch (detached HEAD)"` (`src/git.rs:34`).
  - **Only one error message interpolates a runtime value:** `&format!("{artifact_dir}: no trigger artifact matched")` (`src/commands/mod.rs:150-153`) — value emitted bare (unquoted, no brackets), colon-space before the lowercase clause. Same bare style in the success message `format!("✓ Created {}", path.display())` (`src/commands/mod.rs:94-96`).
  - Verbatim external text: `Error::new("git", stderr.trim())` (`src/git.rs:30`) — git stderr becomes the whole message, pinned by `git.rs:110-115`.
- **The `"io"`/`"toml::de"` paths never interpolate the known path** — `read_config` forwards the underlying error text untouched (`src/config.rs:39-43`); same for all other `?` sites (none touch `Error::new`, none insert a path).
- **Path rendering idiom:** paths become strings exclusively via `.display()` (`src/commands/mod.rs:96`, `:108`; tests at `mod.rs:190,318,340,360,381,404,424,483`). No message wraps an interpolated value in quotes anywhere in `src/`.
- **Coloring placement differs by path:**
  - Errors: at **render time**, inside `Display` (`src/errors.rs:30-32`), via `yellow_string(source)` + `red_string(message)`; stored strings stay plain (`src/errors.rs:18-22`).
  - Success: at **construction time** — `green_string(format!("✓ Created {}", …))` wraps before return (`src/commands/mod.rs:94`), printed plain later (`src/main.rs:30`).
  - JSON: never colored (`src/main.rs:46` reads raw fields).

## Q5: How are config-not-found and error-output behaviors covered by tests?

### Findings
- **Missing-config unit tests assert only the tag:**
  - `read_config_missing_file_tags_io` — `read_config(&dir.path().join("nope.toml")).unwrap_err(); assert_eq!(err.source, "io")` (`src/config.rs:107-112`, assert at `:111`). No message/path assertion.
  - `select_command_routes_step` — dispatches with `config: Some(PathBuf::from("definitely-missing-config-file.toml"))` (`src/commands/mod.rs:457-471`, setup `:462-463`, assert at `:469-470`). Comment at `:467-468` records the contract: "step_command reads the (missing) config first → io".
- **Integration regression for the missing-config path:** `step_without_flag_ignores_cwd_config` (`tests/step.rs:198-213`) — a config deliberately absent from the XDG dir while a CWD config exists; asserts only `.assert().failure()` (`tests/step.rs:211-212`) — non-zero exit, **nothing** about source tag, message, or JSON.
- **No test anywhere asserts error message content or a path string for a missing config file**, and no test asserts the text-mode stderr line for a missing config.
- **Complete source-tag assertion inventory:**
  - `"io"`: `src/config.rs:111`, `src/commands/mod.rs:470`, `src/errors.rs:84-85`; integration `"source":"io"` in tests/init_creates_file.rs:84 (read-only dir).
  - `"toml::de"`: `src/config.rs:125` (malformed TOML), `src/errors.rs:113`.
  - `"toml::ser"`: `src/errors.rs:99`.
  - `"config-dir"`: `src/config_dir.rs:112`; integration `"source":"config-dir"` in tests/init_creates_file.rs:118-122 (`HOME`/`XDG_CONFIG_HOME` removed).
  - `"step"`: `src/commands/mod.rs:485-487` (+ message contains `"no trigger artifact matched"` — only unit test asserting message content).
  - `"git"`: `src/git.rs:91-92`, `:105-106` (contains `"detached HEAD"`), `:113-114` (exact `"fatal: not a git repository"`).
- **Message-content assertions:** `errors.rs:76-77` (Display contains `"Error from test_tag"` + `"something broke"`), `errors.rs:85` (`"no such file"` from a constructed io::Error), `mod.rs:487` (`"no trigger artifact matched"`), `git.rs:92,106,114`. Success message `"✓ Created {}"` asserted as path-containing substring at tests/init_creates_file.rs:44-48 and tests/json_output.rs:29-33,37-41.
- **JSON-shape assertions:** error envelope is `{"error": {"message", "source"}}` (`src/main.rs:46`); exercised only via substring checks — `"source":"io"` (tests/init_creates_file.rs:84), `"source":"config-dir"` (tests/init_creates_file.rs:122), raw-field round-trip at `errors.rs:119-123`. The literal `"error"` envelope key is never asserted; `"data"` key is (tests/json_output.rs:14, reparsed as `serde_json::Value` at `:19`).
- **Exit codes:** only producer is `std::process::exit(1)` on any `Err` (`src/main.rs:76-77`); **no test asserts an exact numeric code** — all uses are `success()` (0) / `failure()` (non-zero) booleans.
- **ANSI assertions:** integration tests assert stdout has no `'\x1b'`: tests/json_output.rs:40-41, tests/step.rs:80,125,193, tests/branch.rs:57, tests/artifact_directory.rs:57. Unit test at `src/format.rs:33-46` (comment `:48`).
- **Platform/test gating:** `#[cfg(test)]` mod tests in `src/config.rs:45+`, `src/errors.rs:66+`, `src/config_dir.rs:48+`, `src/commands/mod.rs:170+`, `src/git.rs:39+`, `src/format.rs:26+`. `cfg!(test)` gates ANSI (`src/format.rs:7`). Only platform conditional is `if cfg!(windows)` for APPDATA (`src/config_dir.rs:32-35`). **No test function is gated** by `#[cfg(unix)]`/`#[cfg(windows)]`.

## Cross-Cutting Observations
- **Everything funnels through a two-field `Error`.** There is exactly one error type (`src/errors.rs:10-12`); both renderers and every test assertion consume only `message` + `source`. Any context (like a config path) that must appear to the user has to be embedded in `message` at the raising site — the render chokepoint (`src/main.rs:62-77`) structurally cannot add it.
- **The path is dropped exactly once, at the furthest downstream point it exists:** `config.rs:40` (`read_to_string`) — the fork's stdlib `io::Error` carries no path, and `read_config`/`step_command` don't re-attach it. The identical path string is displayed elsewhere via `path.display()` (`src/commands/mod.rs:94-96`), so rendering it already has precedent.
- **Two rendering invocations sit in unrelated layers:** success colorizes at construction (`mod.rs:94`), errors colorize at display (`errors.rs:30-32`); JSON bypasses both. The `cfg!(test)` ANSI strip (`src/format.rs:7`) is the linchpin that lets both unit and integration tests assert plain text.
- **Message conventions are consistent and small:** lowercase first letter, no trailing punctuation, bare interpolated values, source tags lowercase/kebab/`::`-namespaced, doc-mandated `&format!` for dynamic messages (`src/errors.rs:16-17`).
- **Test discipline is tag-centric, not content-centric:** every error-path test keys on `source`; message strings are asserted in only four places (git stderr verbatim, detached HEAD, step no-match, Display smoke test). Missing-config tests assert `source == "io"` only — nothing pins the message text, so message changes there are currently unconstrained.

## Open Areas
- Why the fork's stdlib lacks `with_path` entirely vs. upstream — confirmed empirically (zero matches), but the reasoning (fork divergence) is not documented in the repo.
- The intended behavior of the confusing comment at tests/init_creates_file.rs:40 ("the child still applies color") vs. the explicit ANSI-free assertions elsewhere — the comment text contradicts the assertions around it.
- No documentation of whether a path-bearing `"io"` message would be expected to keep bad characters (spaces, quotes, ANSI from filenames) under the current bare-interpolation convention — no existing message interpolates a path on the error side.