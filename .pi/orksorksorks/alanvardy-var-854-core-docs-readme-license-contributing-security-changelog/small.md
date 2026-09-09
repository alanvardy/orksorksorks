# Task

Create core documentation files for the orksorksorks project:

- **README.md**: Purpose, install/build instructions (point to `scripts/install.sh`), local dev setup, test commands (point to `scripts/test.sh`)
- **LICENSE**: MIT license text (Cargo.toml only has `license = "MIT"` string, which is insufficient)
- **CONTRIBUTING.md**: Pre-PR checklist, coding conventions
- **SECURITY.md**: Vulnerability reporting path
- **CHANGELOG.md**: Release notes mechanism
- **AGENTS.md**: Agent/contributor instructions (layout, commands, conventions, error policy)
- **docs/adr/**: Dated decision records directory

Remediation #6 of the audit.

## Why SMALL
Single module (documentation only), localized file creation following existing conventions from scripts/install.sh, scripts/test.sh, and templates/default.toml. No schema/API/UI changes, no cross-cutting concerns, no design sign-off needed.

## Key files (if the recon found any)
- scripts/install.sh (install process reference)
- scripts/test.sh (test commands reference)
- templates/default.toml (documentation conventions)
- Cargo.toml (license field reference)
