# Proof Evidence: vb-qi37.2.4

## Summary
- TLA+ bounded admission model: PASS after proof-artifact repairs.
- Verus bounded budget composition proofs: PASS after proof-artifact repair.
- Workspace proof rollup: BLOCKED_TOOLING before proof execution.
- Runtime/test-code obligations: BLOCKED_SCOPE for this state because production runtime and test edits were explicitly forbidden.

## TLA+ Evidence

Command:
```bash
tlc -config specs/tla/BoundedAdmission.cfg specs/tla/BoundedAdmission.tla
```

Final result:
```text
Model checking completed. No error has been found.
108977 states generated, 9762 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 9.
```

Checked invariants:
- `NoRunAdmittedWithoutReservation`.
- `ShardCapacityBounded`.
- `NoRunAdmittedWithoutVerifiedBudget`.
- `AdmittedResourcesArePositive`.

Obligations covered:
- `TLA-ADM-001`.
- `TLA-ADM-002`.

## Verus Evidence

Command:
```bash
verus verification/verus/budget_bounded.rs
```

Final result:
```text
verification results:: 15 verified, 0 errors
```

Proof lemmas added or repaired:
- `proof_sequential_checked_compose_monotone` for `VERUS-BUD-001`.
- `proof_nested_finite_repeat_cost` for finite collect/reduce/repeat multiplication under bounded factors.
- `proof_unknown_factor_rejects` for reject-on-unknown/invalid factors.
- `proof_nested_overflow_rejects` for multiplication overflow rejection.
- `proof_branch_max_conservative` for conservative branch maximum.
- `proof_together_fanout_bounded` and `proof_together_fanout_over_limit_rejects` for together fanout policy bounds.
- `proof_aggregate_refines_verified_whole` for whole-to-aggregate direct refinement.
- `proof_diagnostic_projection_total` for proof-visible diagnostic mandatory fields.

Obligations covered:
- `VERUS-BUD-001`.
- `VERUS-BUD-002`.
- `VERUS-BUD-003`.
- `VERUS-AGG-001`.
- `VERUS-DIAG-001` at abstract projection level.

## Rollup Gate Evidence

Command:
```bash
moon run :verify-proof
```

Result:
```text
scripts/rust-verification-gauntlet.sh: line 3: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 4: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 5: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 6: //!: No such file or directory
scripts/rust-verification-gauntlet.sh: line 7: syntax error near unexpected token `newline'
scripts/rust-verification-gauntlet.sh: line 7: `//! Usage: scripts/rust-verification-gauntlet.sh <mode>'
Error: task_runner::run_failed
Process bash failed: exit code 2
```

Classification:
- `GATE-BUD-001`: `BLOCKED_TOOLING`.
- Owner state: 12.
- Rerun from: 12.

## Blocked Scope Obligations
- `KANI-BUD-001`: planned target is `crates/vb_core/src/budget.rs`; adding Kani harness integration would edit production/test code and is forbidden in this request. Owner state: 7. Rerun from: 7.
- `PROP-BUD-001`: requires generated runtime/property tests and possibly generators; test-code edits are forbidden in this request. Owner state: 7. Rerun from: 7.
- `PROP-DIAG-001`: requires observable diagnostic property tests; test-code edits are forbidden in this request. Owner state: 7. Rerun from: 7.
- `GATE-BUD-002`: not run because deep lane depends on later proptest/fuzz/Miri/mutation artifacts. Owner state: 12. Rerun from: 12.
- `GATE-BUD-003`: not run in this proof-writer state after proof rollup tooling blocked. Owner state: 12. Rerun from: 12.

## Assumptions Recorded
- TLA+ model uses finite `RunId`/`ShardId` constants and bounded resource reservations from `BoundedAdmission.cfg`.
- TLA+ verified budget state abstracts `WholeWorkflowBudget` plus aggregate policy validation; Rust arithmetic remains owned by Verus/Kani/proptest lanes.
- Verus file is a proof-only abstraction of checked arithmetic and composition, not an executable import of `vb_core`.
- Diagnostic totality proof covers mandatory proof-visible fields; runtime diagnostic parity must still be proven by later property tests.

## State 5 Repair Attempt 2: PR-004 Mapping Gap
- Mapping repair: `VERUS-AGG-001` and `VERUS-DIAG-001` are now declared as executable required Verus rows in `proof-obligations.jsonl` and referenced from `traceability-matrix.jsonl`.
- `VERUS-AGG-001` remains scoped to `proof_aggregate_refines_verified_whole` and covers abstract aggregate-from-verified-whole refinement only.
- `VERUS-DIAG-001` remains scoped to `proof_diagnostic_projection_total` and does not waive `PROP-DIAG-001` runtime diagnostic parity.
- No production runtime/test code edited.
