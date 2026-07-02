# TLA+ Temporal Model Plan - vb-0253.5

## Boundary
- **Temporal/workflow behavior**: StepState is a finite state machine - TLA+ appropriate
- **Rust/core behavior excluded from TLA+**: Enum variant correctness via Verus
- **External systems abstracted**: None

## TLA+-Owned Clauses
- INV-002: State machine transitions match the state diagram

## Model Shape
- Module: StepState
- Variables: current_state (StepState)
- Init: current_state = Pending
- Next: current_state' in next_states(current_state)

## Properties
- Safety: No invalid transitions allowed
- Liveness: Eventually reach terminal state

## Evidence Command
- tlc -config specs/StepState.cfg specs/StepState.tla

## Waivers
- None
