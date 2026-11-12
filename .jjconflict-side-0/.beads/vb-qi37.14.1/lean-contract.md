# Theorem Kernel Projection: `run --step`

## Boundary

- **TLA+-owned temporal model**: None. `run --step` is a single-shot pure function with no temporal behavior. See `tla-spec.md` for rationale.
- **Verus-owned Rust core**: step-state mapping invariant (INV-002), PC bounds invariant (INV-004), taint lattice correctness (INV-006), step_once panic freedom, and slot/taint/state delta correctness.
- **Theorem-owned kernel**: None for this bead. The step-state transition matrix in `vb_proof_kernels::step_state::is_valid_transition` is a 9×9 boolean table. Exhaustiveness is verified by Kani and unit tests, not by a theorem prover.
- **Rust/runtime shell**: CLI argument parsing (`args.rs`), file I/O (`read_file`, `decode_step_inputs`), output formatting (`print_step_result_json*`), and exit code mapping.
- **External systems excluded from theorem proof**: None — `run --step` is fully self-contained.

## Verus-Owned Clauses (Rust-Local Proof Obligations)

The Verus proof surface for this bead is in `crates/vb_core/src/engine/step_verus.rs` (to be created). The proof obligations are:

### V-Obligation 1: INV-002 — Step-State Mapping Invariant

- **Contract clause**: INV-002
- **Rust target**: `crates/vb_core/src/engine/step.rs::mark_step_after_signal`
- **Verus spec function**: `spec_mark_step_after_signal(signal: EngineSignal, step: StepIdx, states: Seq<StepState>) -> Seq<StepState>`
- **Verus proof function**: `proof_inv_step_state_mapping(signal: EngineSignal, step: StepIdx, states: Seq<StepState>) -> bool`
- **Invariant**: For all `signal ∈ EngineSignal` and all valid `step`, `proof_inv_step_state_mapping` returns true iff the resulting states[step] matches the mapping table:
  - Continue → Succeeded
  - Finished → Succeeded
  - AwaitingAction → Running
  - AwaitingWait → Waiting
  - AwaitingAsk → Asking
  - StepBudgetExhausted → Running
- **Trusted boundary**: The `EngineSignal` enum variants are closed and exhaustive; the `StepState` enum is closed and exhaustive. No dynamically dispatched code in the invariant path.
- **Shell exclusions**: No I/O, no async, no storage, no wall-clock time in the invariant proof.

### V-Obligation 2: INV-004 — PC Bounds After step_once

- **Contract clause**: INV-004
- **Rust target**: `crates/vb_core/src/engine/step.rs::step_once`
- **Verus spec function**: `spec_step_once_pc_bounds(plan: &CompiledWorkflow, run: &RunFrame) -> StepIdx`
- **Invariant**: After `step_once` returns `Ok(signal)`, `run.pc() < run.step_count()`
- **Trusted boundary**: `CompiledWorkflow::node(pc).is_some()` check in `step_once`; `set_pc` validates bounds before writing.
- **Shell exclusions**: No I/O, no async, no storage.

### V-Obligation 3: INV-006 — Taint Validity After write_slot_with_taint

- **Contract clause**: INV-006
- **Rust target**: `crates/vb_core/src/frame.rs::RunFrame::write_slot_with_taint`
- **Verus lemma**: `lemma_taint_valid_after_write(slot: SlotIdx, value: SlotValue, taint: Taint, result: Result<(), CoreError>)`
- **Invariant**: If `write_slot_with_taint` returns `Ok(())`, then for the written `slot`, `frame.taint[slot] ∈ {Clean, DerivedFromSecret, Secret}`.
- **Trusted boundary**: `Taint` is a non-exhaustive enum with three valid variants. The `join_taint` function is a pure function on the discriminant values. No dynamically constructed Taint values.
- **Shell exclusions**: No I/O, no async, no storage.

### V-Obligation 4: PRE-001 — Durability Gate in CLI

- **Contract clause**: PRE-001
- **Rust target**: `crates/vb_cli/src/app_impl.rs::cmd_run_step`
- **Verus spec function**: `spec_cmd_run_step_precondition(durability: DurabilityMode) -> bool`
- **Invariant**: `durability == DurabilityMode::None` iff the precondition holds.
- **Note**: This is a simple enum comparison, not a complex proof. Covered by unit test. Not worth a Verus proof for the CLI layer.

## Lean/Aeneas/Hax Theorem Kernel Clauses

None.

**Rationale**: The step-state transition table (`is_valid_transition`) in `vb_proof_kernels::step_state` is a 9×9 boolean matrix. It can be exhaustively verified by:
1. Unit tests in `step.rs` (each transition tested)
2. Kani harness over all `(current, new)` pairs
3. Proptest over all `StepState × StepState` pairs

There is no algebraic structure, no lattice, no arithmetic bound, and no protocol lattice that requires a proof assistant. The proof kernel is simply: for all 81 combinations, check if the boolean matrix entry is true for valid transitions and false for invalid ones. Kani can exhaust this with a bounded harness.

## Waivers

| Clause ID | Layer | Owner | Reason | Limitation | Compensating Evidence |
|-----------|-------|-------|--------|------------|----------------------|
| TLA+ temporal model | tla-plus | vb-qi37.14.1 | Single-shot pure function; no temporal behavior, loop, concurrency, or protocol | Permanent non-applicability | Unit tests + Kani + Verus |
| Lean theorem kernel | lean | vb-qi37.14.1 | Step-state table is 9×9 boolean; exhaustively verified by Kani/unit tests | N/A | Kani + unit tests |
| Verus for CLI layer | verus | vb-qi37.14.1 | CLI argument parsing is not pure Rust core logic; simple enum comparison | N/A | Unit tests |

## Summary

- **Verus**: INV-002 (step-state mapping), INV-004 (PC bounds), INV-006 (taint validity) — Rust core pure invariants
- **Kani**: step_once panic freedom, bounded model over all EngineSignal × StepState combinations
- **Unit tests**: All `EngineSignal` variants covered, all `CompiledNodeKind` dispatch paths covered
- **CLI integration tests**: End-to-end scenarios in `cli_integration.rs`
- **Lean**: Not applicable
- **TLA+**: Not applicable (see tla-spec.md)
