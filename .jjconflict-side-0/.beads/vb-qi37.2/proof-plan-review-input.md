# Proof Plan Review Input: vb-qi37.2 State 4 Attempt 3

## Review Request

Review refreshed State 4 proof planning after repaired State 3. The plan writes no code or proof artifacts and does not claim any pass results.

## Mandatory Context

- State 6 rejected prior proof package because required non-TLA rows were not executed, TLA deadlock checking was disabled by `CHECK_DEADLOCK FALSE`, and TLA certificate/reservation claims lacked Rust refinement evidence.
- Contract-verification rejected prior planning because Verus/Kani/parity rows used placeholders and ValueStore cap lacked Verus-first coverage.
- Repaired State 3 now names exact Verus ValueStore proof surface, exact Kani aggregate/ValueStore harness commands for State 5, and exact parity command plus reviewer source classification.

## Files To Review

- `.beads/vb-qi37.2/proof-strategy.md`
- `.beads/vb-qi37.2/proof-obligations.planned.jsonl`
- `.beads/vb-qi37.2/contract.md`
- `.beads/vb-qi37.2/verification-layers.md`
- `.beads/vb-qi37.2/proof-obligations.jsonl`
- `.beads/vb-qi37.2/traceability-matrix.jsonl`
- `.beads/vb-qi37.2/proof-review.md`
- `.beads/vb-qi37.2/proof-findings.jsonl`
- `.beads/vb-qi37.2/proof-repair-guide.md`
- `.beads/vb-qi37.2/contract-verification-review.md`

## Discovery Commands Run

```bash
pwd -P
test -s ".beads/vb-qi37.2/contract.md"
test -s ".beads/vb-qi37.2/traceability-matrix.jsonl"
test -s ".beads/vb-qi37.2/delivery-scope.jsonl"
rtk grep -n "unsafe|unwrap\(|expect\(|panic!|todo!|unimplemented!|assert!|spawn|tokio|Mutex|RwLock|Atomic|serialize|deserialize|state|transition|lease|queue|retry|cancel" "crates/vb_core/src/budget.rs" "crates/vb_runtime/src/admission.rs" "crates/vb_core/src/workflow/mod.rs" "crates/vb_core/src/validation.rs" "crates/vb_core/src/compiled_workflow.rs" "crates/vb_core/src/value_store.rs" "crates/vb_runtime/src/shard/lifecycle/chunk_001.rs" "crates/vb_core/src/engine/signals.rs" "crates/vb_core/src/engine/run_loop.rs" "crates/vb_core/src/limits.rs" "verification/verus/resource_budget.rs" "verification/verus/budget_monotonic.rs" "verification/verus/budget_bounded.rs" "verification/verus/step_budget.rs" "verification/verus/value_store_invariant.rs" "verification/tla/WorkflowBoundedAdmission.tla" "verification/tla/WorkflowBoundedAdmission.cfg"
rtk grep -n "requires|ensures|proof fn|invariant|kani::|loom::|proptest!|fuzz_target|Flux|TLA|Miri|unsafe" "crates/vb_core/src/budget.rs" "crates/vb_runtime/src/admission.rs" "crates/vb_core/src/workflow/mod.rs" "crates/vb_core/src/validation.rs" "crates/vb_core/src/compiled_workflow.rs" "crates/vb_core/src/value_store.rs" "crates/vb_runtime/src/shard/lifecycle/chunk_001.rs" "crates/vb_core/src/engine/signals.rs" "crates/vb_core/src/engine/run_loop.rs" "crates/vb_core/src/limits.rs" "verification/verus/resource_budget.rs" "verification/verus/budget_monotonic.rs" "verification/verus/budget_bounded.rs" "verification/verus/step_budget.rs" "verification/verus/value_store_invariant.rs" "verification/tla/WorkflowBoundedAdmission.tla" "verification/tla/WorkflowBoundedAdmission.cfg"
```

All discovery commands exited 0. No `DISCOVERY_BLOCKED` row is required.

## Reviewer Checks Requested

- Confirm every required contract clause in `traceability-matrix.jsonl` maps to a planned obligation row.
- Confirm State 6 TLA deadlock/refinement rejection is preserved as required State 5 evidence, not waived.
- Confirm ValueStore cap now has Verus, Kani, Miri, and test/proptest coverage rather than Miri alone.
- Confirm Kani aggregate/ValueStore commands are acceptable exact State 5 commands even if harness creation/repair is required before execution.
- Confirm not-applicable theorem and Flux rows are legitimate and do not waive mandatory Verus/TLA/Kani evidence.

## Expected State 5 Hand-Off

State 5 should write or repair proof artifacts only, run the exact commands where possible, and record raw output. Any unavailable harness/tool must become a precise blocker with attempted command, raw failure, owner, expiry, and compensating evidence.
