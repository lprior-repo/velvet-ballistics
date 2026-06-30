# Test Plan: vb-iucs

## Test Strategy

This recovery is proof/evidence integration, not new product behavior. Test evidence is therefore verifier execution plus source-binding checks.

| Test | Evidence |
|------|----------|
| Gate 8 valid bounded accessors pass | `kani_gate_08_valid_bounded_parts_pass` |
| Gate 8 zero accessors pass | `kani_gate_08_valid_zero_accessors_pass` |
| Gate 8 index-only accessors without symbols pass | `kani_gate_08_valid_index_without_symbols_pass` |
| Gate 8 no-panic bounded inputs | `kani_gate_08_no_panic_bounded_inputs` |
| Gate 8 field symbol OOB rejected | `kani_gate_08_field_symbol_oob_rejected` |
| Gate 8 `u32::MAX` index rejected | `kani_gate_08_index_u32_max_rejected` |
| Gate 8 root OOB rejected | `kani_gate_08_root_oob_rejected` |
| StepState all-pairs parity | `kani_step_state_transition_matches_contract` |
| StepState Verus mirror | `verification/verus/step_state_machine.rs` |
| BudgetArithmetic bounded TLA+ model | `specs/tla/BudgetArithmetic.tla` |
