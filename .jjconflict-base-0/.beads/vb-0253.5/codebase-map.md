# Codebase Map - vb-0253.5

## Bead
**ID**: vb-0253.5  
**Title**: Align StepState contract across runtime and proofs

## Scope

### Relevant Crates/Files
| Path | Role | Risk Tags |
|------|------|-----------|
| crates/vb_proof_kernels/src/step_state.rs | StepState enum and transition proofs | verification, temporal |
| crates/vb_core/src/frame.rs | Core StepState type definition | public-api |
| crates/vb_core/src/kani_step_state_transition.rs | Kani proof harness for StepState | verification |
| crates/vb_core/src/lib.rs | Core re-exports including StepState | public-api |
| crates/vb_runtime/src/runtime.rs | Runtime StepState usage | runtime |
| crates/vb_runtime/src/shard/helpers/tests.rs | StepState test helpers | test |
| crates/vb_ui_snapshot/src/fixtures/execution/overview.rs | StepStateView snapshots | ui |
| crates/vb_ui_snapshot/src/fixtures/execution/details.rs | StepStateView snapshots | ui |

### Key Symbols
- `StepState` enum: `Pending`, `Running`, `Waiting`, `Asking`, `Succeeded`, `Failed`, `Cancelled`, `Skipped`
- `is_valid_transition(from, to)`
- `validate_transition(from, to)`
- `next_states(from)`
- `terminal_states()`
- `non_terminal_states()`

### Public API Surface
- `vb_core::frame::StepState`
- `vb_core::StepState`
- `vb_core::is_valid_step_state_transition()`

### Risk Tags
- **verification**: Formal proofs in vb_proof_kernels
- **temporal**: State machine transition properties
- **runtime**: Runtime behavior depends on StepState validity

### Open Questions
- What is the specific contract misalignment?
- Which runtime vs proof definitions don't align?
- Is there a TLA+ spec for StepState transitions?

### Excluded Paths (Not in Scope)
- vb_runtime shard command queue (separate bead vb-0253.1)
- vb_ipc ingress (separate bead vb-0253.2)

### Downstream Owners
- rust-contract: define StepState contract alignment
- proof-planner: verify StepState transition properties
- proof-reviewer: review proof adequacy for contract
