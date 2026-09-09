//! Layer/architecture guardrails, enforced with `rust_arkitect`.
//!
//! The crate's module hierarchy (top to bottom):
//!
//! ```text
//! main > commands > config > config_dir > git > errors > format
//! ```
//!
//! Real import edges today (every one points downward):
//!
//! ```text
//! commands   -> config, config_dir, errors, format, git
//! config     -> config_dir, errors
//! config_dir -> errors
//! git        -> errors
//! errors     -> format
//! format     -> (nothing crate-internal)
//! ```
//!
//! `commands` is the facade and may import broadly downward; every other
//! module may only depend on modules below it. `rust_arkitect` records the
//! dependencies of `src/commands/mod.rs` as the logical module
//! `orksorksorks::commands`, and stores each dependency exactly as written
//! in the source, so every allowed/forbidden module is listed in both the
//! `crate::` and `orksorksorks::` spellings.

use rust_arkitect::dsl::architectural_rules::ArchitecturalRules;
use rust_arkitect::dsl::arkitect::Arkitect;
use rust_arkitect::dsl::project::Project;
use rust_arkitect::rule::Rule;

/// Assert that `rules` hold for this crate, panicking with the violations.
fn assert_complies(rules: Vec<Box<dyn Rule>>) {
    let result = Arkitect::ensure_that(Project::from_current_crate()).complies_with(rules);
    match result {
        Ok(_) => {}
        Err(violations) => panic!(
            "{} architecture violation(s):\n{}",
            violations.len(),
            violations.join("\n")
        ),
    }
}

/// `commands` (the CLI facade) may import only its five downward modules
/// plus the external crates/`std` modules it uses.
///
/// Adding a new dependency to `commands` — internal or external — must be
/// reflected in this allowlist; that is the point of the guardrail.
#[test]
fn commands_imports_only_downward_modules() {
    #[rustfmt::skip]
    let rules = ArchitecturalRules::define()
        .rules_for_module("orksorksorks::commands")
            .it_may_depend_on(&[
                // Downward internal modules (both spellings).
                "crate::config",
                "orksorksorks::config",
                "crate::config_dir",
                "orksorksorks::config_dir",
                "crate::errors",
                "orksorksorks::errors",
                "crate::format",
                "orksorksorks::format",
                "crate::git",
                "orksorksorks::git",
                // External crates / std paths used by `commands`
                // (including its `#[cfg(test)]` module).
                "clap",
                "std::path",
                "std::io",
                "std::fs",
                "std::env",
                "tempfile",
                "pretty_assertions",
            ])
        .build();

    assert_complies(rules);
}

/// No module imports from a module above it — and therefore no cycles.
///
/// Imports are the only edges between modules, so any cycle would require an
/// upward edge; forbidding every upward edge in the total order
/// `main < commands < config < config_dir < git < errors < format` asserts
/// both "no upward imports" and "no cycles".
#[test]
fn no_upward_imports_or_cycles() {
    #[rustfmt::skip]
    let rules = ArchitecturalRules::define()
        .rules_for_module("orksorksorks::config")
            .it_must_not_depend_on(&[
                "crate::commands",
                "orksorksorks::commands",
            ])

        .rules_for_module("orksorksorks::config_dir")
            .it_must_not_depend_on(&[
                "crate::commands",
                "orksorksorks::commands",
                "crate::config",
                "orksorksorks::config",
            ])

        .rules_for_module("orksorksorks::git")
            .it_must_not_depend_on(&[
                "crate::commands",
                "orksorksorks::commands",
                "crate::config",
                "orksorksorks::config",
                "crate::config_dir",
                "orksorksorks::config_dir",
            ])

        .rules_for_module("orksorksorks::errors")
            .it_must_not_depend_on(&[
                "crate::commands",
                "orksorksorks::commands",
                "crate::config",
                "orksorksorks::config",
                "crate::config_dir",
                "orksorksorks::config_dir",
                "crate::git",
                "orksorksorks::git",
            ])

        .rules_for_module("orksorksorks::format")
            .it_must_not_depend_on(&[
                "crate::commands",
                "orksorksorks::commands",
                "crate::config",
                "orksorksorks::config",
                "crate::config_dir",
                "orksorksorks::config_dir",
                "crate::git",
                "orksorksorks::git",
                "crate::errors",
                "orksorksorks::errors",
            ])

        .build();

    assert_complies(rules);
}
