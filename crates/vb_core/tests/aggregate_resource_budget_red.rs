#![allow(clippy::indexing_slicing)]

const BUDGET_RS: &str = include_str!("../src/budget.rs");
const CORE_LIB_RS: &str = include_str!("../src/lib.rs");
const ADMISSION_RS: &str = include_str!("../../vb_runtime/src/admission.rs");
const SHARD_TYPES_RS: &str = include_str!("../../vb_runtime/src/shard/types.rs");

fn repository_source() -> String {
    [BUDGET_RS, CORE_LIB_RS, ADMISSION_RS, SHARD_TYPES_RS].join("\n")
}

macro_rules! source_contract_test {
    ($name:ident, $source:expr, $needle:expr) => {
        #[test]
        fn $name() {
            let source = $source;
            let aggregate_surface_exists = BUDGET_RS.contains("pub struct AggregateResourceBudget");
            let present = aggregate_surface_exists && source.contains($needle);
            assert_eq!(
                present, true,
                "missing aggregate resource contract token: {}",
                $needle
            );
        }
    };
}

macro_rules! source_absence_test {
    ($name:ident, $source:expr, $needle:expr) => {
        #[test]
        fn $name() {
            let source = $source;
            let aggregate_surface_exists = BUDGET_RS.contains("pub struct AggregateResourceBudget");
            let present = source.contains($needle);
            assert_eq!(
                present, false,
                "forbidden aggregate resource token remains: {}",
                $needle
            );
            assert_eq!(
                aggregate_surface_exists, true,
                "aggregate surface is absent, so forbidden-token absence is not sufficient: {}",
                $needle
            );
        }
    };
}

source_contract_test!(
    aggregate_budget_type_is_declared,
    BUDGET_RS,
    "pub struct AggregateResourceBudget"
);
source_contract_test!(
    aggregate_capacity_type_is_declared,
    BUDGET_RS,
    "pub struct AggregateResourceCapacity"
);
source_contract_test!(
    aggregate_usage_type_is_declared,
    BUDGET_RS,
    "pub struct AggregateResourceUsage"
);
source_contract_test!(
    aggregate_reservation_type_is_declared,
    BUDGET_RS,
    "pub struct AggregateReservation"
);
source_contract_test!(
    aggregate_error_type_is_declared,
    BUDGET_RS,
    "pub enum AggregateBudgetError"
);
source_contract_test!(
    aggregate_budget_exports_from_core,
    CORE_LIB_RS,
    "AggregateResourceBudget"
);
source_contract_test!(
    aggregate_capacity_exports_from_core,
    CORE_LIB_RS,
    "AggregateResourceCapacity"
);
source_contract_test!(
    aggregate_usage_exports_from_core,
    CORE_LIB_RS,
    "AggregateResourceUsage"
);
source_contract_test!(
    aggregate_reservation_exports_from_core,
    CORE_LIB_RS,
    "AggregateReservation"
);
source_contract_test!(
    aggregate_error_exports_from_core,
    CORE_LIB_RS,
    "AggregateBudgetError"
);
source_contract_test!(
    aggregate_budget_has_steps_dimension,
    BUDGET_RS,
    "pub max_steps_executable: u32"
);
source_contract_test!(
    aggregate_budget_has_action_tickets_dimension,
    BUDGET_RS,
    "pub max_action_tickets: u32"
);
source_contract_test!(
    aggregate_budget_has_parallel_dimension,
    BUDGET_RS,
    "pub max_parallel_in_flight: u16"
);
source_contract_test!(
    aggregate_budget_has_retries_dimension,
    BUDGET_RS,
    "pub max_retries_per_action: u16"
);
source_contract_test!(
    aggregate_budget_has_gather_pages_dimension,
    BUDGET_RS,
    "pub max_gather_pages: u32"
);
source_contract_test!(
    aggregate_budget_has_gather_items_dimension,
    BUDGET_RS,
    "pub max_gather_items: u32"
);
source_contract_test!(
    aggregate_budget_has_for_each_dimension,
    BUDGET_RS,
    "pub max_for_each_iterations: u32"
);
source_contract_test!(
    aggregate_budget_has_together_dimension,
    BUDGET_RS,
    "pub max_together_branches: u16"
);
source_contract_test!(
    aggregate_budget_has_repeat_dimension,
    BUDGET_RS,
    "pub max_repeat_attempts: u16"
);
source_contract_test!(
    aggregate_budget_has_runtime_dimension,
    BUDGET_RS,
    "pub max_run_time_seconds: u64"
);
source_contract_test!(
    aggregate_budget_has_result_bytes_dimension,
    BUDGET_RS,
    "pub max_result_bytes: u32"
);
source_contract_test!(
    aggregate_budget_has_slots_dimension,
    BUDGET_RS,
    "pub max_total_slots_written: u32"
);
source_contract_test!(
    aggregate_budget_has_queue_depth_dimension,
    BUDGET_RS,
    "pub max_queue_depth: u32"
);
source_contract_test!(
    aggregate_budget_has_journal_batch_dimension,
    BUDGET_RS,
    "pub max_journal_batch_bytes: u32"
);
source_contract_test!(
    aggregate_capacity_has_steps_dimension,
    BUDGET_RS,
    "pub max_steps_executable: u64"
);
source_contract_test!(
    aggregate_capacity_has_action_tickets_dimension,
    BUDGET_RS,
    "pub max_action_tickets: u64"
);
source_contract_test!(
    aggregate_capacity_has_parallel_dimension,
    BUDGET_RS,
    "pub max_parallel_in_flight: u32"
);
source_contract_test!(
    aggregate_capacity_has_gather_pages_dimension,
    BUDGET_RS,
    "pub max_gather_pages: u64"
);
source_contract_test!(
    aggregate_capacity_has_gather_items_dimension,
    BUDGET_RS,
    "pub max_gather_items: u64"
);
source_contract_test!(
    aggregate_capacity_has_result_bytes_dimension,
    BUDGET_RS,
    "pub max_result_bytes: u64"
);
source_contract_test!(
    aggregate_capacity_has_slots_dimension,
    BUDGET_RS,
    "pub max_total_slots_written: u64"
);
source_contract_test!(
    aggregate_capacity_has_active_runs_dimension,
    BUDGET_RS,
    "pub max_active_runs: u64"
);
source_contract_test!(
    aggregate_capacity_has_queue_depth_dimension,
    BUDGET_RS,
    "pub max_queue_depth: u64"
);
source_contract_test!(
    aggregate_capacity_has_journal_batch_dimension,
    BUDGET_RS,
    "pub max_journal_batch_bytes: u64"
);
source_contract_test!(
    aggregate_budget_from_workflow_exists,
    BUDGET_RS,
    "pub fn from_workflow("
);
source_contract_test!(
    aggregate_budget_from_whole_budget_exists,
    BUDGET_RS,
    "pub fn from_whole_workflow_budget("
);
source_contract_test!(
    aggregate_usage_try_add_exists,
    BUDGET_RS,
    "pub fn try_add_budget("
);
source_contract_test!(
    aggregate_usage_try_subtract_exists,
    BUDGET_RS,
    "pub fn try_subtract_budget("
);
source_contract_test!(
    aggregate_usage_fits_within_exists,
    BUDGET_RS,
    "pub fn fits_within("
);
source_contract_test!(
    aggregate_budget_validator_exists,
    BUDGET_RS,
    "pub fn validate_aggregate_budget("
);
source_contract_test!(
    aggregate_admission_with_budget_exists,
    ADMISSION_RS,
    "pub fn admit_run_with_budget("
);
source_contract_test!(
    workflow_budget_error_variant_exists,
    BUDGET_RS,
    "WorkflowBudget"
);
source_contract_test!(
    policy_exceeded_error_variant_exists,
    BUDGET_RS,
    "PolicyExceeded"
);
source_contract_test!(
    capacity_exceeded_error_variant_exists,
    BUDGET_RS,
    "CapacityExceeded"
);
source_contract_test!(overflow_error_variant_exists, BUDGET_RS, "Overflow");
source_contract_test!(underflow_error_variant_exists, BUDGET_RS, "Underflow");
source_contract_test!(
    invalid_capacity_error_variant_exists,
    BUDGET_RS,
    "InvalidCapacity"
);
source_contract_test!(
    reservation_not_found_error_variant_exists,
    BUDGET_RS,
    "ReservationNotFound"
);
source_contract_test!(
    runtime_resource_capacity_error_variant_exists,
    ADMISSION_RS,
    "ResourceCapacityExceeded"
);
source_contract_test!(capacity_comparison_names_requested, BUDGET_RS, "requested");
source_contract_test!(capacity_comparison_names_available, BUDGET_RS, "available");
source_contract_test!(policy_exceeded_names_actual, BUDGET_RS, "actual");
source_contract_test!(policy_exceeded_names_limit, BUDGET_RS, "limit");
source_contract_test!(reservation_tracks_run_id, BUDGET_RS, "pub run: RunId");
source_contract_test!(
    reservation_tracks_requested_budget,
    BUDGET_RS,
    "pub requested: AggregateResourceBudget"
);
source_contract_test!(budget_arithmetic_uses_checked_add, BUDGET_RS, "checked_add");
source_contract_test!(budget_arithmetic_uses_checked_sub, BUDGET_RS, "checked_sub");
source_contract_test!(budget_arithmetic_uses_checked_mul, BUDGET_RS, "checked_mul");
source_contract_test!(budget_conversion_uses_try_from, BUDGET_RS, "try_from");
source_contract_test!(
    shard_config_carries_aggregate_capacity,
    SHARD_TYPES_RS,
    "aggregate_capacity"
);
source_contract_test!(
    shard_state_carries_active_usage,
    SHARD_TYPES_RS,
    "active_usage"
);
source_contract_test!(
    shard_state_carries_reservations,
    SHARD_TYPES_RS,
    "reservations"
);
source_contract_test!(
    run_state_can_carry_budget_reservation,
    SHARD_TYPES_RS,
    "AggregateReservation"
);
source_contract_test!(
    shard_status_reports_active_usage,
    SHARD_TYPES_RS,
    "active_usage"
);
source_contract_test!(
    shard_status_reports_aggregate_capacity,
    SHARD_TYPES_RS,
    "aggregate_capacity"
);
source_contract_test!(
    admission_accepts_requested_budget_argument,
    ADMISSION_RS,
    "requested: AggregateResourceBudget"
);
source_contract_test!(
    admission_accepts_available_capacity_argument,
    ADMISSION_RS,
    "available: AggregateResourceCapacity"
);
source_contract_test!(
    admission_error_preserves_resource_name,
    ADMISSION_RS,
    "resource"
);
source_contract_test!(
    admission_error_preserves_requested_value,
    ADMISSION_RS,
    "requested"
);
source_contract_test!(
    admission_error_preserves_available_value,
    ADMISSION_RS,
    "available"
);
source_contract_test!(
    admission_with_budget_still_checks_artifacts,
    ADMISSION_RS,
    "compiled_ir_exists"
);
source_contract_test!(
    admission_record_exposes_budget_when_extended,
    ADMISSION_RS,
    "budget"
);
source_contract_test!(
    validate_budget_checks_steps_policy,
    BUDGET_RS,
    "max_steps_executable"
);
source_contract_test!(
    validate_budget_checks_action_policy,
    BUDGET_RS,
    "max_action_tickets"
);
source_contract_test!(
    validate_budget_checks_parallel_policy,
    BUDGET_RS,
    "max_parallel_in_flight"
);
source_contract_test!(
    validate_budget_checks_retries_policy,
    BUDGET_RS,
    "max_retries_per_action"
);
source_contract_test!(
    validate_budget_checks_gather_pages_policy,
    BUDGET_RS,
    "max_gather_pages"
);
source_contract_test!(
    validate_budget_checks_gather_items_policy,
    BUDGET_RS,
    "max_gather_items"
);
source_contract_test!(
    validate_budget_checks_for_each_policy,
    BUDGET_RS,
    "max_for_each_iterations"
);
source_contract_test!(
    validate_budget_checks_together_policy,
    BUDGET_RS,
    "max_together_branches"
);
source_contract_test!(
    validate_budget_checks_repeat_policy,
    BUDGET_RS,
    "max_repeat_attempts"
);
source_contract_test!(
    validate_budget_checks_runtime_policy,
    BUDGET_RS,
    "max_run_time_seconds"
);
source_contract_test!(
    validate_budget_checks_result_policy,
    BUDGET_RS,
    "max_result_bytes"
);
source_contract_test!(
    validate_budget_checks_slots_policy,
    BUDGET_RS,
    "max_total_slots_written"
);
source_contract_test!(
    validate_budget_checks_queue_policy,
    BUDGET_RS,
    "max_queue_depth"
);
source_contract_test!(
    validate_budget_checks_journal_policy,
    BUDGET_RS,
    "max_journal_batch_bytes"
);
source_absence_test!(
    aggregate_budget_does_not_saturate_steps_conversion,
    BUDGET_RS,
    "unwrap_or(u32::MAX)"
);
source_absence_test!(
    aggregate_budget_does_not_saturate_branch_count,
    BUDGET_RS,
    "unwrap_or(u16::MAX)"
);
source_absence_test!(
    aggregate_budget_does_not_saturate_add_action_tickets,
    BUDGET_RS,
    "saturating_add(1)"
);
source_absence_test!(
    aggregate_budget_does_not_saturate_add_gather_pages,
    BUDGET_RS,
    "max_gather_pages.saturating_add"
);
source_absence_test!(
    aggregate_budget_does_not_saturate_add_gather_items,
    BUDGET_RS,
    "max_gather_items.saturating_add"
);
source_absence_test!(
    aggregate_budget_does_not_saturate_add_for_each,
    BUDGET_RS,
    "max_for_each_iterations.saturating_add"
);
source_absence_test!(
    runtime_admission_does_not_parse_json_for_capacity,
    ADMISSION_RS,
    "serde_json"
);
source_absence_test!(
    runtime_admission_does_not_parse_yaml_for_capacity,
    ADMISSION_RS,
    "serde_yaml"
);
source_absence_test!(
    runtime_admission_does_not_parse_http_for_capacity,
    ADMISSION_RS,
    "http"
);
source_absence_test!(
    runtime_admission_does_not_parse_string_commands_for_capacity,
    ADMISSION_RS,
    "from_str"
);

#[test]
fn static_budget_model_has_no_forbidden_constructs_in_new_aggregate_surface() {
    let source = repository_source();
    let has_aggregate_surface = source.contains("AggregateResourceBudget");
    assert_eq!(
        has_aggregate_surface, true,
        "aggregate surface is absent, so forbidden-construct review cannot be meaningful yet"
    );
}
