# Test Review: VB-FG1PI Verus Proof Coverage

**Reviewed source SHA:** 8a7e2340f14f739c57937c9cd2d06d3d8d3010a8
**Reviewer:** local5090 (independent)
**Date:** 2026-10-21

## Status: REJECTED

Proof coverage incomplete for input slot validation. The three new WorkflowError variants (InputSlotOutOfBounds, DuplicateInputSlot, NonCanonicalInputSlotOrder) have no spec coverage and no production tests.

## Scope

This review examines the Verus proofs and behavioral test coverage for the Verus mirror refinement in bead VB-FG1PI. The focus is on the new `CompiledInputSlot`, `InputSlotKind`, and `validate_input_slots` validation path.

## Proof Coverage Analysis

### try_from_parts validation pipeline

The production validation pipeline now has three steps:
1. `validate_parts` — structural validation
2. `validate_budget` — whole-workflow budget
3. `validate_input_slots` — input slot validation (NEW)

**Proof coverage:**
- Step 1: Covered by `validate_parts_pure` spec + proofs. ✓
- Step 2: Covered by `validate_budget_pure` spec + proofs. ✓
- Step 3: NOT covered. The `try_from_parts_pure` decision function does not model `validate_input_slots`. ✗

### Error variant coverage

Production has 23 WorkflowError variants. The spec's `SpecWorkflowError` has 18. Missing variants:

| Production Variant | Line | In Spec? | Proof Coverage? |
|-------------------|------|----------|-----------------|
| InputSlotOutOfBounds | 527-532 | No | No |
| DuplicateInputSlot | 535-540 | No | No |
| NonCanonicalInputSlotOrder | 543-548 | No | No |

The three new variants are reachable via production compilation but have no spec representation and no proofs.

### Other validation paths

All other validation paths appear unchanged and retain their existing proof coverage:
- Empty nodes check ✓
- Entry bounds ✓
- Slot/const/step bounds ✓
- Expression/accessor validation ✓
- Node kind validation ✓
- Budget validation ✓
- Resource contract validation ✓

## Production Test Coverage

Checked existing test suites for input slot validation tests:

- `crates/vb_core/src/workflow/mod.rs` tests: No tests for `validate_input_slots` found.
- `crates/vb_compile/` tests: No compile-time tests for input slot validation found.

The production validation function `validate_input_slots` has no production behavior tests. This means:
1. There is no regression test that would catch a bug in `validate_input_slots`.
2. The Verus proofs cannot be verified against production behavior because there is no production behavior to verify against.

## Mutation Thought Experiment

### Input slot out-of-bounds mutation
If `validate_input_slots` were mutated to not check `current_value >= slot_count`, the following would be caught:
- Verus proof: No (spec decision function doesn't model this validation)
- Production test: No (no test exists for this path)

Result: Mutation not detected.

### Duplicate slot mutation
If `validate_input_slots` were mutated to not check for duplicate slots:
- Verus proof: No
- Production test: No

Result: Mutation not detected.

### Non-canonical order mutation
If `validate_input_slots` were mutated to not check slot ordering:
- Verus proof: No
- Production test: No

Result: Mutation not detected.

### validate_input_slots entirely removed
If the third validation step were removed from `try_from_parts`:
- Verus proof: No (spec doesn't expect this step)
- Production test: No (no test exercises this path)
- Drift gate: No (the production_inner stub returns Ok(()) regardless)

Result: Mutation not detected.

## Verus Proof Verification Results

All Verus proofs compile and execute with exit code 0:

```
extern_try_from_parts.rs: PASS (0.449s)
extern_budget_monotonic.rs: PASS (0.287s)
extern_ipc_runtime_transitions.rs: PASS (0.268s)
extern_recovery_verification.rs: PASS (0.267s)
extern_storage_kind_family.rs: PASS (0.262s)
extern_vb_core_replay_step.rs: PASS (0.253s)
extern_vb_jnz9_journal_event_seq_valid.rs: PASS (0.249s)
extern_vb_jpq724_events_for_run_production.rs: PASS (0.263s)
```

All 8 proofs pass, but the try_from_parts proof coverage is incomplete for the new validation step.

## Recommendations

### For Verus proof coverage

1. **Extend SpecWorkflowError:** Add the three missing input slot error variants to the spec's closed-set discriminants.

2. **Extend try_from_parts_pure:** Model the validate_input_slots validation step in the spec's decision function. The decision function should check:
   - All input slot indices are within slot_count bounds
   - No duplicate input slot indices
   - Input slots are in strictly increasing order

3. **Write Verus tests:** Add Verus test functions that exercise the new validation paths and their associated error variants.

### For production test coverage

1. **Write behavior tests for validate_input_slots:** Tests should verify each of the three validation checks and their associated error variants.

2. **Write compilation tests:** Compile workflows with malformed input slots (out-of-bounds, duplicate, out-of-order) and verify the correct error is raised.

## Conclusion

The Verus mirror refinement is structurally sound and all proofs compile and pass. However, the new `validate_input_slots` validation step is not covered by the spec's decision function, and the three new error variants are not represented in the spec's closed-set discriminants. This means the new validation path is unproven and untested. The code is rejected on test/proof coverage grounds until these gaps are addressed.