# Contract Specification - vb-0253.5

## Context
- **Feature**: Align StepState contract across runtime and proofs
- **Domain terms**: StepState, State Machine, Valid Transition, Terminal States, Non-terminal States
- **Assumptions**: StepState is a finite state machine with 8 states
- **Open questions**: What specific misalignment exists between runtime usage and proof definitions?

## Preconditions
- PRE-001: Transition validation requires current state to be non-terminal

## Postconditions
- POST-001: is_valid_transition returns true for valid transitions
- POST-002: is_valid_transition returns false for invalid transitions
- POST-003: validate_transition returns Ok(()) for valid, Err(msg) for invalid
- POST-004: terminal_states returns exactly {Succeeded, Failed, Cancelled, Skipped}
- POST-005: non_terminal_states returns exactly {Pending, Running, Waiting, Asking}

## Invariants
- INV-001: StepState has exactly 8 variants
- INV-002: Valid transitions match the state machine definition
- INV-003: Terminal states are never followed by non-terminal states

## Error Taxonomy
- Error::InvalidTransition - when attempting invalid state transition

## Contract Signatures
- `fn is_valid_transition(from: StepState, to: StepState) -> bool`
- `fn validate_transition(from: StepState, to: StepState) -> Result<(), &'static str>`
- `fn next_states(from: StepState) -> Vec<StepState>`
- `fn terminal_states() -> Vec<StepState>`
- `fn non_terminal_states() -> Vec<StepState>`

## TLA+-Owned Clauses
- INV-002 -> TLA+ model for StepState transitions

## Verus-Owned Clauses
- INV-001: StepState enum has exactly 8 variants
- INV-002: Transition validity matches specification
- INV-003: Terminal/non-terminal classification is correct

## Non-goals
- Runtime-specific StepState usage in isolation
- Cross-workflow state coordination
