# Proof Strategy - vb-0253.5

## Bead
**ID**: vb-0253.5  
**Title**: Align StepState contract across runtime and proofs

## Risk Classification
| Risk | Assessment |
|------|------------|
| Temporal | High - state machine transitions |
| Verification | High - proof/runtime alignment |
| Public API | Medium - StepState is public |

## Verifier Selection
| Risk | Verifier | Rationale |
|------|----------|-----------|
| State machine validity | Kani + Verus + TLA+ | Multi-verifier approach for critical state |
| Enum correctness | Verus | Rust-local type invariant |
| Transition table | Verus + Kani | Exhaustive checking |

## Proof Obligations

### PO-001: StepState Transition Validity
- **Requirement**: INV-002
- **Verifiers**: Kani + Verus
- **Artifact**: vb_proof_kernels/src/step_state.rs
- **Command**: `cargo kani --harness step_state_transition` + `verus vb_proof_kernels/src/step_state.rs`
- **Expected Evidence**: Kani no witness + Verus 0 errors
- **Required**: Yes
- **Mode**: verify-deep

### PO-002: StepState Enum Correctness
- **Requirement**: INV-001
- **Verifier**: Verus
- **Artifact**: vb_proof_kernels/src/step_state.rs
- **Command**: `verus vb_proof_kernels/src/step_state.rs`
- **Expected Evidence**: Verus verified enum with 0 errors
- **Required**: Yes
- **Mode**: verify-proof

### PO-003: State Machine Protocol
- **Requirement**: INV-002
- **Verifier**: TLA+
- **Artifact**: specs/StepState.tla
- **Command**: `tlc -config specs/StepState.cfg specs/StepState.tla`
- **Expected Evidence**: TLC no invariant violations
- **Required**: No (supplementary)
- **Mode**: verify-standard

## Waiver Requests
None.

## Strategy Summary
- Primary: Kani + Verus for state machine verification
- Supplementary: TLA+ for protocol-level model
- Focus: Align runtime usage with proof definitions
