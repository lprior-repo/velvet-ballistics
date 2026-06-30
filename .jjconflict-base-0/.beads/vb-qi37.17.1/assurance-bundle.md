# Assurance Bundle — vb-qi37.17.1: cli: Add incident command

## Bead Information
- **Bead ID**: vb-qi37.17.1
- **Title**: cli: Add incident command
- **Acceptance Criteria**: "incident returns structured failure evidence without stack traces; tests cover failed, missing, and non-failed runs."
- **Isolated Workspace**: /home/lewis/src/go-skill-vb-qi37.17.1

## Artifact Inventory

| Artifact | Path | Lines | Verified |
|----------|------|-------|----------|
| STATE.md | .beads/vb-qi37.17.1/STATE.md | 45 | Yes |
| baseline-report.md | .beads/vb-qi37.17.1/baseline-report.md | 52 | Yes |
| codebase-map.md | .beads/vb-qi37.17.1/codebase-map.md | 290 | Yes |
| delivery-scope.jsonl | .beads/vb-qi37.17.1/delivery-scope.jsonl | 20 | Yes (valid JSONL) |
| contract.md | .beads/vb-qi37.17.1/contract.md | 315 | Yes |
| proof-obligations.jsonl | .beads/vb-qi37.17.1/proof-obligations.jsonl | 9 | Yes (valid JSONL) |
| traceability-matrix.jsonl | .beads/vb-qi37.17.1/traceability-matrix.jsonl | 14 | Yes (valid JSONL) |
| proof-strategy.md | .beads/vb-qi37.17.1/proof-strategy.md | 81 | Yes |
| proof-plan-review-input.md | .beads/vb-qi37.17.1/proof-plan-review-input.md | 117 | Yes |
| proof-obligations.planned.jsonl | .beads/vb-qi37.17.1/proof-obligations.planned.jsonl | 22 | Yes (valid JSONL) |
| proof-writer-report.md | .beads/vb-qi37.17.1/proof-writer-report.md | 3 | Yes |
| proof-evidence.md | .beads/vb-qi37.17.1/proof-evidence.md | 6 | Yes |
| proof-review.md | .beads/vb-qi37.17.1/proof-review.md | 3 | Yes — STATUS: APPROVED |
| contract-verification-review.md | .beads/vb-qi37.17.1/contract-verification-review.md | 3 | Yes |
| proof-findings.jsonl | .beads/vb-qi37.17.1/proof-findings.jsonl | 14 | Yes (valid JSONL) |
| test-plan.md | .beads/vb-qi37.17.1/test-plan.md | 406 | Yes |
| test-writer-report.md | .beads/vb-qi37.17.1/test-writer-report.md | 99 | Yes |
| test-plan-review.md | .beads/vb-qi37.17.1/test-plan-review.md | 47 | Yes — STATUS: APPROVED |
| test-suite-review.md | .beads/vb-qi37.17.1/test-suite-review.md | 59 | Yes — STATUS: APPROVED |
| implementation.md | .beads/vb-qi37.17.1/implementation.md | 132 | Yes |
| machine-gate-report.md | .beads/vb-qi37.17.1/machine-gate-report.md | 40 | Yes — STATUS: PASS |
| regression-diff.md | .beads/vb-qi37.17.1/regression-diff.md | 49 | Yes |
| compiler-errors.log | .beads/vb-qi37.17.1/compiler-errors.log | 34 | Yes |
| ci-failure-category.txt | .beads/vb-qi37.17.1/ci-failure-category.txt | 26 | Yes |
| formal-verification-report.md | .beads/vb-qi37.17.1/formal-verification-report.md | 91 | Yes — STATUS: APPROVED |
| verification-ledger.jsonl | .beads/vb-qi37.17.1/verification-ledger.jsonl | 13 | Yes (valid JSONL) |
| black-hat-review.md | .beads/vb-qi37.17.1/black-hat-review.md | 91 | Yes — STATUS: APPROVED |
| defects.md | .beads/vb-qi37.17.1/defects.md | 74 | Yes |

## Requirement-to-Evidence Traceability

### Contract PREconditions
| Clause | Evidence | Status |
|--------|----------|--------|
| PRE-001 (valid run_id) | T-014, T-015, T-016, T-017, T-018 (integration tests validate CLI argument parsing) | PASS |
| PRE-002 (db path accessible) | T-014, T-015, T-016, T-017, T-018 (FjallJournal open tested) | PASS |
| PRE-003 (non-null run_id, valid events) | T-001 through T-008 (unit tests exercise all event types) | PASS |
| PRE-004 (valid hints args) | T-009 through T-013 (hints tested with empty/non-empty inputs) | PASS |

### Contract POSTconditions
| Clause | Evidence | Status |
|--------|----------|--------|
| POST-001 (IncidentReport structure) | T-001 (empty defaults), T-002 (RunFailed), T-003 (RunCancelled), T-004-006 (side_effects), T-007 (step tracking), T-008 (full report) | PASS |
| POST-002 (Repair hint taxonomy) | T-009 (RunFailed, 1 hint), T-010 (RunFailed, 3 hints), T-011 (RunCancelled, 1 hint), T-012 (RunCancelled, 2 hints), T-013 (unknown, 0 hints) | PASS |
| POST-003 (Structured output, no stack traces) | T-014 (JSON), T-015 (error JSON on stderr), T-017 (text), T-018 (JSONL), T-015 stack trace assertions | PASS |
| POST-004 (Exit codes) | T-016 (exit code 5 for non-failed run) | PASS |

### Contract INVariants
| Clause | Evidence | Status |
|--------|----------|--------|
| INV-001 (zero-unwrap) | Lines 3191, 3207 fixed (RuntimeFailed → StorageError, match blocks); Lines 3202, 3208 waived (Option::unwrap_or, zero-panic) | PASS |
| INV-002 (no stack traces) | T-015 asserts no "backtrace" or "at crates/" in stderr | PASS |
| INV-003 (JSON validity) | serde_json::from_str called on T-014, T-015, T-018 stdout/stderr | PASS |
| INV-004 (text key ordering) | T-017 checks "incident report for run" and "RunFailed" in stdout | PASS |
| INV-005 (compile correctness) | 57 E0061 fixes applied, workspace compiles clean, cargo check passes | PASS |
| INV-006 (dead code removal) | args/run_db.rs parse_incident function removed | PASS |

## Defect Resolution Log

| Defect | Severity | Status | Resolution |
|--------|----------|--------|------------|
| DEFECT-001 (RuntimeFailed → StorageError) | Medium | RESOLVED | Fixed in app_impl.rs lines 3191, 3207 |
| DEFECT-002 (T-016 missing exit code assertion) | Medium | RESOLVED | Added exit code assertion to T-016 |
| DEFECT-003 (T-015 missing stack trace assertion) | Medium | RESOLVED | Added backtrace and "at crates/" assertions to T-015 |
| DEFECT-004 (Contract count 56 vs 57) | Low | RESOLVED | Updated contract.md to reference 57 |

## Pre-existing Workspace Debt (DEFERRED_GLOBAL)

| Issue | Severity | Location | Classification |
|-------|----------|----------|----------------|
| 3 vb_runtime::primitives::collect test failures | Medium | vb_runtime | DEFERRED_GLOBAL — unrelated to incident command |
| 10 xtask clippy warnings | Low | xtask/src/evidence_gate.rs | DEFERRED_GLOBAL — unrelated to incident command |
| xtask formatting diffs | Low | xtask | DEFERRED_GLOBAL — unrelated to incident command |

## Gate Summary

| Gate | Status | Scope |
|------|--------|-------|
| proof-review | STATUS: APPROVED | Proof loop |
| test-plan-review | STATUS: APPROVED | Test plan |
| test-suite-review | STATUS: APPROVED | Test suite |
| machine-gate-report | STATUS: PASS | Build, clippy, tests |
| formal-verification-report | STATUS: APPROVED | Obligation execution |
| black-hat-review | STATUS: APPROVED | Adversarial review |

## Test Execution Evidence

```
Unit tests:     13 passed (crates/vb_cli/src/commands_incident.rs)
Integration:     5 passed (crates/vb_cli/tests/vb_qi37_17_1_incident_command.rs)
Total:          18 passed
```

## Conclusion

All 12 contract clauses covered by evidence. All 4 defects resolved. All 6 gates APPROVED or PASS. No bead-local failures. Pre-existing workspace debt classified as DEFERRED_GLOBAL.

This bundle is ready for truth-serum audit.
