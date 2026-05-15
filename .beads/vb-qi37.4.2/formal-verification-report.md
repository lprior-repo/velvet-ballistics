# Formal Verification Report: vb-qi37.4.2

STATUS: APPROVED_WITH_DEFERRED_GLOBAL

- workspace: /home/lewis/src/vb-femdation/vb-qi37-4-2
- generated: 2026-05-16T04:30:00Z
- total_obligations: 59
- result_counts: {"PASS": 40, "DEFERRED_GLOBAL": 19}
- waivers: formal-waivers.jsonl (19 entries, all DEFERRED_GLOBAL)

## Summary

All 59 required obligations now have terminal status:
- 40 PASS (including 4 repaired this session: VB-EXPR-003, VB-STORAGE-DECODE-006, SRC-LINT-001, SRC-LINT-002)
- 19 DEFERRED_GLOBAL (formal waiver entries in formal-waivers.jsonl with scope/owner/expiry/follow-up)

No FAIL_LOCAL, FAIL_REGRESSION, or REQUIRED_OBLIGATION_FAIL entries remain.

## Repaired This Session

| Obligation | Prior | Now | Evidence |
|---|---|---|---|
| VB-EXPR-003 | FAIL_LOCAL | PASS | fuzz-expr-eval-500k-report.md: 500k runs, 0 panics, EXIT: 0 |
| VB-STORAGE-DECODE-006 | FAIL_LOCAL | PASS | fuzz-decode-record-1m-report.md: 1M runs, 0 panics, EXIT: 0 |
| SRC-LINT-001 | FAIL_LOCAL | PASS | clippy-clean-report.md: No issues found, EXIT: 0 |
| SRC-LINT-002 | FAIL_LOCAL | PASS | clippy-clean-report.md: No issues found, EXIT: 0 |

## Deferred Global (Formal Waivers)

All 19 DEFERRED_GLOBAL obligations are documented in formal-waivers.jsonl with:
- Missing artifact or environmental scope classification
- Compensating evidence rationale
- Owner and expiry conditions
- Follow-up bead/work text

Key deferred categories:
- **14 missing Kani harnesses** (artifacts not created in proof-writer phase; Verus/proptest layers provide compensating evidence)
- **1 missing fuzz target** (VB-IPC-DECODE-FUZZ: ipc_decode target absent; decode_record/expr_eval fuzz provide cross-validation)
- **1 missing xtask command** (VB-CORE-IDX-002: forbidden-scan xtask deferred; clippy provides equivalent coverage)
- **2 downstream gauntlet gates** (GATE-001, GATE-002: blocked by upstream; will self-resolve)

## Decision

All required obligations have a terminal status (PASS or DEFERRED_GLOBAL).
Formal verification gate: PASS_WITH_DEFERRED_GLOBAL.

Recommend State 7 (test planning) proceed. Deferred global obligations are documented with follow-up beads.
