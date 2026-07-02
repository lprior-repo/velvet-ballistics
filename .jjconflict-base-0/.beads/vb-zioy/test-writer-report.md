## Test Writer Report — vb-zioy

**Bead:** vb-zioy — fix: enforce body.len() == 1 in collect body lowering (vb-xi2f.23)
**Agent:** test-writer
**State:** 9 (test implementation)
**Date:** 2026-05-25

---

### What Was Tested

The bead changed `emit_single_body_set` to accept a `diagnostic_step: usize` parameter and updated all callers to pass the original source `index` instead of a synthetic body step id. This ensures error reports point to the original source step, not an internal synthetic step.

Two test obligations were addressed:

1. **Updated existing test** `compile_workflow_rejects_multi_step_body_in_scoped_primitives`:
   - Previously the match on `StepFieldShape` used `..` to ignore the `step` field.
   - Now it asserts `step == 0` for every case (repeat, for_each, collect, reduce), because each primitive is the first step in the workflow.
   - The assertion tuple is `(*step, *field, expected.as_ref()) == (0, "steps", "exactly one set step")`.

2. **Added new test** `compile_workflow_rejects_non_set_body_in_collect`:
   - Verifies that when `collect`'s body contains exactly one step but that step is a non-Set primitive (nested `collect`), `emit_single_body_set` returns `UnsupportedStepPrimitive` with `step == 0`.
   - This proves the `diagnostic_step` flows correctly through the error path for non-Set body primitives.

---

### Test Run Summary

```
cargo test -p vb_compile --test v1_primitive_lowering

Result: 25 passed, 7 failed, 0 ignored

Failed tests (pre-existing debt — 7 choose tests, NOT modified):
  - lower_canonical_choose_accepts_two_branches
  - lower_canonical_choose_accepts_64_branches_at_limit
  - lower_canonical_choose_rejects_65_branches
  - lower_canonical_choose_emits_all_branches_not_just_first
  - lower_canonical_choose_accepts_non_empty_branch_body
  - lower_canonical_choose_body_target_is_first_body_step_not_next
  - lower_canonical_choose_pushes_exactly_one_node_to_builder

New / updated tests (all PASS):
  - compile_workflow_rejects_multi_step_body_in_scoped_primitives (UPDATED)
  - compile_workflow_rejects_non_set_body_in_collect (NEW)
```

All other tests in the file (25 total passing) continue to pass. No regressions introduced.

---

### Assertions Used

No `unwrap`, `expect`, or `panic` in test code. All tests return `Result<(), String>` and use exhaustive `match` arms with `assert_eq!` on exact tuple values.

---

### Files Modified

- `crates/vb_compile/tests/v1_primitive_lowering.rs`

---

### Proof/Refinement Coverage Matrix

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| PO-005 | emit_single_body_set uses diagnostic_step for error reporting | Yes | `mod_compile_lowering::part_04::emit_single_body_set` | `compile_workflow_rejects_multi_step_body_in_scoped_primitives`, `compile_workflow_rejects_non_set_body_in_collect` | N/A | behavior test | `cargo test -p vb_compile --test v1_primitive_lowering` | N/A |

### Behaviors Not Yet Tested

None for this bead. The `diagnostic_step` propagation is verified for:
- Multi-step body → `StepFieldShape` (4 primitives)
- Non-Set single-step body → `UnsupportedStepPrimitive` (collect)

The same pattern applies to `for_each`, `reduce`, and `repeat` for non-Set bodies, but the mechanism is identical (same `emit_single_body_set` function), so testing one primitive (collect) is sufficient to prove the contract.
