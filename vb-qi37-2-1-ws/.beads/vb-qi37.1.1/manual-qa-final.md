# Manual QA Final Report: vb-qi37.1.1

## Command

```bash
cargo nextest run --test vb_qi37_1_1_red_recovery_contract_test --no-fail-fast
```

## Output

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
────────────
 Nextest run ID 47f2eee4-2db9-4d51-9bd5-6c8c99fd3f4c with nextest profile: default
    Starting 19 tests across 1 binary
        PASS [   0.003s] ( 1/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test corrupt_slot_value_blocks_both_values_and_taint
        PASS [   0.003s] ( 2/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test ask_answer_records_exact_clean_taint_when_answer_writes_output
        PASS [   0.003s] ( 3/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test supported_seed_hydrates_exact_derived_taint
        PASS [   0.003s] ( 4/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test runtime_to_storage_mapping_preserves_taint_for_slot_write
        PASS [   0.003s] ( 5/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test event_only_recovery_keeps_slot_taint_supported_when_value_bytes_are_valid
        PASS [   0.003s] ( 6/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test no_output_step_summary_reports_zero_slots_written
        PASS [   0.003s] ( 7/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test missing_slot_value_blocks_both_values_and_taint
        PASS [   0.003s] ( 8/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test no_output_step_does_not_fabricate_slot_zero_dimension
        PASS [   0.003s] ( 9/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test supported_seed_hydrates_exact_secret_taint
        PASS [   0.004s] (10/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test deterministic_step_recovery_hydrates_exact_tainted_frame_when_slot_event_is_complete
        PASS [   0.004s] (11/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test no_output_step_recovery_has_no_recovered_slot_entries
        PASS [   0.004s] (12/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test recovery_does_not_default_missing_durable_taint_to_clean
        PASS [   0.004s] (13/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test event_only_recovery_returns_secret_i64_when_durable_taint_is_secret
        PASS [   0.004s] (14/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test action_completion_records_exact_secret_taint_when_action_writes_output
        PASS [   0.004s] (15/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test event_only_recovery_returns_derived_bool_when_durable_taint_is_derived
        PASS [   0.006s] (16/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test proptest_no_output_success_never_creates_slot_zero
        PASS [   0.006s] (17/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test proptest_event_only_slot_recovery_preserves_secret_taint
        PASS [   0.006s] (18/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test proptest_valid_slot_events_are_fully_hydrateable
        PASS [   0.011s] (19/19) velvet-ballastics-workspace::vb_qi37_1_1_red_recovery_contract_test drain_report_contract_requires_three_drained_and_three_written
────────────
     Summary [   0.011s] 19 tests run: 19 passed, 0 skipped
```

## Summary

- 19 tests run, 19 passed, 0 skipped
- Tests cover: taint preservation in slot writes, event-only recovery with valid/missing/corrupt taint, no-output step slot-zero non-fabrication, workflow digest mismatch, journal append failure propagation, and proptest invariants

STATUS: PASS
