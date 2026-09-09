# ADR 0001 — Adopt dated architecture decision records (docs/adr/)

Status: Accepted

Context: Audit remediation #6 asked for a dated decision-records directory
for a project that lacked one. Important technical decisions currently live
only in code and commit history; a lightweight record keeps that history
findable and gives future decisions a stable home.

Decision: Keep architecture decision records as ISO-dated markdown files in
`docs/adr/` (`YYYY-MM-DD-<slug>.md`), each following the Status / Context /
Decision / Consequences template, with `docs/adr/README.md` as the index.
Records are append-only; superseded decisions are marked, not rewritten.

Consequences: Decision history becomes traceable and cheap to search. The
process overhead is small — a short record when a notable decision is made,
plus an index update in `README.md`.