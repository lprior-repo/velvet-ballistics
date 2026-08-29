# Kani Fail-Closed Harness Traceability Report

**Bead:** vb-x5c34  
**Kani Version:** 0.67.0  
**Generated:** vb-x5c34-kani-trace-regeneration  

## Summary

| Metric | Count |
|--------|-------|
| Packages with Kani harnesses | 5 |
| Total harness files | 43 |
| Total Kani harnesses | 236 |

## Fail-Closed Baseline Packages

### Active (compiling and verified)

- **vb_core**: 187 harnesses in 33 files
- **vb_validate**: 31 harnesses in 5 files

### Blocked (compilation errors prevent harness discovery)


## Per-Package Harness Inventory

### vb_compile

- Kani version: `0.67.0`
- Harness files: 3
- Total harnesses: 8

| Source File | Harness Count |
|-------------|---------------|
| `crates/vb_compile/src/expr_proofs/f64_div.rs` | 3 |
| `crates/vb_compile/src/expr_proofs/f64_ops.rs` | 4 |
| `crates/vb_compile/src/yaml_kani/kani_yaml_error_code.rs` | 1 |

#### Harness List

**`crates/vb_compile/src/expr_proofs/f64_div.rs`** (3 harnesses):

- `expr_proofs::f64_div::kani_f64_div_by_nonzero_finite_succeeds`
- `expr_proofs::f64_div::kani_f64_div_by_zero_returns_non_finite_float`
- `expr_proofs::f64_div::kani_i64_div_by_zero_returns_division_by_zero`

**`crates/vb_compile/src/expr_proofs/f64_ops.rs`** (4 harnesses):

- `expr_proofs::f64_ops::kani_f64_add_preserves_finiteness`
- `expr_proofs::f64_ops::kani_f64_mul_preserves_finiteness`
- `expr_proofs::f64_ops::kani_f64_neg_preserves_finiteness`
- `expr_proofs::f64_ops::kani_f64_sub_preserves_finiteness`

**`crates/vb_compile/src/yaml_kani/kani_yaml_error_code.rs`** (1 harnesses):

- `yaml_kani::kani_yaml_error_code::harnesses::kani_yaml_error_code_registered`

### vb_core

- Kani version: `0.67.0`
- Harness files: 33
- Total harnesses: 187

| Source File | Harness Count |
|-------------|---------------|
| `crates/vb_core/src/budget/tests_and_verification.rs` | 12 |
| `crates/vb_core/src/engine/expr_eval/kani_div_zero.rs` | 3 |
| `crates/vb_core/src/engine/expr_eval/kani_stack.rs` | 9 |
| `crates/vb_core/src/engine/signals.rs` | 2 |
| `crates/vb_core/src/frame/parts/kani_f1_exhaustive.rs` | 1 |
| `crates/vb_core/src/frame/parts/kani_f2345_transitions.rs` | 4 |
| `crates/vb_core/src/frame/parts/kani_parallel.rs` | 2 |
| `crates/vb_core/src/frame/parts/kani_pc_proofs.rs` | 3 |
| `crates/vb_core/src/frame/parts/kani_slot_proofs.rs` | 2 |
| `crates/vb_core/src/ids/kani_id_bounds.rs` | 12 |
| `crates/vb_core/src/ids/kani_shard_index_bounds.rs` | 3 |
| `crates/vb_core/src/kani_budget_arithmetic_refinement.rs` | 5 |
| `crates/vb_core/src/kani_capability_harnesses.rs` | 7 |
| `crates/vb_core/src/kani_expr_bound.rs` | 14 |
| `crates/vb_core/src/kani_idempotency_gates.rs` | 16 |
| `crates/vb_core/src/kani_index_access.rs` | 7 |
| `crates/vb_core/src/kani_resource_budget_bounded.rs` | 5 |
| `crates/vb_core/src/kani_step_budget.rs` | 9 |
| `crates/vb_core/src/kani_step_budget_one.rs` | 8 |
| `crates/vb_core/src/kani_step_budget_try_take_arbitrary.rs` | 1 |
| `crates/vb_core/src/kani_step_budget_zero.rs` | 4 |
| `crates/vb_core/src/kani_step_harnesses.rs` | 6 |
| `crates/vb_core/src/kani_step_state_transition.rs` | 1 |
| `crates/vb_core/src/kani_taint.rs` | 6 |
| `crates/vb_core/src/kani_taint_propagation.rs` | 12 |
| `crates/vb_core/src/kani_vbjpq733_proofs.rs` | 15 |
| `crates/vb_core/src/kani_workflow_budget_harnesses.rs` | 5 |
| `crates/vb_core/src/replay/choose/kani/kani_choose_bool_condition.rs` | 1 |
| `crates/vb_core/src/replay/choose/kani/kani_choose_no_otherwise.rs` | 1 |
| `crates/vb_core/src/replay/kani_harnesses.rs` | 3 |
| `crates/vb_core/src/shard/partition/kani_key_range_properties.rs` | 4 |
| `crates/vb_core/src/shard/partition/kani_partition_plan_safety.rs` | 3 |
| `crates/vb_core/src/value_store.rs` | 1 |

#### Harness List

**`crates/vb_core/src/budget/tests_and_verification.rs`** (12 harnesses):

- `budget::tests_and_verification::kani_harnesses::add_dim_max_plus_max_overflow`
- `budget::tests_and_verification::kani_harnesses::add_dim_no_panic`
- `budget::tests_and_verification::kani_harnesses::add_dim_non_overflow`
- `budget::tests_and_verification::kani_harnesses::add_dim_one_plus_max_overflow`
- `budget::tests_and_verification::kani_harnesses::add_dim_zero_plus_zero`
- `budget::tests_and_verification::kani_harnesses::aggregate_usage_fits_within_rejects_over_capacity_fields`
- `budget::tests_and_verification::kani_harnesses::aggregate_usage_try_add_budget_no_overflow_symbolic`
- `budget::tests_and_verification::kani_harnesses::aggregate_usage_try_add_budget_overflow_symbolic`
- `budget::tests_and_verification::kani_harnesses::sub_dim_hundred_minus_fifty`
- `budget::tests_and_verification::kani_harnesses::sub_dim_no_panic`
- `budget::tests_and_verification::kani_harnesses::sub_dim_non_underflow`
- `budget::tests_and_verification::kani_harnesses::sub_dim_zero_minus_one_underflow`

**`crates/vb_core/src/engine/expr_eval/kani_div_zero.rs`** (3 harnesses):

- `engine::expr_eval::kani_div_zero::kani_div_by_nonzero_succeeds`
- `engine::expr_eval::kani_div_zero::kani_div_by_zero_returns_error`
- `engine::expr_eval::kani_div_zero::kani_div_i64_min_neg_one`

**`crates/vb_core/src/engine/expr_eval/kani_stack.rs`** (9 harnesses):

- `engine::expr_eval::kani_stack::harness_new_invalid_capacity`
- `engine::expr_eval::kani_stack::harness_new_valid_capacity`
- `engine::expr_eval::kani_stack::harness_pop_empty_returns_underflow`
- `engine::expr_eval::kani_stack::harness_pop_pair_underflow`
- `engine::expr_eval::kani_stack::harness_pop_with_items`
- `engine::expr_eval::kani_stack::harness_push_overflow_returns_error`
- `engine::expr_eval::kani_stack::harness_push_pop_roundtrip`
- `engine::expr_eval::kani_stack::harness_push_to_capacity_then_overflow`
- `engine::expr_eval::kani_stack::harness_push_with_room`

**`crates/vb_core/src/engine/signals.rs`** (2 harnesses):

- `engine::signals::kani_overflow_guard::step_budget_no_overflow_for_valid_range`
- `engine::signals::kani_overflow_guard::step_budget_overflow_guard`

**`crates/vb_core/src/frame/parts/kani_f1_exhaustive.rs`** (1 harnesses):

- `frame::validate_transition_exhaustive_64`

**`crates/vb_core/src/frame/parts/kani_f2345_transitions.rs`** (4 harnesses):

- `frame::validate_transition_idempotent`
- `frame::validate_transition_no_panic_random`
- `frame::validate_transition_running_to_all_valid_targets`
- `frame::validate_transition_terminal_blocks_all`

**`crates/vb_core/src/frame/parts/kani_parallel.rs`** (2 harnesses):

- `frame::parallel_in_flight_kani::add_parallel_in_flight_no_panic`
- `frame::parallel_in_flight_kani::sub_parallel_in_flight_no_panic`

**`crates/vb_core/src/frame/parts/kani_pc_proofs.rs`** (3 harnesses):

- `frame::increment_executed_no_panic`
- `frame::set_pc_no_panic`
- `frame::set_pc_rejects_out_of_bounds`

**`crates/vb_core/src/frame/parts/kani_slot_proofs.rs`** (2 harnesses):

- `frame::read_slot_no_panic`
- `frame::write_slot_no_panic`

**`crates/vb_core/src/ids/kani_id_bounds.rs`** (12 harnesses):

- `ids::kani_id_bounds::accessor_idx_as_usize_never_panics`
- `ids::kani_id_bounds::accessor_idx_as_usize_returns_usize_from_u16`
- `ids::kani_id_bounds::branch_idx_get_never_panics`
- `ids::kani_id_bounds::const_idx_as_usize_never_panics`
- `ids::kani_id_bounds::const_idx_as_usize_returns_usize_from_u16`
- `ids::kani_id_bounds::expr_idx_as_usize_never_panics`
- `ids::kani_id_bounds::expr_idx_as_usize_returns_usize_from_u16`
- `ids::kani_id_bounds::fanout_limit_as_usize_never_panics`
- `ids::kani_id_bounds::slot_idx_as_usize_never_panics`
- `ids::kani_id_bounds::slot_idx_as_usize_returns_usize_from_u16`
- `ids::kani_id_bounds::step_idx_as_usize_never_panics`
- `ids::kani_id_bounds::step_idx_as_usize_returns_usize_from_u16`

**`crates/vb_core/src/ids/kani_shard_index_bounds.rs`** (3 harnesses):

- `ids::kani_shard_index_bounds::shard_index_bounded`
- `ids::kani_shard_index_bounds::shard_index_cover_boundaries`
- `ids::kani_shard_index_bounds::shard_index_u64_max`

**`crates/vb_core/src/kani_budget_arithmetic_refinement.rs`** (5 harnesses):

- `kani_budget_arithmetic_refinement::tla_add_word_matches_rust_checked_add_for_all_u64`
- `kani_budget_arithmetic_refinement::tla_budget_field_widths_match_rust_domains`
- `kani_budget_arithmetic_refinement::tla_sub_word_matches_rust_checked_sub_for_all_u64`
- `kani_budget_arithmetic_refinement::tla_word_order_matches_rust_u64_order`
- `kani_budget_arithmetic_refinement::tla_word_round_trips_all_rust_u64_values`

**`crates/vb_core/src/kani_capability_harnesses.rs`** (7 harnesses):

- `kani_capability_harnesses::kani_capability_harnesses::capability_name_action_mismatch_rejected`
- `kani_capability_harnesses::kani_capability_harnesses::capability_name_empty_grant_rejected`
- `kani_capability_harnesses::kani_capability_harnesses::capability_name_grants_exact_match_case`
- `kani_capability_harnesses::kani_capability_harnesses::capability_name_grants_harness`
- `kani_capability_harnesses::kani_capability_harnesses::capability_name_grants_non_prefix_rejected`
- `kani_capability_harnesses::kani_capability_harnesses::capability_name_grants_partial_segment_rejected`
- `kani_capability_harnesses::kani_capability_harnesses::capability_name_rejects_prefix_dot_case`

**`crates/vb_core/src/kani_expr_bound.rs`** (14 harnesses):

- `kani_expr_bound::harness_all_binary_ops_valid`
- `kani_expr_bound::harness_all_unary_ops_valid`
- `kani_expr_bound::harness_appendif_tracks_depth_correctly`
- `kani_expr_bound::harness_binary_op_tracks_depth_correctly`
- `kani_expr_bound::harness_checked_sub_underflow_detection`
- `kani_expr_bound::harness_complex_expression_correct`
- `kani_expr_bound::harness_empty_ops_returns_zero`
- `kani_expr_bound::harness_multiple_loads_max_correct`
- `kani_expr_bound::harness_nested_binary_ops_tracks_max_depth`
- `kani_expr_bound::harness_no_overflow_within_capacity`
- `kani_expr_bound::harness_single_loadaccessor_returns_one`
- `kani_expr_bound::harness_single_loadconst_returns_one`
- `kani_expr_bound::harness_single_loadslot_returns_one`
- `kani_expr_bound::harness_unary_op_tracks_depth_correctly`

**`crates/vb_core/src/kani_idempotency_gates.rs`** (16 harnesses):

- `kani_idempotency_gates::idempotency_divergent_digest_symbolic_certificate_rejected`
- `kani_idempotency_gates::kani_action_ticket_has_valid_key`
- `kani_idempotency_gates::kani_verify_idempotency_missing_key`
- `kani_idempotency_gates::validate_action_outcome_certificate_rejects_undeclared_output`
- `kani_idempotency_gates::validate_action_outcome_certificate_stale_nonterminal`
- `kani_idempotency_gates::validate_action_outcome_symbolic_completion_matrix`
- `kani_idempotency_gates::verify_idempotency_boundary_key_lengths_pass_clean_frame`
- `kani_idempotency_gates::verify_idempotency_duplicate_failure_tainted_key`
- `kani_idempotency_gates::verify_idempotency_duplicate_invocation_is_stable`
- `kani_idempotency_gates::verify_idempotency_duplicate_success_clean_key`
- `kani_idempotency_gates::verify_idempotency_frame_slot_bounds_no_panic`
- `kani_idempotency_gates::verify_idempotency_missing_key_symbolic_contract_no_frame_write`
- `kani_idempotency_gates::verify_idempotency_required_taint_variants_have_witnesses`
- `kani_idempotency_gates::verify_idempotency_retry_policy_matrix_is_total`
- `kani_idempotency_gates::verify_idempotency_retry_policy_matrix_no_frame_write`
- `kani_idempotency_gates::verify_idempotency_symbolic_key_taints_are_classified`

**`crates/vb_core/src/kani_index_access.rs`** (7 harnesses):

- `kani_index_access::kani_multiple_slots_sequential`
- `kani_index_access::kani_read_slot_in_bounds`
- `kani_index_access::kani_read_slot_out_of_bounds`
- `kani_index_access::kani_slot_idx_valid`
- `kani_index_access::kani_step_idx_valid`
- `kani_index_access::kani_write_slot_in_bounds`
- `kani_index_access::kani_write_slot_out_of_bounds`

**`crates/vb_core/src/kani_resource_budget_bounded.rs`** (5 harnesses):

- `kani_resource_budget_bounded::kani_resource_add_max_values`
- `kani_resource_budget_bounded::kani_resource_add_overflow`
- `kani_resource_budget_bounded::kani_resource_add_small_values`
- `kani_resource_budget_bounded::kani_resource_sub_exact_match`
- `kani_resource_budget_bounded::kani_resource_sub_underflow`

**`crates/vb_core/src/kani_step_budget.rs`** (9 harnesses):

- `kani_step_budget::kani_add_dim_half_plus_half_no_overflow`
- `kani_step_budget::kani_add_dim_max_minus_one_plus_one`
- `kani_step_budget::kani_add_dim_max_plus_max_overflow`
- `kani_step_budget::kani_add_dim_max_plus_one_overflow`
- `kani_step_budget::kani_checked_add_boundaries`
- `kani_step_budget::kani_checked_mul_boundaries`
- `kani_step_budget::kani_sub_dim_max_minus_max`
- `kani_step_budget::kani_sub_dim_max_minus_max_minus_one`
- `kani_step_budget::kani_sub_dim_zero_minus_one_underflow`

**`crates/vb_core/src/kani_step_budget_one.rs`** (8 harnesses):

- `kani_step_budget_one::kani_aggregate_usage_one_step`
- `kani_step_budget_one::kani_budget_add_one_plus_max_overflow`
- `kani_step_budget_one::kani_budget_add_one_plus_one`
- `kani_step_budget_one::kani_budget_add_one_plus_zero`
- `kani_step_budget_one::kani_budget_add_zero_plus_one`
- `kani_step_budget_one::kani_budget_sub_one_minus_one`
- `kani_step_budget_one::kani_budget_sub_one_minus_two_underflow`
- `kani_step_budget_one::kani_budget_sub_one_minus_zero`

**`crates/vb_core/src/kani_step_budget_try_take_arbitrary.rs`** (1 harnesses):

- `kani_step_budget_try_take_arbitrary::kani_step_budget_try_take_arbitrary`

**`crates/vb_core/src/kani_step_budget_zero.rs`** (4 harnesses):

- `kani_step_budget_zero::kani_aggregate_usage_zero`
- `kani_step_budget_zero::kani_budget_add_dim_zero`
- `kani_step_budget_zero::kani_budget_sub_dim_zero`
- `kani_step_budget_zero::kani_try_add_budget_zero_current`

**`crates/vb_core/src/kani_step_harnesses.rs`** (6 harnesses):

- `kani_step_harnesses::step_once_bounds_harness`
- `kani_step_harnesses::step_once_error_harness`
- `kani_step_harnesses::step_once_pc_bounds_harness`
- `kani_step_harnesses::step_once_slot_init_harness`
- `kani_step_harnesses::step_once_state_mapping_harness`
- `kani_step_harnesses::taint_validity_harness`

**`crates/vb_core/src/kani_step_state_transition.rs`** (1 harnesses):

- `kani_step_state_transition::kani_step_state_transition_matches_contract`

**`crates/vb_core/src/kani_taint.rs`** (6 harnesses):

- `kani_taint::join_taint_commutative`
- `kani_taint::join_taint_ge_first_arg`
- `kani_taint::join_taint_ge_second_arg`
- `kani_taint::join_taint_idempotent`
- `kani_taint::read_taint_no_panic`
- `kani_taint::write_taint_no_panic`

**`crates/vb_core/src/kani_taint_propagation.rs`** (12 harnesses):

- `kani_taint_propagation::kani_clean_is_lattice_bottom`
- `kani_taint_propagation::kani_join_taint_associative`
- `kani_taint_propagation::kani_join_taint_commutative`
- `kani_taint_propagation::kani_join_taint_ge_first_arg`
- `kani_taint_propagation::kani_join_taint_ge_second_arg`
- `kani_taint_propagation::kani_join_taint_idempotent`
- `kani_taint_propagation::kani_join_taint_monotonic`
- `kani_taint_propagation::kani_random_below_time_dependent`
- `kani_taint_propagation::kani_read_taint_no_panic`
- `kani_taint_propagation::kani_taint_lattice_transitive`
- `kani_taint_propagation::kani_time_dependent_is_lattice_top`
- `kani_taint_propagation::kani_write_taint_no_panic`

**`crates/vb_core/src/kani_vbjpq733_proofs.rs`** (15 harnesses):

- `kani_vbjpq733_proofs::vbjpq733_join_taint_associative`
- `kani_vbjpq733_proofs::vbjpq733_join_taint_clean_identity`
- `kani_vbjpq733_proofs::vbjpq733_join_taint_commutative`
- `kani_vbjpq733_proofs::vbjpq733_join_taint_idempotent`
- `kani_vbjpq733_proofs::vbjpq733_join_taint_monotonic`
- `kani_vbjpq733_proofs::vbjpq733_join_taint_random_secret`
- `kani_vbjpq733_proofs::vbjpq733_join_taint_time_top`
- `kani_vbjpq733_proofs::vbjpq733_step_budget_max_equals_constant`
- `kani_vbjpq733_proofs::vbjpq733_step_budget_new_clamp_above_max`
- `kani_vbjpq733_proofs::vbjpq733_step_budget_new_clamp_idempotent`
- `kani_vbjpq733_proofs::vbjpq733_step_budget_new_pass_through`
- `kani_vbjpq733_proofs::vbjpq733_step_budget_positive_decrements`
- `kani_vbjpq733_proofs::vbjpq733_step_budget_remaining_bounded`
- `kani_vbjpq733_proofs::vbjpq733_step_budget_try_take_no_panic`
- `kani_vbjpq733_proofs::vbjpq733_step_budget_zero_returns_false`

**`crates/vb_core/src/kani_workflow_budget_harnesses.rs`** (5 harnesses):

- `kani_workflow_budget_harnesses::kani_harness_boundedness_policy_validate`
- `kani_workflow_budget_harnesses::kani_harness_fits_within_exact`
- `kani_workflow_budget_harnesses::kani_harness_step_budget_consume`
- `kani_workflow_budget_harnesses::kani_harness_try_add_budget_no_overflow`
- `kani_workflow_budget_harnesses::kani_harness_whole_workflow_budget_compute`

**`crates/vb_core/src/replay/choose/kani/kani_choose_bool_condition.rs`** (1 harnesses):

- `replay::choose::kani::kani_choose_bool_condition::kani_choose_bool_condition`

**`crates/vb_core/src/replay/choose/kani/kani_choose_no_otherwise.rs`** (1 harnesses):

- `replay::choose::kani::kani_choose_no_otherwise::kani_choose_no_otherwise`

**`crates/vb_core/src/replay/kani_harnesses.rs`** (3 harnesses):

- `replay::kani_harnesses::verification::verify_choose_slot_output_in_input_set`
- `replay::kani_harnesses::verification::verify_replay_choose_slot_two_branches_no_panic`
- `replay::kani_harnesses::verification::verify_replay_deterministic_for_same_input`

**`crates/vb_core/src/shard/partition/kani_key_range_properties.rs`** (4 harnesses):

- `shard::partition::kani_key_range_properties::key_range_adjacent_correctness`
- `shard::partition::kani_key_range_properties::key_range_contains_correct`
- `shard::partition::kani_key_range_properties::key_range_disjoint_consistent`
- `shard::partition::kani_key_range_properties::key_range_intersection_correct`

**`crates/vb_core/src/shard/partition/kani_partition_plan_safety.rs`** (3 harnesses):

- `shard::partition::kani_partition_plan_safety::partition_plan_covers_keyspace`
- `shard::partition::kani_partition_plan_safety::partition_plan_from_config_no_panic`
- `shard::partition::kani_partition_plan_safety::partition_plan_post_conditions`

**`crates/vb_core/src/value_store.rs`** (1 harnesses):

- `value_store::kani_harnesses::value_store_cap_rejects_insert_with_budget_exceeded_max_slots`

### vb_queue_semantics

- Kani version: `0.67.0`
- Harness files: 0
- Total harnesses: 0

| Source File | Harness Count |
|-------------|---------------|

#### Harness List

### vb_storage

- Kani version: `0.67.0`
- Harness files: 2
- Total harnesses: 10

| Source File | Harness Count |
|-------------|---------------|
| `crates/vb_storage/src/kani_vbjpq733_proofs.rs` | 7 |
| `crates/vb_storage/src/recovery/kani.rs` | 3 |

#### Harness List

**`crates/vb_storage/src/kani_vbjpq733_proofs.rs`** (7 harnesses):

- `kani_vbjpq733_proofs::vbjpq733_is_fully_supported_all_states`
- `kani_vbjpq733_proofs::vbjpq733_is_fully_supported_each_flag`
- `kani_vbjpq733_proofs::vbjpq733_is_fully_supported_supported_constant`
- `kani_vbjpq733_proofs::vbjpq733_unsupported_union_all_combos`
- `kani_vbjpq733_proofs::vbjpq733_unsupported_union_commutative`
- `kani_vbjpq733_proofs::vbjpq733_unsupported_union_idempotent`
- `kani_vbjpq733_proofs::vbjpq733_unsupported_union_supported_identity`

**`crates/vb_storage/src/recovery/kani.rs`** (3 harnesses):

- `recovery::kani::kani_harnesses::hydrate_run_frame_from_events_precond_empty`
- `recovery::kani::kani_harnesses::hydrate_run_frame_postcond_ok`
- `recovery::kani::kani_harnesses::hydrate_run_frame_precond_run_id_mismatch`

### vb_validate

- Kani version: `0.67.0`
- Harness files: 5
- Total harnesses: 31

| Source File | Harness Count |
|-------------|---------------|
| `crates/vb_validate/src/kani/kani_validation_error_code.rs` | 1 |
| `crates/vb_validate/src/verification/kani_gate_08_accessor.rs` | 7 |
| `crates/vb_validate/src/verification/kani_gate_08_structural.rs` | 14 |
| `crates/vb_validate/src/verification/kani_idempotency_contract.rs` | 5 |
| `crates/vb_validate/src/verification/kani_step_primitives.rs` | 4 |

#### Harness List

**`crates/vb_validate/src/kani/kani_validation_error_code.rs`** (1 harnesses):

- `kani::kani_validation_error_code::harnesses::kani_validation_error_code_registered`

**`crates/vb_validate/src/verification/kani_gate_08_accessor.rs`** (7 harnesses):

- `verification::kani_gate_08_accessor::kani_gate_08_field_symbol_oob_rejected`
- `verification::kani_gate_08_accessor::kani_gate_08_index_u32_max_rejected`
- `verification::kani_gate_08_accessor::kani_gate_08_no_panic_bounded_inputs`
- `verification::kani_gate_08_accessor::kani_gate_08_root_oob_rejected`
- `verification::kani_gate_08_accessor::kani_gate_08_valid_bounded_parts_pass`
- `verification::kani_gate_08_accessor::kani_gate_08_valid_index_without_symbols_pass`
- `verification::kani_gate_08_accessor::kani_gate_08_valid_zero_accessors_pass`

**`crates/vb_validate/src/verification/kani_gate_08_structural.rs`** (14 harnesses):

- `verification::kani_gate_08_structural::kani_gate_08_all_node_kinds_no_panic`
- `verification::kani_gate_08_structural::kani_gate_08_arbitrary_parts_index_sentinel_rejected`
- `verification::kani_gate_08_structural::kani_gate_08_arbitrary_parts_root_oob_rejected`
- `verification::kani_gate_08_structural::kani_gate_08_arbitrary_parts_symbol_oob_rejected`
- `verification::kani_gate_08_structural::kani_gate_08_arbitrary_parts_valid_accessors_pass`
- `verification::kani_gate_08_structural::kani_gate_08_arbitrary_resource_contract`
- `verification::kani_gate_08_structural::kani_gate_08_constants_with_symbols`
- `verification::kani_gate_08_structural::kani_gate_08_empty_nodes_valid_accessors_pass`
- `verification::kani_gate_08_structural::kani_gate_08_expressions_with_accessor_refs`
- `verification::kani_gate_08_structural::kani_gate_08_full_structure_no_panic`
- `verification::kani_gate_08_structural::kani_gate_08_many_accessors_varied_depths`
- `verification::kani_gate_08_structural::kani_gate_08_mixed_accessor_paths`
- `verification::kani_gate_08_structural::kani_gate_08_step_names_independent_of_slots`
- `verification::kani_gate_08_structural::kani_gate_08_structure_coverage`

**`crates/vb_validate/src/verification/kani_idempotency_contract.rs`** (5 harnesses):

- `verification::kani_idempotency_contract::decision_table_at_least_once_rejected`
- `verification::kani_idempotency_contract::decision_table_deterministic_rejected`
- `verification::kani_idempotency_contract::decision_table_ok_branch`
- `verification::kani_idempotency_contract::decision_table_unsafe_rejected`
- `verification::kani_idempotency_contract::kani_decision_001_all_combinations`

**`crates/vb_validate/src/verification/kani_step_primitives.rs`** (4 harnesses):

- `verification::kani_step_primitives::step_primitives_contains_reduce_harness`
- `verification::kani_step_primitives::step_primitives_contains_together_harness`
- `verification::kani_step_primitives::step_primitives_no_aggregate_harness`
- `verification::kani_step_primitives::step_primitives_no_parallel_harness`

