# Research Questions

## Context

Two Rust projects live side by side: a mature CLI at `/Users/vardy/dev/tod`
with a complete developer toolchain, and a fresh, nearly-empty crate at
`/Users/vardy/dev/alanvardy-var-817-copy-over-scripts-and-test-infrastructure`
which currently contains only a manifest, `src/main.rs`, and a `.gitignore`.
Three specific tooling pieces of the mature project are of interest:
`scripts/test.sh`, `rust-toolchain.toml`, and `codecov.yml`. Questions below
explore how each of those works, what they depend on, and what the fresh
crate looks like today.

## Questions

1. **test.sh mechanics and dependencies:** What exactly does
   `tod/scripts/test.sh` run, step by step (which cargo commands, in what
   order, with which flags)? Which external tools does it require to be
   installed (e.g. cargo-nextest, cargo-clippy, ripgrep) and which other
   tod scripts does it invoke (e.g. `./scripts/testcfg_clean.sh`)? For each
   step, how does it behave in a repo that has no `tests/` directory, no
   `.testcfg` fixtures, and no test code yet?

2. **rust-toolchain.toml:** What does `tod/rust-toolchain.toml` pin
   (channel, components, other settings), and how does rustup/cargo consume
   it? Is the pinned toolchain compatible with the destination crate's
   manifest (check `Cargo.toml` `edition` and any MSRV implications)? What
   toolchain is the destination crate currently building with (any local
   toolchain file, or the rustup default)?

3. **codecov.yml configuration and consumers:** What does `tod/codecov.yml`
   configure — code coverage ignore paths, status/threshold rules, comment
   layout — and do the ignore paths reference tod-specific source files?
   How is coverage data produced and uploaded (which cargo/test setup emits
   coverage, which Codecov action or uploader consumes the file), and does
   the config have hardcoded assumptions about the repo layout?

4. **Destination baseline and local conventions:** What exists right now in
   the fresh crate (manifest metadata, edition, dependencies, `src/main.rs`,
   `.gitignore`, `linear-project.md`, any CI or scripts), and are there any
   repo-local conventions — in either project — that prescribe where
   `scripts/`, toolchain, or coverage config files must live (e.g. home
   `AGENTS.md` references `./scripts/test.sh` as a project gate, `.pi/`
   skills)?