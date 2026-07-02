# Proof Evidence: vb-iucs

| Obligation | Command | Result | Raw Evidence |
|------------|---------|--------|--------------|
| Gate 8 Kani | seven Gate 8 harnesses in `crates/vb_validate/src/kani_gate_08_accessor.rs` | PASS, each `0 failed` | `.beads/vb-qi37.8/formal-verification-report.md` lines 18-30 |
| StepState Kani | `cargo kani -p vb_core --harness kani_step_state_transition_matches_contract --output-format=regular` | PASS, `0 of 98 failed`, `3 of 3 cover properties satisfied` | `.beads/vb-qi37.8/formal-verification-report.md` lines 32-37 |
| StepState Verus | `verus verification/verus/step_state_machine.rs` | PASS, `6 verified, 0 errors` | `.beads/vb-qi37.8/evidence/verus-step-state-machine.out` |
| BudgetArithmetic TLC | `tlc -config specs/tla/BudgetArithmetic.cfg specs/tla/BudgetArithmetic.tla` | PASS, `166 states generated`, `84 distinct states found`, depth `2` | `.beads/vb-qi37.8/evidence/tlc-budget-arithmetic.out` |

## Current Source Binding

- `crates/vb_core/src/frame.rs` delegates `is_valid_step_state_transition` to `vb_proof_kernels::step_state::is_valid_transition`.
- `crates/vb_core/src/kani_step_state_transition.rs` calls the production runtime predicate.
- `verification/verus/step_state_machine.rs` names the proof-kernel source, runtime refinement target, and Kani parity harness.
