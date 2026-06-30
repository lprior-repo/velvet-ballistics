# Proof Plan Review Input: vb-5m8w Attempt 3

## Decision Requested
Review the repaired State 4 proof plan after contract-verification rejection of schema defects, TLA metadata gaps, Verus waiver metadata, and detached Kani planning.

## Repaired Items
1. Both ledgers now include the mandatory schema fields on every row: `id`, `contract_clause`, `target`, `claim`, `layer`, `checker`, `command`, `evidence`, `expected_evidence`, `risk`, `scope`, `required`, `mode`, `owner_state`, `rerun_from`, and `status:"planned"`.
2. Every TLA row includes `tla_module`, `model`, `config`, `variables`, `actions`, `invariants`, `temporal_properties`, `fairness`, `state_constraints`, and `refinement`.
3. Verus is explicitly waived, not passed: `required:false`, `mode:"waived"`, `status:"planned"`, with `clause_ids`, `layer_waived:"verus"`, owner, expiry/followup, limitation, and downstream-required compensation.
4. `moon ci` is a downstream CI obligation only; it is not cited as completed Verus waiver compensation.
5. State 5 TLA repair is planned for executable `MAX_U64` representative arithmetic and explicit above-u64/overflow/zero-underflow behavior.
6. State 5 Kani repair is planned to bind actual zero-budget production behavior or replace the structural Kani row with an explicit waiver.

## Reviewer Attack Points
- Reject if any row lacks the mandatory schema fields or has `status` other than `planned`.
- Reject if any TLA row lacks the required TLA metadata fields.
- Reject if TLA can still pass with unbounded `Nat`, documentation-only `MAX_U64`, no `MAX_U64 = 18446744073709551615`, no above-u64/overflow sink, or no zero-underflow sink.
- Reject if `MAX_STEP_BUDGET` is unreachable in `RunnableState` or cannot decrement to `MAX_STEP_BUDGET - 1`.
- Reject if Verus PASS is claimed without binding to `vb_core` executable implementation.
- Reject if the Verus waiver lacks clause IDs, waived layer, owner, expiry/followup, limitation, or honest compensation status.
- Reject if structural Kani still uses immutable shadow structs or fixed dummy `WorkflowParts`/`RunFrame` rather than actual production zero-budget behavior.

## Current Waivers / Not Applicable Rows
- Verus: waived because existing files are detached/vacuum proofs for this contract. Compensation is downstream TLA/Kani/scoped-test/property evidence, not completed CI.
- Lean/Aeneas/Hax: not applicable; no theorem-kernel owned clause.
- Loom/Miri/Flux/fuzz/dependency/performance/release: not applicable; no risk trigger in this bead scope.

## Status
- `current_state=4`
- `next_state=5`
- `status=READY_FOR_PROOF_REPAIR`

This is a plan repair only. No repaired proof/model/test success is claimed.
