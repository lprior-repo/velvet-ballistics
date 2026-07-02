# Final Evidence Decision: vb-core-yaml-e2e-chain

bead_id: vb-core-yaml-e2e-chain
state: 14 (evidence-packaging)
decision_date: 2026-05-16

## Decision

**STATUS: APPROVED**

## Summary

All required artifacts exist and are non-empty:
- delivery-scope.jsonl: OK
- contract.md: OK
- traceability-matrix.jsonl: OK
- proof-review.md: APPROVED
- test-plan-review.md: APPROVED
- formal-verification-report.md: APPROVED
- verification-ledger.jsonl: OK
- black-hat-review.md: APPROVED
- machine-gate-report.md: APPROVED
- regression-diff.md: APPROVED

All JSONL artifacts are valid (jq -c . parsed successfully).

All 4 review artifacts have STATUS: APPROVED.

## Obligation Status

| Category | Count |
|---|---|
| PASS | 18 |
| FAIL_LOCAL | 3 (production code, not verification) |
| DEFERRED_GLOBAL | 2 (pre-existing environment) |

## Waiver Table

| Item | Reason | Owner | Compensating Evidence |
|---|---|---|---|
| STATIC-BOUNDARY-009 | fuzz clippy | State 8 | Clippy PASS on production |
| STRICT-YAML-012 | digest assertion | State 10 | Kani + 35 tests PASS |
| ERR-STRICT-013 | Same | State 10 | Same |
| MIRI-CODEC-024 | rust-src absent | Tooling | Kani + 983 + 1460 tests PASS |
| GATE-RELEASE-025 | jj workspace | Environment | 18 obligations PASS |

## Evidence Quality

- All 18 PASS obligations have exact command evidence.
- All test gates verified in active execution context.
- No hallucinated evidence.
- No deleted tests.
- No contract parity violations.
- 3 FAIL_LOCAL are production code issues owned by States 8 and 10.
- 2 DEFERRED_GLOBAL are pre-existing environment debt.

## Next Action

State 14 (evidence-packaging) COMPLETE. Advance to State 15 (landing): jj push + bd close + git push.
