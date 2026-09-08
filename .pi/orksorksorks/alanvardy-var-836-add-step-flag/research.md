# Research Findings

Repo: `/Users/vardy/dev/alanvardy-var-836-add-step-flag`, branch `alanvardy-var-836-add-step-flag`.
Branch markers: bare `file:line` = current branch; `amc:file:line` = unmerged `add-model-command` branch (read via `git show`, tip `333ab82`); `main` = `2d661d1`.
Topology discovered: current branch has **zero source delta vs main** (only docs/`DELETEME` commits); `add-model-command` was rebased onto `main`'s tip mid-analysis and adds the Model/Thinking/Prompt commands.

## Q1: Command dispatch flow — CLI parse → handler output

### Entry and output layer (identical on all three branches; main.rs is 85 lines)
- `src/main.rs:81-85` `main()` → `commands::Cli::parse()` then `run_command(cli)`.
- `src/main.rs:62-79` `run_command`: reads `cli.json` (`main.rs:63`), calls `commands::select_command(&cli)` (`main.rs:64`), wraps the `Result<String, Error>` in `CommandResult { result, json }` (`main.rs:19-24`), then `output_result(&cr)` (`main.rs:75`).
- `src/main.rs:53-59` `output_result` routes to `output_json` (`main.rs:39-50`; envelope `{"data": ...}` on Ok, `{"error": {"message", "source"}}` on Err) when `cr.json`, else `output_text` (`main.rs:27-36`; `println!` on Ok, `eprintln!("\n\n{e}")` on Err). `main.rs:76-78` `std::process::exit(1)` on error.

### Dispatch site
- `src/commands/mod.rs:83-85` `select_command(cli)` = `select_command_with_env(cli, &ConfigEnv::from_env())` — the only public entry (`main.rs:64`).
- `src/commands/mod.rs:66-80` `select_command_with_env` matches `&cli.command`:
  - `Init` `mod.rs:68-71` → `config_file_path_with_env(config.as_deref(), env)?` → `init_command(&path)`.
  - `Branch` `mod.rs:72` → `branch_command()`; `ArtifactDirectory` `mod.rs:73` → `artifact_directory_command()`.
  - `Step` `mod.rs:74-78` → `config_file_path_with_env(config.as_deref(), env)?` → `step_command(&path, source)` (source threaded through).

### Step handler — current branch
- `src/commands/mod.rs:169-177` `step_command(path, source)`:
  1. `read_config(path, source)?` (`mod.rs:173`) — config read intentionally runs **before** git resolution so a missing config deterministically fails with source `"io"` (doc comment `mod.rs:162-168`).
  2. `std::env::current_dir()?` (`mod.rs:174`).
  3. `artifact_dir_path(&cwd, &git::current_branch()?)` (`mod.rs:175`).
  4. `determine_step(&cfg, &artifact_dir)` (`mod.rs:176`) → the step name `String`, which is what the handler returns (nothing else consumes it on this branch).

### add-model-command handlers (unmerged, does not compile — see Q4)
- `determine_step` returns the whole matched `Step`: `amc:src/commands/mod.rs:180`.
- `step_command` `amc:mod.rs:223-232` → `Ok(step.name)` (`amc:mod.rs:231`).
- `model_command(path)` `amc:mod.rs:237-244` → `resolve_model(&cfg, &step.model)?` then `Ok(model.model)` (`amc:mod.rs:242-243`) — the concrete model string.
- `thinking_command(path)` `amc:mod.rs:249-256` → same shape, `Ok(model.thinking)` (`amc:mod.rs:255`).
- `prompt_command(path, step_name)` `amc:mod.rs:272-284`: explicit positional `step_name` short-circuits (`amc:mod.rs:274-276`); else derives via cwd + branch + `determine_step(...).name` (`amc:mod.rs:277-281`), then `resolve_prompt(&cfg, &name)` (`amc:mod.rs:283`).

### Config/env reaching handlers
- `ConfigEnv::from_env()` (`src/config_dir.rs:26-36`) is the only place the process env is read; every handler below dispatch receives only a resolved `(path, ConfigPathSource)` pair from `config_file_path_with_env` (`src/config_dir.rs:106-113`); `env` stays in `select_command_with_env`.

## Q2: clap argument declaration and parsing

- Dependency: `Cargo.toml:11` `clap = { version = "4.6.6", features = ["derive"] }`.
- Root struct `Cli` `src/commands/mod.rs:30-37` (`#[derive(Parser, Clone)]`, `mod.rs:21`): exactly two fields — the global flag and the subcommand enum.
- **Global flag** `-j/--json`: `src/commands/mod.rs:32-33` `#[arg(short = 'j', long, global = true, default_value_t = false)] pub json: bool`. `global = true` permits it before or after the subcommand (tests use after: `tests/step.rs:66-67`). Consumed in `run_command` at `src/main.rs:63` before dispatch; **never passed to handlers**.
- **Per-subcommand option** `--config`: two spellings exist —
  - `Init`: `src/commands/mod.rs:43-47`, `#[arg(short = 'c', long, value_parser = clap::value_parser!(PathBuf))]` (`mod.rs:45`).
  - `Step` (and amc Model/Thinking/Prompt): `#[arg(long, value_name = "CONFIG")]` (`mod.rs:60`; amc `:68, :76, :89`) on an `Option<PathBuf>` field.
- **Positional** `STEP_NAME` (amc only): `amc:src/commands/mod.rs:84` `#[arg(value_name = "STEP_NAME")] step_name: Option<String>` on `Prompt` (`amc:mod.rs:81-90`). No `short`/`long` token + `Option<String>` ⇒ free positional; declared before `config` so CLI shape is `prompt [STEP_NAME] [--config PATH]`.
- End-to-end trace of `step --config` (same shape for amc commands):
  1. Parse: `Cli::try_parse_from([...])` in unit tests (`mod.rs:446-456` asserts bound `Step { config }`), or `Cli::parse()` in `main.rs:83`.
  2. Dispatch destructure: `Commands::Step { config } =>` arm `mod.rs:74-78`; `config.as_deref()` passes the parsed option into resolution.
  3. Resolution: `config_file_path_with_env` `src/config_dir.rs:106-113` — `Some(path)` → `(path, ExplicitFlag)`; `None` → `resolve_config_dir_with_env(env)` + `FILE_NAME` (`orksorksorks.toml`, `config_dir.rs:9`).
  4. Handler: `step_command(&path, source)` `mod.rs:169-177`.
- amc positional flow: prompt arm `amc:mod.rs:115-118` destructures `step_name` and calls `prompt_command(&path, step_name.clone())`; normalization to a concrete name happens inside the handler (`amc:mod.rs:274-276`).

## Q3: Config schema and step-name consumption

### Current branch `src/config.rs`
- `Step { pub name: String, pub trigger_artifact: String }` (`src/config.rs:7-12`) — no `model` field.
- `Config { version, steps }` (`src/config.rs:19-25`), `steps: Vec<Step>` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` (`config.rs:23-24`). `Default` = version `"0.1.0"`, empty steps (`config.rs:27-33`).
- `read_config(path, source)` `src/config.rs:41-57`: missing/unreadable → `Error::new("io", "could not read config file at {path} ({source}): {e}")` (`config.rs:47-54`); malformed TOML → `"toml::de"` via `From<toml::de::Error>` (`src/errors.rs:44-49`).

### add-model-command `src/config.rs`
- `Step` gains `pub model: String` (`amc:src/config.rs:12-13`, a key into `Config::models`); new `Model { name, model, thinking }` (`amc:config.rs:17-25`) and `Prompt { name, content }` (`amc:config.rs:28-34`).
- `Config` gains `models: Vec<Model>` (`amc:config.rs:48-49`) and `prompts: Vec<Prompt>` (`amc:config.rs:51-52`), same serde-default pattern. `read_config` keeps the 2-arg signature (`amc:config.rs:71`).

### Step-name consumption
- Current branch: `determine_step` returns the name `String` (`src/commands/mod.rs:138-160`); `step_command` returns it unchanged (`mod.rs:176`); nothing else consumes step names.
- amc: `determine_step` returns the whole `Step` (`amc:mod.rs:180`). `model_command`/`thinking_command` key off `step.model`; `prompt_command` keys `resolve_prompt` off the step **name** (`amc:mod.rs:283`).
- `resolve_model` `amc:mod.rs:207-214`: linear scan of `config.models` by exact `==` name match, first match wins; miss → `Error::new("model", "no model named {name:?}")` (`amc:mod.rs:213`), Debug formatting quotes the name.
- `resolve_prompt` `amc:mod.rs:260-267`: linear scan of `config.prompts`; miss → `Error::new("prompt", "no prompt named {name:?}")` (`amc:mod.rs:266`).
- No load-time cross-validation: `Step::model` is never checked against `Config::models` at parse; unknown names surface only when the resolving command runs.
- `determine_step` no-match: after reverse scan, if no default (empty-trigger) step exists → `Error::new("step", "{artifact_dir}: no trigger artifact matched")` (`src/commands/mod.rs:156-159`); identical on amc (`amc:mod.rs:197-201`).

## Q4: Branch divergence in step derivation

- **Current branch vs main: source-identical.** Current tip `c78bf01` = main `2d661d1` + docs-only commits (`9586813` added a 1-line `DELETEME`, `c78bf01` removed it and added `.pi/` docs). `git diff main HEAD -- src/` is empty; blob hashes match. The add-step-flag branch carries **no code change**.
- **`determine_step` signature**: current/mod.rs:138 `-> Result<String, Error>`; amc:mod.rs:180 `-> Result<Step, Error>` (doc `amc:mod.rs:172-179` explains callers need `name` vs `model`).
- **Config-dir plumbing** (current/main): `ConfigEnv` `src/config_dir.rs:17-23`; `from_env` `:26-36`; `ConfigPathSource` enum `:45-52` with pinned Display phrases `:54-64`; `resolve_config_dir_with_env` `:72-92`; `config_file_path_with_env` `:106-113`. **No plain `config_file_path` exists on current/main.**
- **amc config_dir.rs is identical to main** (post-rebase; `git diff main amc -- src/config_dir.rs` empty) — but the amc Model/Thinking/Prompt arms call a **nonexistent** `crate::config_dir::config_file_path(config.as_deref())?` (`amc:mod.rs:108-109, 112-113, 116-117`) and their handlers call one-arg `read_config(path)` (`amc:mod.rs:238, 250, 273`) against the 2-arg definition (`amc:src/config.rs:71`). Verified: `cargo check` on an extracted amc tree fails with 3× E0425 + 3× E0061; the `Step` arm and `step_command` are fully wired. The branch is mid-refactor and does not compile.
- Pre-rebase amc (`049504a`, still in the object store) held the OLD api: `config_file_path(explicit)` reading the process env directly and tests mutating env via `unsafe { std::env::set_var/remove_var }` — the flakiness fixed on main by the ConfigEnv phases (`322fa19`→`685daac`, `2d661d1`).
- Commands per branch: current/main = Init, Branch, ArtifactDirectory, Step (`mod.rs:41-63`); amc adds Model/Thinking/Prompt (`amc:mod.rs:65-95`).

## Q5: Artifact-dir path composition and filesystem reads

- `artifact_dir_path(cwd, branch)` `src/commands/mod.rs:111-117`: pure string composition `format!("{}/.pi/orksorksorks/{}/", cwd.display(), branch.replace('/', "-"))` — branch slashes → hyphens, **trailing slash is part of the contract** (doc `mod.rs:107-110`, `:134-136`). No I/O, no error path.
- Callers: `artifact_directory_command` `mod.rs:126-128` (returns it plain, no creation); `step_command` `mod.rs:175`; amc `model_command` `amc:mod.rs:239-240`, `thinking_command` `amc:mod.rs:251-252`, `prompt_command` `amc:mod.rs:279-280`.
- Branch source: `git::current_branch()` `src/git.rs:8-10` → `current_branch_in(dir)` `src/git.rs:17-26` runs `git branch --show-current` with `.current_dir(dir)`. `parse_branch_output` `src/git.rs:28-36`: nonzero exit → `Error::new("git", stderr.trim())`; empty stdout → `Error::new("git", "not on a branch (detached HEAD)")`. A failed spawn (git not on PATH) rides `From<std::io::Error>` → `"io"` (`git.rs:7` doc).
- `determine_step` filesystem behavior `src/commands/mod.rs:138-160`:
  - Reverse-iterates `config.steps`.
  - Empty `trigger_artifact` = default step, never an existence check; first seen in reverse (last forward) wins via `default.get_or_insert_with(...)` (`mod.rs:141-147`).
  - Real match: `Path::new(&format!("{artifact_dir}{}", step.trigger_artifact)).try_exists()?` (`mod.rs:148-151`); first hit returns its name (`mod.rs:150`).
  - After the loop: default wins if captured (`mod.rs:153-155`); else `Error::new("step", "{artifact_dir}: no trigger artifact matched")` (`mod.rs:156-159`).
  - Failure modes: `try_exists()?` can propagate an io error before fallbacks; `current_dir()`/`git::current_branch()` failures in `step_command` (`mod.rs:174-175`) occur after config read, so a bad git state surfaces as source `"git"`.
- Unit tests pin the contract: `artifact_dir_path_*` `mod.rs:278-308` (trailing slash, slash→hyphen, pure); `determine_step_*` `mod.rs:312-499` (present artifact, last-in-reverse, empty-trigger default, default never shadows a real match, latest empty-trigger wins, no-match error with source `"step"`).

## Q6: Integration test patterns for step commands

- **No shared helper module** — all test files duplicate small helpers. `init_git_repo()` is copy-pasted in `tests/step.rs:7`, `amc:tests/model.rs:7`, `amc:tests/prompt.rs:7`; `write_config`, `artifact_dir` likewise per file. Dev-deps: `Cargo.toml:20-22` `assert_cmd 2.2.2`, `predicates 3.1.4`, `pretty_assertions 1.4.1`.
- **Temp git repo setup**: `tempfile::tempdir()` + `git init -b main` leaving the repo **unborn** — `git branch --show-current` still reports the branch name (doc `tests/step.rs:4-6`). A committed-repo variant `git_repo_on_branch` (`git init` + `checkout -b` + commit) exists in `tests/branch.rs:8-33` / `tests/artifact_directory.rs:8-35` because CI's `actions/checkout` leaves the ambient tree detached (`tests/branch.rs:3-5`).
- **Fixture config**: written to repo root as `orksorksorks.toml` via `std::fs::write(dir.join("orksorksorks.toml"), concat!(...))`. Steps fixture: `tests/step.rs:18-32` (steps `one`/`two`, triggers `first.txt`/`second.txt`). amc adds `model = "small|high"` per step plus `[[models]]` (`amc:tests/model.rs:20-45`) and `[[prompts]]` with multiline TOML `content = """..."""` (`amc:tests/prompt.rs:24-51`).
- **Artifact dir + triggers**: `artifact_dir(dir) = dir.join(".pi").join("orksorksorks").join("main")` (`tests/step.rs:34-36`), mirroring `artifact_dir_path`; tests `create_dir_all` then touch empty trigger files (`tests/step.rs:43-44`).
- **CLI invocation**: `assert_cmd::Command::cargo_bin("orksorksorks").unwrap()` (`tests/step.rs:49`); args as slices `.args(["step", "--config", "orksorksorks.toml", "-j"])` (`tests/step.rs:49-51`) or chained `.arg(...)`; `.current_dir(dir.path())`; env via `.env("XDG_CONFIG_HOME", &config_dir)` (`tests/step.rs:190`), `.env_remove("HOME")` (`tests/init_creates_file.rs:115-116`). Crate is binary-only (no lib target), so tests assert fixtures by string (`tests/init_creates_file.rs:20-21`).
- **Assertion conventions**:
  - Text: `output.stdout` decoded via `String::from_utf8_lossy`, exact compare `assert_eq!(stdout.trim_end(), "two")` (`tests/step.rs:56-58`); substring for multiline (`amc:tests/prompt.rs:69`).
  - amc model/thinking/prompt tests additionally strip a trailing bell `\x07`: `.trim_end_matches('\x07').trim_end()` (`amc:tests/model.rs:68, 198, 220, 241, 340`).
  - No-ANSI guard on nearly every test: `assert!(!stdout.contains('\x1b'))` (`tests/step.rs:59, 81, 124, 161, 197`; also stderr `:228-229`, JSON `:305`) — color is a compile-time no-op under test at `src/format.rs:7-12`.
  - JSON success: parse into `serde_json::Value`, assert `v["data"].as_str()` (`tests/step.rs:72-73`); envelope from `src/main.rs:42`.
  - JSON error: assert both `json["error"]["message"].as_str()` and `["error"]["source"].as_str()` (`tests/step.rs:292-298`, `src/main.rs:46`).
  - Text errors go to **stderr** (`eprintln!` `src/main.rs:33`), JSON errors to **stdout** (`src/main.rs:47`); both `exit(1)`.
  - Error probes assert the config path verbatim plus resolution phrase `"specified via --config"` (`tests/step.rs:219-229`, phrase `src/config_dir.rs:59`) / `"resolved from XDG_CONFIG_HOME"` (`tests/step.rs:253-266`, `src/config_dir.rs:60`).
  - Exit-code-only style: `.assert().failure()` (`tests/step.rs:89-93`).

## Cross-Cutting Observations

- Single dispatch chokepoint: `select_command_with_env` (`src/commands/mod.rs:66-80`) is the one place a parsed subcommand becomes a handler call; all config-resolution side effects (env reads) happen there, and handlers receive only plain values (`path`, `source`, `step_name`).
- The `(path, source)` pair is the universal config contract: `read_config` embeds both in the `"io"` error message (`src/config.rs:41-57`), and `ConfigPathSource::Display` phrases (`src/config_dir.rs:54-64`) are pinned by both unit tests (`config_dir.rs:196-203`) and integration tests (`tests/step.rs:219-266`).
- Error convention: `Error::new(source_tag, message)` (`src/errors.rs:18-24`) with tags `io`, `toml::de`, `git`, `config-dir`, `step`, `model`, `prompt`; never bare status codes.
- `determine_step` is a pure function over `(Config, artifact_dir_str)` with a string-composition contract (trailing slash); both branches keep the same reverse-priority default logic, differing only in return type (`String` vs `Step`).
- Path composition is deliberately I/O-free (`artifact_dir_path`) and git is shelled out only via `git branch --show-current` (`src/git.rs:17-26`); the binary never reads the artifact dir itself during step determination — it only stats trigger files with `try_exists`.
- amc fixture divergence: step fixtures add a `model` key to every `[[steps]]` (7 insertions in `amc:tests/step.rs`), meaning a single TOML fixture cannot satisfy both branches' schema.

## Open Areas

- The intended work on this branch ("add step flag") has **no code on the branch yet** — Q1–Q6 describe `main`'s state plus the unmerged amc commands; the `--step` flag that presumably overrides or supplements `determine_step` does not exist anywhere.
- amc `prompt_command`'s derivation shape (`amc:mod.rs:277-281`) is the only current-branch-adjacent precedent for "explicit value overrides derivation," but it sits on a branch that does not compile — its exact intended final form (post-refactor) is unknown.
- Whether `-j/--json` can appear in both positions under clap 4.6.6 (`global = true`) is exercised only after the subcommand in integration tests; before-subcommand position is untested at the integration layer.