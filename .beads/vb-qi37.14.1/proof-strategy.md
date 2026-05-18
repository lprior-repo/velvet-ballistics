# Proof Strategy: `run --step` (vb-qi37.14.1)

## Risk Classification

| Risk Tag | Classification | Impact |
|---|---|---|
| `structured-output-gap` | Rust-local / I/O | Medium — JSON/JSONL output requires integration test coverage |
| `delta-reporting` | Rust-local / state snapshot | High — before/after frame diff is acceptance-critical |
| `durability-gates` | Invariant enforcement | Low — already correct by inspection |
| `typed-errors` | Exhaustive error mapping | Medium — ERR-001 taxonomy is complete, unit+integration sufficient |

## Verifier Lane Assignments

### Verus (proof-fn layer)
**Why**: Rust-local invariants INV-001, INV-002, INV-004, INV-006 are safety-critical
and cheaper to prove than exhaustively test.

- **VB-INV001-VERUS**: `RunFrame::new` bounds proof. Artifact:
  `verification/verus/run_frame_invariant.rs`. Command: `verus
  crates/vb_core/src/frame.rs`. Assumes `step_count > 0`, `first_step <
  step_count`. Shell exclusions confirmed (no I/O, no async, no storage).
- **VB-INV002-VERUS**: `mark_step_after_signal` exhaustiveness proof. Artifact:
  `verification/verus/step_state_machine.rs`. Command: `verus
  crates/vb_core/src/engine/step.rs`. Assumes closed enums
  `EngineSignal`/`StepState`. Shell exclusions confirmed.
- **VB-INV004-VERUS**: `step_once` PC bounds proof. Artifact:
  `verification/verus/signals_invariant.rs`. Command: `verus
  crates/vb_core/src/engine/step.rs`. Assumes `set_pc` validates before write.
- **VB-INV006-VERUS**: `write_slot_with_taint` taint validity proof. Artifact:
  `verification/verus/run_frame_invariant.rs`. Command: `verus
  crates/vb_core/src/frame.rs`. Assumes `Taint` enum has exactly 3 closed
  variants.

### Kani (bounded model-check layer)
**Why**: Bounded state (step_count ≤ 16, slot_count ≤ 32) allows honest bounded
checking of panic freedom, state-mapping, and error-path coverage.

- **VB-PRE002-KANI**: `step_once` returns `Err(InvalidProgramCounter)` on OOB.
  Harness: `step_once_bounds_harness`. Bounds: `step_count ∈ [1, 16]`,
  `slot_count ∈ [0, 32]`. Input: `kani::Arbitrary` for `CompiledWorkflow`,
  `RunFrame`, `ValueStore`.
- **VB-INV002-KANI**: State mapping after `step_once`. Harness:
  `step_once_state_mapping_harness`. Checks `states[step]` matches signal
  mapping for all `EngineSignal` variants via `kani::cover`.
- **VB-INV003-KANI**: No panic on uninitialized slot read. Harness:
  `step_once_slot_init_harness`. Bounded slot state.
- **VB-INV004-KANI**: PC in bounds after `step_once`. Harness:
  `step_once_pc_bounds_harness`.
- **VB-INV006-KANI**: Taint validity after `write_slot_with_taint`. Harness:
  `taint_validity_harness`. Three-variant enum makes this a small bounded check.
- **VB-ERR001-KANI**: No panic on any `EngineError` path. Harness:
  `step_once_error_harness`.

### Unit Tests (Rust)
**Why**: Happy-path dispatch for each `CompiledNodeKind`, error variants,
durability gate, argument parsing. Fast, debuggable, comprehensive.

Targets: `crates/vb_cli/src/app_impl.rs`,
`crates/vb_core/src/engine/step.rs`, `crates/vb_core/src/frame.rs`.

### Integration Tests (black-box CLI)
**Why**: End-to-end correctness of CLI arguments, file I/O, output format,
exit codes. Cannot be verified in-process.

Targets: `crates/vb_cli/tests/cli_integration.rs`.

### Clippy (static analysis)
**Why**: Zero-tolerance for `unwrap`/`expect`/`panic` in step execution paths.
Existing `forbid(unsafe_code)` on `step.rs` and `frame.rs` provides UB-free
baseline.

Command: `cargo clippy --package vb_cli --package vb_core --lib --bins -- -D
warnings`.

## Waiver: TLA+

**Not applicable — confirmed in contract.md §TLA+-Owned and
verification-layers.md §Waiver: TLA+.**

Rationale: `run --step` is a single-shot pure function. No state machine,
no loop, no temporal behavior, no concurrent actors, no liveness property.
TLA+ spec would be a single-state dot with no verification value. Waiver
approved by contract.

## Waiver: Lean/Aeneas/Hax

**Not applicable — confirmed in contract.md §Theorem-Owned and
verification-layers.md §Waiver: Lean.**

Rationale: The 9×9 `StepState` transition boolean matrix is exhaustively
verified by Kani + unit tests. No theorem prover needed.

## Budget Summary

| Lane | Obligations | Est. Time |
|---|---|---|
| Verus | 4 (INV-001, 002, 004, 006) | ~8 min |
| Kani | 6 (PRE-002, INV-002/003/004/006, ERR-001) | ~15 min |
| Unit | 5 (PRE-001, POST-007, INV-005, ERR-001, CLIPPY) | ~3 min |
| Integration | 13 (PRE-002/003/004/005, POST-001/002/003/004/005/006/008) | ~5 min |
| Waivers | 2 (TLA+, Lean) | ~0 min |

## Assumptions and Model Bounds

- `step_count` bounded to `[1, 16]` in all Kani harnesses (matches
  `delivery-scope.jsonl` realistic workflow size)
- `slot_count` bounded to `[0, 32]` in all Kani harnesses
- `kani::Arbitrary` for `CompiledWorkflow` constructed from arbitrary
  `WorkflowParts`; `RunFrame` constructed via `RunFrame::new` with valid
  bounds
- No concurrent execution within one CLI invocation (A3 in contract.md)
- `EngineSignal` and `StepState` are closed enums (verified at type-system
  level)

## Open Questions (must resolve before proof execution)

- **Q2**: JSON output — full `SlotValue` serialization or summary? Affects
  `VB-POST005-INT` test oracle.
- **Q3**: Delta reporting — diff-only (changed slots) or full frame snapshot?
  Affects `VB-POST004-INT` expected evidence.

Both questions are marked UNKNOWN; proof-writer must not assume either
direction. Resolved by contract decision before integration test writing.
