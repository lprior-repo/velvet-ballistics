# Proof Strategy: vb-5m8w Step Budget Suspension

## Scope
- Bead: `vb-5m8w` only.
- State: 4 proof-plan ledger repair, attempt 3.
- Workspace: `/home/lewis/src/go-skill-vb-5m8w` only.
- No production code, proof model code, test code, dependencies, or CI config were edited.

## Rejection Inputs Consumed
- `.beads/vb-5m8w/contract-verification-review.md`: `STATUS: REJECTED`.
- `.beads/vb-5m8w/proof-repair-guide.md`: `STATUS: REJECTED`.

## Repair Performed
- Regenerated `.beads/vb-5m8w/proof-obligations.jsonl` and `.beads/vb-5m8w/proof-obligations.planned.jsonl` so every row has:
  - `id`, `contract_clause`, `target`, `claim`, `layer`, `checker`, `command`, `evidence`, `expected_evidence`, `risk`, `scope`, `required`, `mode`, `owner_state`, `rerun_from`, and `status:"planned"`.
- Added required TLA metadata to every TLA row:
  - `tla_module`, `model`, `config`, `variables`, `actions`, `invariants`, `temporal_properties`, `fairness`, `state_constraints`, and `refinement`.
- Repaired Verus waiver row:
  - `status:"planned"`, `mode:"waived"`, `required:false`.
  - Waiver includes `clause_ids`, `layer_waived:"verus"`, `owner`, `expiry`, `followup`, `limitation`, and compensating evidence marked as downstream-required.
  - `moon ci` is not cited as completed compensation.
- Kept non-applicable lanes as ledger rows with `status:"planned"` and `mode:"not_applicable"`.

## Risk Classification
- Temporal/state-machine: required; budget exhaustion is graceful scheduler suspension/reschedule, not terminal completion/failure.
- Bounded u64 arithmetic: required; State 5 must repair TLA to executable `MAX_U64 = 18446744073709551615` representative semantics.
- Rust-local invariant: applicable, but Verus is waived because current artifacts are detached/vacuum for this contract.
- Bounded model checking: required; Kani must use package/lib commands and bind to actual production zero-budget behavior or receive an explicit structural waiver.
- Concrete tests/proptest/CI: required downstream regression/compensation gates.
- Concurrency/unsafe/parser/dependency/performance/release lanes: not applicable unless downstream edits introduce a trigger.

## State 5 Repair Plan
1. TLA exact bounded arithmetic repair:
   - Define executable `MAX_U64 = 18446744073709551615` in `verification/tla/StepBudgetSuspension.tla`.
   - Model representative valid u64 values including `0`, `1`, `MAX_STEP_BUDGET - 1`, `MAX_STEP_BUDGET`, and `MAX_U64`.
   - Model above-u64/overflow as explicit sink/error behavior, not as unbounded `Nat` arithmetic.
   - Model zero-underflow as explicit sink/error behavior.
   - Keep reachable `RunnableState` at `MAX_STEP_BUDGET` and prove decrement to `MAX_STEP_BUDGET - 1`.
2. Kani production binding repair:
   - Repair `kani_step_budget_try_take_arbitrary` so it calls actual production zero-budget behavior (`StepBudget::new(0)`/`try_take` and `run_until_blocked`/`drive_deterministic`, or a production pure transition used by that path).
   - Assert actual PC/frame/run-state/evidence preservation under generated bounded inputs.
   - If construction is infeasible, replace the row with an explicit Kani structural waiver containing owner, expiry, limitation, and compensation; do not claim Kani proves frame preservation.
3. Verus:
   - Keep waived unless State 5 binds specs to actual executable Rust functions; no detached Verus PASS may satisfy this contract.

## Commands Planned
- TLA: `tla2tools verification/tla/StepBudgetSuspension.tla -config verification/tla/StepBudgetSuspension.cfg`.
- Kani boundary: package/lib harness commands in `KANI-BUDGET-001` / `PO-008`.
- Kani structural: `cargo kani -p vb_core --lib --harness kani_step_budget_try_take_arbitrary --no-assertion-reach-checks`.
- Scoped tests: `cargo +nightly nextest run -p vb_core -p vb_runtime -E 'test(/budget|Budget|StepBudgetExhausted|AwaitingAction|AwaitingWait|AwaitingAsk|evidence/)'`.
- Property tests: `PROPTEST_CASES=1024 cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture`.
- CI: `moon ci` downstream; not claimed run here.

## Routing
- `current_state=4`
- `next_state=5`
- `status=READY_FOR_PROOF_REPAIR`

No verifier success for future repaired artifacts is claimed by this planning state.
