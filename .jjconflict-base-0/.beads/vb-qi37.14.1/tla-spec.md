# TLA+ Temporal Model Plan: `run --step`

## Non-applicability Rationale

The `run --step` feature is **not a temporal/state-over-time system** and does not require a TLA+ model.

**Justification:**

1. **Single-shot execution**: `run --step` executes exactly one node transition. There is no loop, no iteration, no continuation, and no state machine that evolves over multiple steps within the scope of this command.

2. **No concurrency**: The CLI invocation is single-process and single-threaded. There are no concurrent actors, no message passing, no race conditions, and no need for fairness or liveness properties.

3. **No retry/claim/lease logic**: The single step either succeeds, fails, or suspends. There is no retry logic, no claim mechanism, no lease expiry, and no distributed coordination.

4. **No distributed coordination**: No multi-node, no replication, no eventual consistency concerns, no coordinator/participant protocols.

5. **No protocol**: The step execution is a pure function from `(CompiledWorkflow, RunFrame, SlotValue[]) → (EngineSignal, RunFrame)`. There is no protocol state to model — the "before" and "after" frame states are directly observable as delta output.

6. **No liveness property**: The command either completes (returns a signal) or errors. There is no "eventually" property to verify — the step executes exactly once and terminates.

7. **No deadlock concern**: With no loop and no blocking wait within the CLI command itself (suspension signals like `AwaitingAction` are returned to the caller, not handled internally), there is no deadlock possibility within the scope of `run --step`.

8. **TLA+ would be a single-state dot**: The minimal TLA+ specification for this feature would have:
   - Variables: `pc ∈ {0}`, `frame ∈ Frame`, `signal ∈ Signal`
   - Init: `pc = 0 ∧ frame = empty ∧ signal = None`
   - Next: `pc' = pc ∧ frame' = frame ∧ signal' = step_once(...)`
   - No invariants to check (trivial)
   - No temporal properties (no "always", "eventually", "until")

   This provides zero verification value and would be rejected by the plan-shredder as a meaningless model.

## TLA+-Owned Clauses

None.

## What IS being verified (non-TLA+)

- **Rust unit tests**: `step_once_*` tests in `crates/vb_core/src/engine/step.rs` cover all `EngineSignal` variants and `CompiledNodeKind` dispatch paths.
- **Kani**: Bounded model check over `step_once` with arbitrary `CompiledWorkflow`, `RunFrame`, `ValueStore` inputs — verifies no panic, no UB, and correct signal mapping.
- **Verus**: `INV-002` (step-state mapping invariant) and `INV-004` (PC bounds) are proven in Verus.
- **Integration tests**: CLI integration tests in `crates/vb_cli/tests/cli_integration.rs` verify end-to-end behavior.

## Waiver

| Clause | Owner | Reason | Expiry | Compensating Evidence |
|--------|-------|--------|--------|-----------------------|
| TLA+ temporal model | vb-qi37.14.1 | Feature is a single-shot pure function with no temporal behavior, loop, concurrency, or protocol | N/A (permanent non-applicability) | Unit tests + Kani + Verus + CLI integration tests |

## Alternative Considered: Delta State TLA+

If future requirements extend `run --step` to support multi-step delta reporting (e.g., "run steps 0 through N and report cumulative deltas"), a TLA+ model would become relevant. At that point, the model would need:

- Variables: `pc ∈ 0..MAX_STEP`, `slots ∈ [0..MAX_SLOT] → Value ⊎ None`, `taint ∈ [0..MAX_SLOT] → Taint`, `states ∈ [0..MAX_STEP] → StepState`
- Init: Initialize all slots to None, taint to Clean, states to Pending, pc to entry
- Next: `step_once(pc)` — single-step transition relation
- Invariant: `ValidSignalStateMapping` — the mapping from signal to step state enforced as a state predicate
- Bounded model: `pc ∈ 0..16` (u16), `slot ∈ 0..256` (u16)

But this is out of scope for the current bead.
