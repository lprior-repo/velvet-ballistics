# Assurance Bundle — vb-vt2f

STATUS: APPROVED

## Scope And Provenance

- Bead: `vb-vt2f` only.
- State: 13 owner-authorized substitute evidence packaging only.
- Attempt: `owner-authorized-substitute-1`.
- Workdir: `/home/lewis/src/bd-vb-vt2f-bdd` only.
- Provenance: this bundle is produced by a direct substitute packaging subagent, not by the missing `evidence-packaging` specialist.
- Owner authorization: `User explicitly stated no evidence-package agent is required and instructed femdation to use subagents and resolve the provenance differences. Substitute packaging by a direct general subagent is authorized for this State 13 lane.` See `.beads/vb-vt2f/dispatch-state13-owner-waived-packaging.json:29-31`.
- Previous fallback issue: `.beads/vb-vt2f/truth-serum-report.md:149-156` rejected the earlier bundle because the required named `evidence-packaging` delegate was unavailable/fell back and the bundle did not disclose that provenance.
- Provenance repair: this bundle does not launder the previous fallback as specialist approval. It explicitly records owner-authorized substitute packaging and pairs it with `.beads/vb-vt2f/state13-provenance-waiver.md`.

## JSONL Validation

- `jq -c . .beads/vb-vt2f/traceability-matrix.jsonl` completed successfully in `/home/lewis/src/bd-vb-vt2f-bdd`.
- `jq -c . .beads/vb-vt2f/verification-ledger.jsonl` completed successfully in `/home/lewis/src/bd-vb-vt2f-bdd`.
- Traceability rows consumed: 32 (`.beads/vb-vt2f/traceability-matrix.jsonl:1-32`).
- Verification ledger rows consumed: 40 (`.beads/vb-vt2f/verification-ledger.jsonl:1-40`).

## Requirement Evidence Map

| Requirement group | Contract evidence | Trace/proof evidence | Test/command evidence | Review evidence | Status |
|---|---|---|---|---|---|
| Preconditions `PRE-001` through `PRE-005` | `.beads/vb-vt2f/contract.md:30-34` | `.beads/vb-vt2f/traceability-matrix.jsonl:1-5`; `.beads/vb-vt2f/verification-ledger.jsonl:1-5` | Direct API run `f215647f` and public-surface audit evidence in `.beads/vb-vt2f/test-review.md:29-60`; machine gate lines `.beads/vb-vt2f/machine-gate-report.md:14-16` | `.beads/vb-vt2f/test-review.md:3-4,21-27`; `.beads/vb-vt2f/contract-verification-review.md:43-50,54` | PASS |
| Postconditions `POST-001` through `POST-012` | `.beads/vb-vt2f/contract.md:38-49` | `.beads/vb-vt2f/traceability-matrix.jsonl:6-17`; `.beads/vb-vt2f/verification-ledger.jsonl:6-17` | Direct API run `f215647f` `14 passed`, catalog run `b996c7a3` `13 passed`, trace-eviction focused run `70fb5f9e` `1 passed`; `.beads/vb-vt2f/formal-verification-report.md:23-25` | `.beads/vb-vt2f/test-plan-review.md:11-18,28-36,48`; `.beads/vb-vt2f/test-suite-review.md:11-21,31-43,61` | PASS |
| Invariants `INV-001` through `INV-006` | `.beads/vb-vt2f/contract.md:53-58` | `.beads/vb-vt2f/traceability-matrix.jsonl:18-23`; `.beads/vb-vt2f/verification-ledger.jsonl:18-23` | `moon ci` raw `tool_e3c4e9cf8001AzrDsx9ke49onI`, `9016 tests run: 9016 passed`, `MOON_CI_EXIT=0`; `.beads/vb-vt2f/machine-gate-report.md:19` | `.beads/vb-vt2f/proof-review.md:57-74`; `.beads/vb-vt2f/black-hat-review.md:17-22` | PASS with approved Verus waiver caveat |
| Error taxonomy `ERR-001` through `ERR-006` | `.beads/vb-vt2f/contract.md:62-67` | `.beads/vb-vt2f/traceability-matrix.jsonl:24-29`; `.beads/vb-vt2f/verification-ledger.jsonl:24-29` | Direct API and catalog runs in `.beads/vb-vt2f/machine-gate-report.md:14-16`; focused stale ask run `70fb5f9e`; full BDD run `f215647f` | `.beads/vb-vt2f/contract-verification-review.md:45-50`; `.beads/vb-vt2f/black-hat-review.md:17-22` | PASS |
| Release gate and formal lanes | `.beads/vb-vt2f/contract.md:151-160` | `.beads/vb-vt2f/traceability-matrix.jsonl:30-32`; `.beads/vb-vt2f/verification-ledger.jsonl:34-39` | TLC lifecycle `3600 states generated, 1302 distinct`; TLC strict admission `2892 states generated, 1096 distinct`; Kani facade `0 of 489 failed`; Kani shard-lower `0 of 122 failed`; `moon ci` `9016 passed`; `.beads/vb-vt2f/formal-verification-report.md:28-32` | `.beads/vb-vt2f/proof-review.md:48-55`; `.beads/vb-vt2f/formal-verification-report.md:55-59`; `.beads/vb-vt2f/black-hat-review.md:3,36-38` | PASS |
| Waivers and trusted boundary | `.beads/vb-vt2f/contract.md:157-160` | `.beads/vb-vt2f/verification-ledger.jsonl:30-33,40` | No new command evidence claimed; waivers are review artifacts only | `.beads/vb-vt2f/proof-review.md:57-74`; `.beads/vb-vt2f/formal-verification-report.md:42-48`; `.beads/vb-vt2f/contract-verification-review.md:45-50` | WAIVED/APPROVED where applicable |

## Proof, Test, And Review Statuses

- Proof review: `STATUS: APPROVED` at `.beads/vb-vt2f/proof-review.md:72-74`.
- Contract verification review: `STATUS: APPROVED` at `.beads/vb-vt2f/contract-verification-review.md:3,52-54`.
- Test plan review: `STATUS: APPROVED` at `.beads/vb-vt2f/test-plan-review.md:3,48`.
- Test suite review: `STATUS: APPROVED` at `.beads/vb-vt2f/test-suite-review.md:3,61`.
- Test review/public surface audit: `STATUS: APPROVED` and `PUBLIC_SURFACE_AUDIT: PASS` at `.beads/vb-vt2f/test-review.md:3-4`.
- Machine gate: `STATUS: PASS` at `.beads/vb-vt2f/machine-gate-report.md:6`; commands and `moon ci` pass at lines 14-19.
- Formal verification: `STATUS: APPROVED` at `.beads/vb-vt2f/formal-verification-report.md:3`; all 40 obligations accounted with no fail/deferred debt at lines 34-40 and 55-57.
- Black-hat review: `STATUS: APPROVED` at `.beads/vb-vt2f/black-hat-review.md:3`; no blocking findings at lines 11-13; decision approved at lines 36-38.

## Ledger Summary

- PASS rows: 35 (`.beads/vb-vt2f/verification-ledger.jsonl:1-29,34-39`).
- WAIVED rows: 5 (`.beads/vb-vt2f/verification-ledger.jsonl:30-33,40`).
- FAIL rows: 0.
- DEFERRED_GLOBAL rows: 0.
- Formal report independently records `PASS: 35`, `WAIVED: 5`, `FAIL_LOCAL: 0`, `FAIL_REGRESSION: 0`, `DEFERRED_GLOBAL: 0` at `.beads/vb-vt2f/formal-verification-report.md:34-40`.

## Defect Closure

- `LETHAL-001` stale ask terminal trace eviction is closed by shard-owned terminal tombstones and trace-independent stale ask rejection. Implementation report records the repair at `.beads/vb-vt2f/implementation.md:419-463` and focused/full command evidence at `.beads/vb-vt2f/implementation.md:471-483`.
- State 11 and State 12 confirmed the repair with current evidence: `.beads/vb-vt2f/machine-gate-report.md:14-21`, `.beads/vb-vt2f/formal-verification-report.md:22-32,50-57`, and `.beads/vb-vt2f/black-hat-review.md:17-22,28-30`.

## Decision

- Missing-evidence blockers: none found in consumed artifacts.
- Provenance blocker from the prior truth-serum rejection is resolved by explicit owner-authorized substitute disclosure, not by claiming the missing `evidence-packaging` specialist ran.
- Final assurance status: APPROVED FOR DELIVERY under owner-authorized substitute State 13 packaging provenance.
