# Architecture Decision Records

This directory records notable architecture and design decisions for
orksorksorks as lightweight, append-only documents.

## Convention

- One file per decision, named `YYYY-MM-DD-<slug>.md` (ISO date, kebab-case
  slug).
- Each record follows the Status / Context / Decision / Consequences
  template:
  ~~~markdown
  # ADR NNNN — <title>

  Status: <Accepted | Proposed | Superseded by ADR NNNN>

  Context: <why the decision matters>

  Decision: <what was decided>

  Consequences: <what this enables or costs>
  ~~~
- Records are **appended**: once committed, a record is never rewritten. If a
  decision changes, a new record supersedes it and the superseded record's
  status is updated to *Superseded by ADR NNNN*.
- This `README.md` is the index: add new records here when they land.

## Records

- [2026-09-09-adopt-dated-architecture-decision-records.md](2026-09-09-adopt-dated-architecture-decision-records.md) —
  ADR 0001: adopt dated architecture decision records (`docs/adr/`)
- [2026-09-09-typed-errors-over-bare-status-codes.md](2026-09-09-typed-errors-over-bare-status-codes.md) —
  ADR 0002: typed errors over bare status codes (retrospective)