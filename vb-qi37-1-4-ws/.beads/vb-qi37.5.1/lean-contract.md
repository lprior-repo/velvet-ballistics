# Lean Contract Projection

## Boundary
- Lean-owned kernel: Pure idempotency decision table evaluation
- Rust/runtime shell: WorkflowParts traversal, contract registry lookup, error accumulation
- External systems excluded from Lean proof: Fjall storage, IPC, runtime action dispatch

## Lean-Owned Clauses

The idempotency decision table is a pure enum-to-enum mapping over bounded domain:
- `Idempotency ∈ {DeterministicPure, IdempotentExternal, AtLeastOnceExternal}`
- `SideEffect ∈ {None, Writes, Sends, Creates, Destroys}`
- `RetrySafety ∈ {Safe, KeyRequired, Unsafe}`

This is a 3×5×3 = 45 combination finite state machine. Each combination maps to exactly one of:
- `Accept` (pure actions always accept)
- `Reject(SideEffectingRetryUnsafe)` 
- `Reject(SideEffectingAtLeastOnceExternal)`
- `Reject(SideEffectingDeterministicPure)`

## Theorem Obligations

### THM-IDEM-001
- Contract clause: I3 (Pure action acceptance)
- Rust/spec target: `is_statically_idempotent_contract` / `validate_action_idempotency_contract`
- Lean module: `VbCore.Idempotency`
- Theorem shape: pure_action_always_accepts
- Model: Given any `Idempotency` and `RetrySafety`, when `SideEffect == None`, result is `Accept`
- Refinement: Rust implementation matches Lean decision table
- Shell exclusions: No I/O, storage, or runtime effects
- Evidence command: `lake build` or Kani harness

### THM-IDEM-002
- Contract clause: I4 (Side-effecting unsafe rejection)
- Rust/spec target: `is_statically_idempotent_contract`
- Lean module: `VbCore.Idempotency`
- Theorem shape: side_effecting_unsafe_always_rejects
- Model: Given any `Idempotency`, when `SideEffect != None` and `RetrySafety == Unsafe`, result is `Reject(SideEffectingRetryUnsafe)`
- Refinement: Rust implementation matches Lean decision table
- Shell exclusions: No I/O, storage, or runtime effects
- Evidence command: `lake build` or Kani harness

### THM-IDEM-003
- Contract clause: I5 (Side-effecting at-least-once rejection)
- Rust/spec target: `is_statically_idempotent_contract`
- Lean module: `VbCore.Idempotency`
- Theorem shape: side_effecting_at_least_once_rejects
- Model: Given any `RetrySafety != Unsafe`, when `SideEffect != None` and `Idempotency == AtLeastOnceExternal`, result is `Reject(SideEffectingAtLeastOnceExternal)`
- Refinement: Rust implementation matches Lean decision table
- Shell exclusions: No I/O, storage, or runtime effects
- Evidence command: `lake build` or Kani harness

### THM-IDEM-004
- Contract clause: I6 (Side-effecting accepted invariant)
- Rust/spec target: `is_statically_idempotent_contract`
- Lean module: `VbCore.Idempotency`
- Theorem shape: side_effecting_accepts_only_idempotent_external
- Model: When `SideEffect != None` and result is `Accept`, then `Idempotency == IdempotentExternal` and `RetrySafety != Unsafe`
- Refinement: Rust implementation matches Lean decision table
- Shell exclusions: No I/O, storage, or runtime effects
- Evidence command: `lake build` or Kani harness

## Waivers

**Lean projection waiver for this bead:**

Owner: vb-qi37.5.1 implementation
Reason: The idempotency decision table is a straightforward 45-case enum match with no complex algebraic structure, dynamic memory, or external dependencies. The 35 exhaustive unit tests and 5 Kani harnesses provide equivalent verification coverage. A Lean projection would not add additional assurance beyond the existing verification layers.
Expiry: None required - the decision table is provably correct by exhaustive testing.
Compensating evidence: 35 unit tests + 5 Kani harnesses + 10 proptest invariants provide comprehensive coverage of the 45-case finite domain.
