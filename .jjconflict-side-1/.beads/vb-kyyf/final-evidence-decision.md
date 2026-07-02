# vb-kyyf Final Evidence Decision

STATUS: APPROVED

## Decision

State 13 owner-authorized substitute evidence packaging approves `vb-kyyf` for the next controller state, subject to the disclosed provenance waiver.

## Basis

- State 11 formal verification is approved: `.beads/vb-kyyf/formal-verification-report.md:3`.
- State 11 machine gate is approved: `.beads/vb-kyyf/machine-gate-report.md:3`.
- State 12 black-hat review is approved: `.beads/vb-kyyf/black-hat-review.md:3`, with final verdict at `.beads/vb-kyyf/black-hat-review.md:43-45`.
- PO-001..PO-009 passed with command evidence in `.beads/vb-kyyf/formal-verification-report.md:30-38` and `.beads/vb-kyyf/verification-ledger.jsonl:1-9`.
- PO-010 is `DEFERRED_GLOBAL`, not bead-local rejection: `.beads/vb-kyyf/formal-verification-report.md:39`, `.beads/vb-kyyf/formal-verification-report.md:49-53`, `.beads/vb-kyyf/machine-gate-report.md:21-23`, and `.beads/vb-kyyf/verification-ledger.jsonl:10`.
- Required evidence files are present and mapped: `.evidence/vb-kyyf/bdd-cross-run-determinism.md`, `.evidence/vb-kyyf/storage-replay-resume.md`, `.evidence/vb-kyyf/non-replay-safe-actions.md`, `.evidence/vb-kyyf/recovery-bdd-errors.md`, `.evidence/vb-kyyf/generated-ir-parity.md`, `.evidence/vb-kyyf/generated-subset-fail-closed.md`, and `.evidence/vb-kyyf/acceptance-catalog-traceability.md`.
- Provenance is explicitly waived and disclosed: `.beads/vb-kyyf/state13-provenance-waiver.md` and `.beads/vb-kyyf/assurance-bundle.md`.

## JSONL Check

`jq -c .` was run in `/home/lewis/src/bd-vb-kyyf-bdd` against `.beads/vb-kyyf/traceability-matrix.jsonl` and `.beads/vb-kyyf/verification-ledger.jsonl`; both exited 0.

## Non-Claims

This decision does not claim execution by the missing `evidence-packaging` agent. It approves only the owner-authorized substitute State 13 package with explicit provenance disclosure.
