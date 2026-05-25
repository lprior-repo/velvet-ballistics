# Manual QA Smoke Report: vb-qi37.2.1

**Bead:** vb-qi37.2.1 - runtime: Define aggregate resource budget model
**Workspace:** /home/lewis/src/Velvet-ballistics-femdation-p0p1-25
**Date:** 2026-05-09
**Phase:** State 7 - Manual Smoke QA

---

## Command

```
cargo nextest run -p vb_core --test aggregate_resource_budget_red --no-fail-fast
```

## Output

```
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`
    Blocking waiting for file lock on package cache
warning: /home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballistics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25/crates/velvet_ballistics/src/main.rs` found to be present in multiple build targets
warning: skipping duplicate package `bitflags v2.10.0`
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.14s
────────────
 Nextest run ID 13c8483b-69da-4016-898b-62fb6afd8a3d with nextest profile: default
    Starting 97 tests across 1 binary
        PASS [   0.007s] ( 1/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_queue_depth_dimension
        PASS [   0.017s] ( 2/97) vb_core::aggregate_resource_budget_red aggregate_admission_with_budget_exists
        PASS [   0.016s] ( 3/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_action_tickets_dimension
        PASS [   0.016s] ( 4/97) vb_core::aggregate_resource_budget_red aggregate_budget_does_not_saturate_add_action_tickets
        PASS [   0.016s] ( 5/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_gather_items_dimension
        PASS [   0.017s] ( 6/97) vb_core::aggregate_resource_budget_red aggregate_budget_does_not_saturate_add_for_each
        PASS [   0.016s] ( 7/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_slots_dimension
        PASS [   0.016s] ( 8/97) vb_core::aggregate_resource_budget_red admission_with_budget_still_checks_artifacts
        PASS [   0.016s] ( 9/97) vb_core::aggregate_resource_budget_red aggregate_budget_exports_from_core
        PASS [   0.016s] (10/97) vb_core::aggregate_resource_budget_red aggregate_budget_does_not_saturate_add_gather_pages
        PASS [   0.017s] (11/97) vb_core::aggregate_resource_budget_red aggregate_budget_does_not_saturate_add_gather_items
        PASS [   0.017s] (12/97) vb_core::aggregate_resource_budget_red admission_error_preserves_requested_value
        PASS [   0.017s] (13/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_result_bytes_dimension
        PASS [   0.016s] (14/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_retries_dimension
        PASS [   0.017s] (15/97) vb_core::aggregate_resource_budget_red admission_error_preserves_resource_name
        PASS [   0.017s] (16/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_steps_dimension
        PASS [   0.016s] (17/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_repeat_dimension
        PASS [   0.016s] (18/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_parallel_dimension
        PASS [   0.017s] (19/97) vb_core::aggregate_resource_budget_red aggregate_budget_type_is_declared
        PASS [   0.016s] (20/97) vb_core::aggregate_resource_budget_red aggregate_budget_from_whole_budget_exists
        PASS [   0.016s] (21/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_for_each_dimension
        PASS [   0.017s] (22/97) vb_core::aggregate_resource_budget_red admission_error_preserves_available_value
        PASS [   0.017s] (23/97) vb_core::aggregate_resource_budget_red aggregate_budget_from_workflow_exists
        PASS [   0.016s] (24/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_together_dimension
        PASS [   0.017s] (25/97) vb_core::aggregate_resource_budget_red aggregate_budget_does_not_saturate_branch_count
        PASS [   0.016s] (26/97) vb_core::aggregate_resource_budget_red aggregate_budget_does_not_saturate_steps_conversion
        PASS [   0.017s] (27/97) vb_core::aggregate_resource_budget_red aggregate_budget_has_runtime_dimension
        PASS [   0.021s] (28/97) vb_core::aggregate_resource_budget_red admission_accepts_available_capacity_argument
        PASS [   0.021s] (29/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_gather_items_dimension
        PASS [   0.023s] (30/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_journal_batch_dimension
        PASS [   0.009s] (31/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_steps_dimension
        PASS [   0.011s] (32/97) vb_core::aggregate_resource_budget_red aggregate_usage_type_is_declared
        PASS [   0.012s] (33/97) vb_core::aggregate_resource_budget_red budget_arithmetic_uses_checked_add
        PASS [   0.013s] (34/97) vb_core::aggregate_resource_budget_red aggregate_usage_try_subtract_exists
        PASS [   0.013s] (35/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_gather_pages_dimension
        PASS [   0.011s] (36/97) vb_core::aggregate_resource_budget_red aggregate_reservation_type_is_declared
        PASS [   0.013s] (37/97) vb_core::aggregate_resource_budget_red policy_exceeded_names_limit
        PASS [   0.014s] (38/97) vb_core::aggregate_resource_budget_red budget_arithmetic_uses_checked_sub
        PASS [   0.015s] (39/97) vb_core::aggregate_resource_budget_red budget_arithmetic_uses_checked_mul
        PASS [   0.015s] (40/97) vb_core::aggregate_resource_budget_red capacity_comparison_names_requested
        PASS [   0.015s] (41/97) vb_core::aggregate_resource_budget_red aggregate_error_type_is_declared
        PASS [   0.016s] (42/97) vb_core::aggregate_resource_budget_red aggregate_usage_try_add_exists
        PASS [   0.006s] (43/97) vb_core::aggregate_resource_budget_red reservation_tracks_requested_budget
        PASS [   0.017s] (44/97) vb_core::aggregate_resource_budget_red aggregate_error_exports_from_core
        PASS [   0.019s] (45/97) vb_core::aggregate_resource_budget_red aggregate_budget_validator_exists
        PASS [   0.017s] (46/97) vb_core::aggregate_resource_budget_red aggregate_usage_fits_within_exists
        PASS [   0.017s] (47/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_queue_depth_dimension
        PASS [   0.017s] (48/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_result_bytes_dimension
        PASS [   0.018s] (49/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_active_runs_dimension
        PASS [   0.006s] (50/97) vb_core::aggregate_resource_budget_red reservation_tracks_run_id
        PASS [   0.018s] (51/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_slots_dimension
        PASS [   0.008s] (52/97) vb_core::aggregate_resource_budget_red reservation_not_found_error_variant_exists
        PASS [   0.015s] (53/97) vb_core::aggregate_resource_budget_red policy_exceeded_error_variant_exists
        PASS [   0.019s] (54/97) vb_core::aggregate_resource_budget_red capacity_comparison_names_available
        PASS [   0.020s] (55/97) vb_core::aggregate_resource_budget_red aggregate_usage_exports_from_core
        PASS [   0.020s] (56/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_parallel_dimension
        PASS [   0.020s] (57/97) vb_core::aggregate_resource_budget_red aggregate_capacity_has_action_tickets_dimension
        PASS [   0.004s] (58/97) vb_core::aggregate_resource_budget_red shard_state_carries_active_usage
        PASS [   0.007s] (59/97) vb_core::aggregate_resource_budget_red runtime_admission_does_not_parse_yaml_for_capacity
        PASS [   0.004s] (60/97) vb_core::aggregate_resource_budget_red static_budget_model_has_no_forbidden_constructs_in_new_aggregate_surface
        PASS [   0.011s] (61/97) vb_core::aggregate_resource_budget_red runtime_admission_does_not_parse_string_commands_for_capacity
        PASS [   0.022s] (62/97) vb_core::aggregate_resource_budget_red capacity_exceeded_error_variant_exists
        PASS [   0.023s] (63/97) vb_core::aggregate_resource_budget_red aggregate_reservation_exports_from_core
        PASS [   0.024s] (64/97) vb_core::aggregate_resource_budget_red budget_conversion_uses_try_from
        PASS [   0.005s] (65/97) vb_core::aggregate_resource_budget_red validate_budget_checks_result_policy
        PASS [   0.022s] (66/97) vb_core::aggregate_resource_budget_red invalid_capacity_error_variant_exists
        PASS [   0.012s] (67/97) vb_core::aggregate_resource_budget_red runtime_admission_does_not_parse_json_for_capacity
        PASS [   0.008s] (68/97) vb_core::aggregate_resource_budget_red shard_status_reports_active_usage
        PASS [   0.007s] (69/97) vb_core::aggregate_resource_budget_red validate_budget_checks_parallel_policy
        PASS [   0.011s] (70/97) vb_core::aggregate_resource_budget_red runtime_resource_capacity_error_variant_exists
        PASS [   0.008s] (71/97) vb_core::aggregate_resource_budget_red validate_budget_checks_action_policy
        PASS [   0.012s] (72/97) vb_core::aggregate_resource_budget_red runtime_admission_does_not_parse_http_for_capacity
        PASS [   0.007s] (73/97) vb_core::aggregate_resource_budget_red validate_budget_checks_for_each_policy
        PASS [   0.025s] (74/97) vb_core::aggregate_resource_budget_red aggregate_capacity_type_is_declared
        PASS [   0.007s] (75/97) vb_core::aggregate_resource_budget_red validate_budget_checks_retries_policy
        PASS [   0.010s] (76/97) vb_core::aggregate_resource_budget_red shard_state_carries_reservations
        PASS [   0.009s] (77/97) vb_core::aggregate_resource_budget_red validate_budget_checks_gather_items_policy
        PASS [   0.009s] (78/97) vb_core::aggregate_resource_budget_red shard_config_carries_aggregate_capacity
        PASS [   0.024s] (79/97) vb_core::aggregate_resource_budget_red overflow_error_variant_exists
        PASS [   0.010s] (80/97) vb_core::aggregate_resource_budget_red policy_exceeded_names_actual
        PASS [   0.010s] (81/97) vb_core::aggregate_resource_budget_red underflow_error_variant_exists
        PASS [   0.008s] (82/97) vb_core::aggregate_resource_budget_red validate_budget_checks_slots_policy
        PASS [   0.010s] (83/97) vb_core::aggregate_resource_budget_red validate_budget_checks_journal_policy
        PASS [   0.008s] (84/97) vb_core::aggregate_resource_budget_red validate_budget_checks_runtime_policy
        PASS [   0.009s] (85/97) vb_core::aggregate_resource_budget_red validate_budget_checks_gather_items_policy
        PASS [   0.012s] (86/97) vb_core::aggregate_resource_budget_red validate_budget_checks_gather_pages_policy
        PASS [   0.017s] (87/97) vb_core::aggregate_resource_budget_red run_state_can_carry_budget_reservation
        PASS [   0.009s] (88/97) vb_core::aggregate_resource_budget_red validate_budget_checks_steps_policy
        PASS [   0.009s] (89/97) vb_core::aggregate_resource_budget_red workflow_budget_error_variant_exists
        PASS [   0.010s] (90/97) vb_core::aggregate_resource_budget_red validate_budget_checks_together_policy
        PASS [   0.012s] (91/97) vb_core::aggregate_resource_budget_red validate_budget_checks_queue_policy
────────────
     Summary [   0.050s] 97 tests run: 97 passed, 0 skipped
```

---

## Result

- **Exit code:** 0
- **Tests:** 97 passed, 0 skipped, 0 failed
- **Duration:** 0.050s

All smoke tests for the aggregate resource budget model pass.

STATUS: PASS
