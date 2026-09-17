# orksorksorks

A small Rust CLI that drives step-based conventions for coding-agent
workflows. It reads an `orksorksorks.toml` config that declares a workflow's
steps, the model and prompt for each step, and optional scripts, then answers
"which step am I on?" from the files present in the artifact directory and
prints the step's prompt, model, or thinking budget on demand.

The name is not a typo: it is a Warhammer reference to Orks shouting
"Orks Orks Orks", and a tongue-in-cheek comparison with coding agents —
individually a little dumb, but more powerful in larger numbers. Worth
spelling carefully: artifacts live under `.pi/orksorksorks/`, and a
`orksworksorks` misspelling would mis-create `.pi/orksworksorks/`
parents.

## What it is

- **Binary-only Rust CLI** (no library target; Rust 2024 edition). Run it as
  `orksorksorks <subcommand>`; `orksorksorks --help` lists everything.
- **Reads one TOML config**: `orksorksorks.toml`, validated against a strict
  schema (unknown keys are rejected).
- **Step-based workflows**: a config declares named `[[steps]]`, each gated on
  a *trigger artifact* — the current step is the last step whose trigger file
  exists in the artifact directory. A step with an empty `trigger_artifact`
  is the default fallback when nothing has matched yet (at most one allowed).
- **Artifact directory**: `$PWD/.pi/orksorksorks/<branch>/` (slashes in the
  branch name become `-`). The `artifact_directory` subcommand prints this
  path; steps are derived from it.
- **Coding-agent plumbing**: `prompt` prints the current step's prompt text,
  prefixed by an `## Important variables` frontmatter block (step, branch,
  artifact directory) that tells the agent where to write its artifacts —
  `show_frontmatter = false` in the config suppresses the prefix. `model` and
  `thinking` print the configured model and reasoning budget; `script` prints
  the raw script text.

### Subcommands

| Subcommand        | What it prints                                       |
| ----------------- | ---------------------------------------------------- |
| `init`            | Writes a commented `orksorksorks.toml` template; refuses to overwrite an existing file |
| `branch`          | The current git branch |
| `artifact_directory` | The artifact directory path (`cwd/.pi/orksorksorks/<branch>/`) |
| `step`            | The current step name (derived from trigger artifacts, or `--step NAME`) |
| `model`           | The model for the current step |
| `thinking`        | The thinking budget for the current step |
| `prompt`          | The current step's prompt, with frontmatter unless disabled |
| `script`          | The current step's raw script content (positional `STEP_NAME`) |

`step`, `model`, `thinking`, and `prompt` take `--config PATH` and
`--step NAME`; `script` takes a positional step name and `--config PATH`.
`-j`/`--json` is a global flag for JSON output.

## What it does not do

- **It never creates the artifact directory or writes artifacts.** The
  `artifact_directory` command only prints the path — no directory is
  created — and step derivation only *checks existence* of trigger files.
  Agents and scripts are responsible for producing the artifacts.
- **It never executes scripts.** `script` prints raw content from the config
  ("meant to be run/piped"); orksorksorks never runs it, and it never shells
  out to the declared scripts.
- **It never edits files.** The only write is `init` creating the config file
  you asked it to create.
- **It never calls an LLM or any network API.** `model`/`thinking` return the
  strings configured in TOML; the crate has no networking code.
- **It is not a general-purpose agent and not a build tool.** It is a
  thin conventions spine (typed errors, colored output, JSON envelope, TOML
  config) for resolving *where you are* in a workflow.
- **It needs a git checkout on a branch** when deriving steps: step
  derivation and `branch`/`artifact_directory` call `git branch
  --show-current`, which fails on a detached HEAD. Pass `--step NAME` (or a
  positional step name to `script`) to skip git for that command — except
  `prompt`, which still resolves the branch for the frontmatter block unless
  `show_frontmatter = false`.

## Installation

**Prerequisites**

- Install a Rust toolchain with `rustup` (https://rustup.rs). The repo pins
  `1.98.1` in `rust-toolchain.toml`, which `rustup` honors automatically.

**Install**

```sh
scripts/install.sh
```

This runs `cargo install --path . --locked` and installs the `orksorksorks`
binary into `~/.cargo/bin` (ensure that directory is on your `PATH`).

**First run**

```sh
orksorksorks init
```

writes a fully commented `orksorksorks.toml` (from
`templates/default.toml`) to your config directory and refuses if the file
already exists. Point it elsewhere with `-c/--config PATH`.

### Where the config lives

`--config PATH` wins when given. Otherwise the path is resolved from
environment variables (in order):

1. `$XDG_CONFIG_HOME/orksorksorks.toml` — only when `XDG_CONFIG_HOME` is set
   to an *absolute* path;
2. `%APPDATA%\orksorksorks.toml` — Windows;
3. `$HOME/.config/orksorksorks.toml` — otherwise;
4. If none of the above resolves, orksorksorks errors with a
   `config-dir` message saying to set `XDG_CONFIG_HOME` or `HOME`.

## A basic `orksorksorks.toml`

```toml
version = "0.1.0"
show_frontmatter = true
[[steps]]
name = "example"
trigger_artifact = "example.md"
model = "small"
script = "my_script"
[[models]]
name = "small"
model = "openrouter/example/model"
thinking = "high"
[[scripts]]
name = "my_script"
content = """
echo "hello"
"""
[[prompts]]
name = "example"
content = """
Your prompt body goes here.
"""
```

The `version` field must equal the supported config version, `"0.1.0"`
(mismatches fail with `config:version`). `show_frontmatter` defaults to
`true` and controls whether `prompt` prefixes the `## Important variables`
block. `[[steps]]` are the workflow's phases: `name` must match a
`[[prompts]]` entry (else `config:missing-prompt`) and `model` must name a
`[[models]]` entry (else `config:missing-model`). `script` is optional and
references a `[[scripts]]` entry. Names must be unique
(`config:duplicate-name`) and non-blank (`config:empty-name`) in the `steps`,
`models`, and `prompts` sections — script names are not validated — and each
step needs a non-empty model reference (`config:empty-model`), trigger
artifacts must be unique across steps (`config:duplicate-trigger`), and at
most one step may use an empty trigger as the default
(`config:multiple-default`).

## JSON output

With `-j/--json`, successful commands print `{"data": ...}` and failures
print `{"error": {"message": ..., "source": ...}}` with exit status 1. The
`source` is a lowercase tag (`io`, `toml::de`, `config:*`, `git`, …) and the
`message` is human-readable.

## License

MIT — see `Cargo.toml`. Homepage and repository:
<https://github.com/alanvardy/orksorksorks>.
