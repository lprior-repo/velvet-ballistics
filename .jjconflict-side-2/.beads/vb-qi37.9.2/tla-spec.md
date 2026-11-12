# TLA+ Temporal Model Plan — vb-qi37.9.2

## Boundary
- **Temporal/workflow behavior**: None — F64 bytecode evaluation is pure deterministic computation with no state over time.
- **Rust/core behavior excluded from TLA+**: F64 arithmetic ops (`eval_add_op`, `eval_sub_op`, `eval_mul_op`, `eval_div_op`, `eval_neg_op`, comparison ops) are pure functions; handled by Verus, Kani, and proptest.
- **External systems abstracted**: None — no I/O, network, DB, or FFI in the F64 bytecode eval path.
- **Non-applicability rationale**: F64 bytecode evaluation (`eval_expr_program`, `eval_binary_op`, `eval_unary_op`) processes a static bytecode program over a fixed-size stack. There is no temporal behavior, liveness requirement, fairness condition, deadlock risk, workflow, protocol, scheduler, retry logic, claim/lease, concurrent modification, or distributed coordination. The evaluation is a pure function: `(program, slots, constants, store) → Result<SlotValue, ExprError>`. TLA+ provides no verification value for this scope.

## TLA+-Owned Clauses
- **None** — No temporal or state-machine behavior in F64 bytecode semantics scope.

## Model Shape
- **Module/model path**: N/A
- **Variables**: N/A
- **Init action**: N/A
- **Next/actions**: N/A
- **State constraints**: N/A
- **Symmetry sets**: N/A
- **Bounded model limits**: N/A

## Properties
- **Safety invariants**: N/A
- **Liveness/eventuality**: N/A
- **Fairness assumptions**: N/A
- **Deadlock freedom**: N/A
- **Refinement to Rust/runtime behavior**: N/A

## Evidence Command
- N/A — TLA+ not applicable for this bead's scope

## Waivers
- **TLA+ waiver for all F64 bytecode semantics**: Owner = contract phase (State 3). Reason = pure deterministic Rust computation with no temporal/state-over-time behavior. Compensating evidence = Verus postconditions, Kani bounded model check, proptest cross-validation against IEEE 754 reference behavior, and integration tests.
