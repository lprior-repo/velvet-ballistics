# Kani Evidence: vb-gvmt

## Command

```bash
cargo kani -p vb_codegen \
  --harness join_taint_is_monotonic_for_generated_lattice_model \
  --harness no_contract_action_rejects_clean_output_from_tainted_input \
  --harness journal_capacity_precheck_prevents_overflowing_append \
  --harness invalid_action_resume_preserves_slot_and_journal_model \
  --harness slot_bounds_model_distinguishes_valid_and_invalid_indices
```

## Result

- Status: PASS
- Tool: Kani 0.67.0
- Observed evidence: `Manual Harness Summary: Complete - 5 successfully verified harnesses, 0 failures, 5 total.`

## Harnesses

- `slot_bounds_model_distinguishes_valid_and_invalid_indices`
- `invalid_action_resume_preserves_slot_and_journal_model`
- `journal_capacity_precheck_prevents_overflowing_append`
- `no_contract_action_rejects_clean_output_from_tainted_input`
- `join_taint_is_monotonic_for_generated_lattice_model`

## Scope and Limits

This is bounded model evidence for the explicit harnesses in `crates/vb_codegen/src/kani_generated_runtime.rs`. It does not prove unbounded whole-program correctness or replace the TLA+/Verus layers.
