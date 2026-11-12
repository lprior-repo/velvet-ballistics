# Verification Layers

## Boundary
- **Verus-owned kernel**: All `lower_*` functions — pure slot recording and node construction
- **TLA+ temporal model**: Structural well-formedness of step chain (no duplicate IDs, correct offsets, AskResume chain)
- **Theorem projection**: None — Verus is sufficient
- **Runtime shell**: `WorkflowBuilder` mutable state, `SlotCompiler`, error propagation to `CompileError`
- **External systems excluded from formal proof**: None — pure compilation transform

## Layer Assignment
| Contract Clause | Primary Layer | Secondary | Evidence |
|---|---|---|---|
| PRE-001 (id-plus-one range) | verus | kani | VERUS-INV-001, VERUS-INV-002 |
| PRE-002 (slot recording) | verus | — | VERUS-INV-003 |
| POST-001 to POST-007 (correct node count/kind) | verus | — | VERUS-POST-001..007 |
| INV-001 (step width) | verus | — | VERUS-INV-WIDTH |
| INV-003 (no overflow in id+1) | verus | kani | VERUS-INV-001, KANI-OVERFLOW |
| ERR-TYPES (CompileError exhaustive) | static-scan | — | clippy |
| TLA-WF-001 (well-formed chain) | tla-plus | — | TLC model check |
| TLA-WF-002 (slot bounded) | tla-plus | — | TLC model check |

## Verus Scope
- **Rust target**: `crates/vb_compile/src/lib.rs`
- **Spec/proof function**:
  - `spec_lower_repeat_preserves_no_overflow(id, max_attempts, body, done) -> bool`
  - `spec_lower_ask_preserves_no_overflow(id, prompt, answer, timeout) -> bool`
  - `proof_lower_repeat_length_3()`
  - `proof_lower_ask_length_2()`
  - `proof_foreach_start_kind_correct()`
- **Invariants**: `id.checked_add(1).is_some()` precondition for repeat/ask
- **Trusted boundary**: `StepIdx::new`, `SlotIdx::new`, `CompiledNode` constructors
- **Shell exclusions**: `WorkflowBuilder`, mutable slot recording, I/O, async

## TLA+ Scope
- **Module/model path**: `specs/ControlLowering.tla`
- **Variables**: `steps`, `slots`
- **Actions**: `LowerForEach`, `LowerTogether`, `LowerCollect`, `LowerReduce`, `LowerRepeat`, `LowerAsk`, `LowerWait`
- **Safety invariants**: `NoDuplicateStepIds`, `ValidOffsets`, `AskResumeIdCorrect`
- **Temporal properties**: None (pure function)
- **Fairness/deadlock stance**: Not applicable
- **Refinement boundary**: Each TLA+ action corresponds 1:1 to one `lower_*` Rust function
- **Evidence command**: `tlc -config specs/ControlLowering.cfg specs/ControlLowering.tla`

## Kani Scope
- **Harness**: `kani_capability_harnesses.rs` — bounded overflow check for id+1
- **Command**: `cargo kani --harnesses kani_lower_control_primitives`
- **Expected**: No counterexamples for `u16::MAX - 1` and below

## Waivers
- Theorem kernel: Not needed — Verus can express all pure lowering invariants
- Fuzzing: Not applicable — YAML AST is pre-validated; lowering is deterministic
- Loom/Shuttle: Not applicable — no concurrency in the lowering pass
