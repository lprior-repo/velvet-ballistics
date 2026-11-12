# Formal Verification Report: vb-qi37.8

**bead_id**: vb-qi37.8
**executed**: 2026-05-17
**scope**: current-tree repair evidence for Gate 8 Kani, StepState Kani/Verus, and BudgetArithmetic TLC.

## Executive Summary

| Lane | Status | Evidence |
|------|--------|----------|
| Cargo metadata | PASS | `cargo metadata --no-deps` resolved workspace metadata |
| Gate 8 Kani | PASS | 7 named harnesses verified, each with `0 failed` |
| StepState Kani | PASS | `0 of 98 failed`, `3 of 3 cover properties satisfied` |
| StepState Verus | PASS | `6 verified, 0 errors` |
| BudgetArithmetic TLC | PASS | `166 states generated`, `84 distinct states found`, depth `2` |
| PO-030 pipeline composition | DEFERRED_GLOBAL | Not refreshed by this Gate 8 repair |

## Gate 8 Kani Results

| Harness | Result | Raw Evidence |
|---------|--------|--------------|
| `kani_gate_08_valid_bounded_parts_pass` | PASS, `0 of 502 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34ef1482001qcOlXtLV6oho6J` |
| `kani_gate_08_valid_zero_accessors_pass` | PASS, `0 of 691 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34f581fb001fuSDAdY6gUn2ug` |
| `kani_gate_08_valid_index_without_symbols_pass` | PASS, `0 of 703 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34f5b38d0013LAew53aQImHU4` |
| `kani_gate_08_no_panic_bounded_inputs` | PASS, `0 of 473 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34f700520011ShVGRxe0Y3Jzl` |
| `kani_gate_08_field_symbol_oob_rejected` | PASS, `0 of 700 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34fa8963001IInVFbayevs0LH` |
| `kani_gate_08_index_u32_max_rejected` | PASS, `0 of 699 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34fab23a001J3G2O3ssQdacfZ` |
| `kani_gate_08_root_oob_rejected` | PASS, `0 of 693 failed` | `/home/lewis/.local/share/opencode/tool-output/tool_e34faec38001KSsoRuQHUd5m1B` |

The restored harnesses are `kani_gate_08_valid_zero_accessors_pass` and `kani_gate_08_valid_index_without_symbols_pass` in `crates/vb_validate/src/kani_gate_08_accessor.rs`.

## StepState Evidence

| Verifier | Command | Result | Raw Evidence |
|----------|---------|--------|--------------|
| Kani | `cargo kani -p vb_core --harness kani_step_state_transition_matches_contract --output-format=regular` | PASS, `0 of 98 failed`, `3 of 3 cover properties satisfied` | `/home/lewis/.local/share/opencode/tool-output/tool_e34fbcc37001x2PAzA97hgWznY` |
| Verus | `verus verification/verus/step_state_machine.rs` | PASS, `verification results:: 6 verified, 0 errors` | `.beads/vb-qi37.8/evidence/verus-step-state-machine.out` |

## TLA+ Evidence

| Model | Command | Result |
|-------|---------|--------|
| `specs/tla/BudgetArithmetic.tla` | `tlc -config specs/tla/BudgetArithmetic.cfg specs/tla/BudgetArithmetic.tla` | PASS, no errors, `166 states generated`, `84 distinct states found`, depth `2`; raw `.beads/vb-qi37.8/evidence/tlc-budget-arithmetic.out` |

## Deferred And Non-Claims

| Item | Status | Rationale |
|------|--------|-----------|
| `PO-030` pipeline composition | `DEFERRED_GLOBAL` | Full validation pipeline Kani composition was not rerun. |
| Gate 8 Verus | `DEFERRED_GLOBAL` | No Gate 8 Verus proof is claimed by this report. |

## Conclusion

The current checkout has executable Gate 8 Kani proof coverage for valid zero accessors, valid index-only accessors without symbols, bounded valid accessors, no-panic bounded inputs, field-symbol rejection, `u32::MAX` index rejection, and root-slot out-of-bounds rejection. This evidence is scoped to Gate 8 and must not be used as proof of full pipeline composition.
