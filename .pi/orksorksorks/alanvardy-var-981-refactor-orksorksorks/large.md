# Task

Refactor the orksorksorks Rust CLI so that `src/commands/mod.rs` (currently 1,459 of the crate's 2,884 total lines — the CLI parser, all subcommand structs, and their handlers in one file) is split into an organized set of submodules. The ticket explicitly calls for researching how to organize the codebase before moving code, and the change must be behavior-preserving (same CLI surface, same output).

## Why LARGE

UNKNOWNS + DESIGN_SIGN-OFF: the ticket itself asks to "research how to organize the codebase" — no organization scheme is given, several viable module groupings exist (by command domain, by shared-helper extraction, by parser-vs-handler split), and this is the central command surface of the crate, so a design pass with human sign-off is expected. A prior attempt (draft PR #29) was never merged, corroborating that a naive one-pass split is not satisfactory.