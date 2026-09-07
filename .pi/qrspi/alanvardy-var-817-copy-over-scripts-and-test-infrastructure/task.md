# VAR-817 — Copy over scripts and test infrastructure

Bring three specific pieces of developer tooling from the sibling `../tod`
project into this new, nearly-empty Rust crate (`orksorksorks`):
`scripts/test.sh`, `rust-toolchain.toml`, and `codecov.yml`. The user has
explicitly ruled out all other tod tooling (release scripts, CI workflows,
test fixtures, git hooks, commit/version tooling).