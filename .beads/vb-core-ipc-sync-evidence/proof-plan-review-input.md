# Proof Plan Review Input: vb-core-ipc-sync-evidence

updated_at: 2026-05-15T20:48:34Z
state: 4
attempt: 3-of-7
status: READY_FOR_PROOF_PLAN_REVIEW

## Reviewer Focus

- Confirm repaired State 3 coverage is preserved in `.beads/vb-core-ipc-sync-evidence/proof-obligations.planned.jsonl`.
- Confirm no invalidated State 4/5/6 pass claims remain in the refreshed plan.
- Confirm true TLA+ liveness/fairness is not claimed as passed; `BLOCK-TLA-LIVENESS` remains required.
- Confirm pure Verus rows are paired with production-refinement blocker rows for CON-IPC-001 through CON-IPC-005.
- Confirm Loom, slow-client, static-scan, and `moon ci` gaps are explicit planned obligations, not omitted.

## Rejection Artifacts Incorporated

- `proof-review.md`: required Loom compile blocker, slow-client zero-test blocker, static-scan partial blocker, TLA+ liveness downgrade, Verus production-linkage blocker, and deferred `moon ci` gate.
- `proof-findings.jsonl`: six findings carried into planned rows.
- `proof-repair-guide.md`: rerun targets retained as exact commands where executable.
- `contract-verification-review.md`: repaired missing Verus CON-IPC-003..005 and TLA+ CON-IPC-007 coverage reflected in planned rows.

## Discovery Evidence

- Workspace check: `pwd -P` matched `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-ipc-sync-evidence`.
- Required input artifacts existed: `contract.md`, `traceability-matrix.jsonl`, and `delivery-scope.jsonl`.
- Risk-pattern scan over scoped crates returned `8366 matches in 240 files`.
- Verifier-pattern scan over scoped crates plus `verification` returned `988 matches in 188 files`.
- Blocked discovery commands: none.

## Planned Status Policy

- `planned`: exact command exists and is intended for State 5, State 8, State 10, or State 11 evidence. A `planned` row is not a pass result.
- `blocked_tooling`: no current executable command can discharge the obligation without missing model/test/source/tooling work.
- `waived`: verifier not applicable with owner, reason, expiry, and compensating evidence.
- `not_applicable`: lane is not risk-triggered for this bead scope.

## Files For Review

- `.beads/vb-core-ipc-sync-evidence/proof-strategy.md`
- `.beads/vb-core-ipc-sync-evidence/proof-plan-review-input.md`
- `.beads/vb-core-ipc-sync-evidence/proof-obligations.planned.jsonl`
