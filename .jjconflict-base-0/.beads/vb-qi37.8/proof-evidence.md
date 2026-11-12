# Proof Evidence: vb-qi37.8

## Scope

This repair records current-tree evidence for the proof artifacts touched in this session. It does not refresh every historical validation-pipeline obligation from the original bead.

## Current-Tree Evidence

| Area | Command | Result | Raw Evidence |
|------|---------|--------|--------------|
| Workspace metadata | `cargo metadata --no-deps` | PASS | `/home/lewis/.local/share/opencode/tool-output/tool_e34fbb7280013kY0QiT8e8IwMQ` |
| StepState Kani parity | `cargo kani -p vb_core --harness kani_step_state_transition_matches_contract --output-format=regular` | PASS, `0 of 98 failed`, `3 of 3 cover properties satisfied` | `/home/lewis/.local/share/opencode/tool-output/tool_e34fbcc37001x2PAzA97hgWznY` |
| StepState Verus mirror | `verus verification/verus/step_state_machine.rs` | PASS, `6 verified, 0 errors` | `.beads/vb-qi37.8/evidence/verus-step-state-machine.out` |
| BudgetArithmetic TLC | `tlc -config specs/tla/BudgetArithmetic.cfg specs/tla/BudgetArithmetic.tla` | PASS, `166 states generated`, `84 distinct states found`, depth `2` | `.beads/vb-qi37.8/evidence/tlc-budget-arithmetic.out` |

## Gate 8 Kani Evidence

All Gate 8 harnesses live in `crates/vb_validate/src/kani_gate_08_accessor.rs` and were rerun after restoring the missing success-case harnesses.

| Harness | Source Lines | Result | Raw Evidence |
|---------|--------------|--------|--------------|
| `kani_gate_08_valid_bounded_parts_pass` | 12-34 | PASS, `0 of 502 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34ef1482001qcOlXtLV6oho6J` |
| `kani_gate_08_valid_zero_accessors_pass` | 36-42 | PASS, `0 of 691 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34f581fb001fuSDAdY6gUn2ug` |
| `kani_gate_08_valid_index_without_symbols_pass` | 44-63 | PASS, `0 of 703 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34f5b38d0013LAew53aQImHU4` |
| `kani_gate_08_no_panic_bounded_inputs` | 65-84 | PASS, `0 of 473 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34f700520011ShVGRxe0Y3Jzl` |
| `kani_gate_08_field_symbol_oob_rejected` | 86-112 | PASS, `0 of 700 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34fa8963001IInVFbayevs0LH` |
| `kani_gate_08_index_u32_max_rejected` | 114-130 | PASS, `0 of 699 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34fab23a001J3G2O3ssQdacfZ` |
| `kani_gate_08_root_oob_rejected` | 132-154 | PASS, `0 of 693 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34faec38001KSsoRuQHUd5m1B` |

## Deferred Scope

| Obligation | Status | Reason |
|------------|--------|--------|
| `PO-030` full validation pipeline composition | `DEFERRED_GLOBAL` | Gate 8 Kani evidence proves Gate 8 accessor behavior only; it does not prove whole-pipeline composition. |

## Notes

Gate 8 Verus is not claimed here. Any Verus work for Gate 8 remains deferred outside this repair scope.
