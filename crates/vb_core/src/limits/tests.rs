
use super::*;

// --- All constants are non-zero ---

#[test]
fn max_steps_per_workflow_is_nonzero() {
    assert_ne!(MAX_STEPS_PER_WORKFLOW, 0);
}

#[test]
fn max_slots_per_workflow_is_nonzero() {
    assert_ne!(MAX_SLOTS_PER_WORKFLOW, 0);
}

#[test]
fn max_slots_per_step_is_nonzero() {
    assert_ne!(MAX_SLOTS_PER_STEP, 0);
}

#[test]
fn max_constants_is_nonzero() {
    assert_ne!(MAX_CONSTANTS, 0);
}

#[test]
fn max_expression_depth_is_nonzero() {
    assert_ne!(MAX_EXPRESSION_DEPTH, 0);
}

#[test]
fn max_expression_ops_is_nonzero() {
    assert_ne!(MAX_EXPRESSION_OPS, 0);
}

#[test]
fn max_expressions_is_nonzero() {
    assert_ne!(MAX_EXPRESSIONS, 0);
}

#[test]
fn max_accessors_is_nonzero() {
    assert_ne!(MAX_ACCESSORS, 0);
}

#[test]
fn max_expression_stack_is_nonzero() {
    assert_ne!(MAX_EXPRESSION_STACK, 0);
}

#[test]
fn max_expression_stack_usize_matches_u8() {
    assert_eq!(
        usize::from(MAX_EXPRESSION_STACK),
        MAX_EXPRESSION_STACK_USIZE
    );
}

#[test]
fn max_run_name_length_is_nonzero() {
    assert_ne!(MAX_RUN_NAME_LENGTH, 0);
}

#[test]
fn max_bytecode_ops_per_expression_is_nonzero() {
    assert_ne!(MAX_BYTECODE_OPS_PER_EXPRESSION, 0);
}

#[test]
fn max_path_depth_is_nonzero() {
    assert_ne!(MAX_PATH_DEPTH, 0);
}

#[test]
fn max_language_nesting_depth_is_nonzero() {
    assert_ne!(MAX_LANGUAGE_NESTING_DEPTH, 0);
}

#[test]
fn max_slots_is_u16_max() {
    assert_eq!(MAX_SLOTS, u16::MAX);
}

#[test]
fn max_list_items_per_value_is_nonzero() {
    assert_ne!(MAX_LIST_ITEMS_PER_VALUE, 0);
}

#[test]
fn max_object_fields_per_value_is_nonzero() {
    assert_ne!(MAX_OBJECT_FIELDS_PER_VALUE, 0);
}

#[test]
fn max_symbol_bytes_per_value_is_nonzero() {
    assert_ne!(MAX_SYMBOL_BYTES_PER_VALUE, 0);
}

#[test]
fn max_blob_bytes_per_value_is_nonzero() {
    assert_ne!(MAX_BLOB_BYTES_PER_VALUE, 0);
}

#[test]
fn max_values_per_run_is_nonzero() {
    assert_ne!(MAX_VALUES_PER_RUN, 0);
}

#[test]
fn max_step_budget_is_nonzero() {
    assert_ne!(MAX_STEP_BUDGET, 0);
}

// --- Relationship invariants ---

#[test]
fn max_slots_per_step_fits_in_max_slots_per_workflow() {
    assert!(
        MAX_SLOTS_PER_STEP <= MAX_SLOTS_PER_WORKFLOW,
        "per-step slot cap must not exceed per-workflow cap"
    );
}

#[test]
fn max_bytecode_ops_equals_max_expression_ops() {
    assert_eq!(
        MAX_BYTECODE_OPS_PER_EXPRESSION, MAX_EXPRESSION_OPS,
        "bytecode ops per expression must match expression ops limit"
    );
}

#[test]
fn max_expression_ops_fits_in_u16() {
    assert!(
        MAX_EXPRESSION_OPS <= usize::from(u16::MAX),
        "expression ops must fit in u16 for compact storage"
    );
}

#[test]
fn max_expressions_less_than_max_accessors() {
    assert!(
        MAX_EXPRESSIONS < MAX_ACCESSORS,
        "expressions are typically fewer than accessors"
    );
}

#[test]
fn max_path_depth_less_than_max_expression_depth() {
    assert!(
        MAX_PATH_DEPTH < MAX_EXPRESSION_DEPTH,
        "path depth must be less than expression depth"
    );
}

// --- Reasonable upper bounds ---

#[test]
fn max_steps_per_workflow_fits_in_u16() {
    assert!(
        MAX_STEPS_PER_WORKFLOW <= usize::from(u16::MAX),
        "steps must fit in u16 for StepIdx compatibility"
    );
}

#[test]
fn max_slots_per_workflow_fits_in_u16() {
    assert!(
        MAX_SLOTS_PER_WORKFLOW <= usize::from(u16::MAX),
        "slots must fit in u16 for SlotIdx compatibility"
    );
}

#[test]
fn max_blob_bytes_is_reasonable_megabytes() {
    let mb = MAX_BLOB_BYTES_PER_VALUE / (1024 * 1024);
    assert!(mb >= 1, "blob bytes must be at least 1 MiB, got {mb}");
    assert!(mb <= 1024, "blob bytes must be at most 1 GiB, got {mb}");
}

#[test]
fn max_values_per_run_is_at_least_one_million() {
    assert!(
        MAX_VALUES_PER_RUN >= 1_000_000,
        "values per run must accommodate at least 1M arena values"
    );
}

// --- Type-fit invariants ---

#[test]
fn max_constants_fits_in_u16() {
    assert!(
        MAX_CONSTANTS <= usize::from(u16::MAX),
        "MAX_CONSTANTS must fit in u16 for ConstIdx compatibility"
    );
}

#[test]
fn max_expressions_fits_in_u16() {
    assert!(
        MAX_EXPRESSIONS <= usize::from(u16::MAX),
        "MAX_EXPRESSIONS must fit in u16 for ExprIdx compatibility"
    );
}

#[test]
fn max_accessors_fits_in_u16() {
    assert!(
        MAX_ACCESSORS <= usize::from(u16::MAX),
        "MAX_ACCESSORS must fit in u16 for AccessorIdx compatibility"
    );
}

#[test]
fn max_list_items_per_value_fits_in_u32() {
    // On any platform we support, u32::MAX (4_294_967_295) is representable
    // as usize. We check that the constant is at most 4 billion.
    const U32_MAX_USIZE: usize = 4_294_967_295;
    assert!(
        MAX_LIST_ITEMS_PER_VALUE <= U32_MAX_USIZE,
        "MAX_LIST_ITEMS_PER_VALUE must fit in u32 for runtime storage"
    );
}

#[test]
fn max_step_budget_fits_in_u32() {
    assert!(
        MAX_STEP_BUDGET <= u64::from(u32::MAX),
        "MAX_STEP_BUDGET must fit in u32 for compact runtime representation"
    );
}

#[test]
fn max_run_name_length_under_64k() {
    assert!(
        MAX_RUN_NAME_LENGTH < 65_536,
        "MAX_RUN_NAME_LENGTH must be under 64k for reasonable allocation"
    );
}

#[test]
fn constants_less_than_or_equal_to_slots_per_workflow() {
    assert!(
        MAX_CONSTANTS <= MAX_SLOTS_PER_WORKFLOW,
        "constants must not exceed total slots per workflow"
    );
}

// --- New limits are non-zero ---

#[test]
fn max_input_bytes_is_nonzero() {
    assert_ne!(MAX_INPUT_BYTES, 0);
}

#[test]
fn max_output_bytes_is_nonzero() {
    assert_ne!(MAX_OUTPUT_BYTES, 0);
}

#[test]
fn max_blob_bytes_is_nonzero() {
    assert_ne!(MAX_BLOB_BYTES, 0);
}

#[test]
fn max_ipc_payload_bytes_is_nonzero() {
    assert_ne!(MAX_IPC_PAYLOAD_BYTES, 0);
}

#[test]
fn max_retry_attempts_is_nonzero() {
    assert_ne!(MAX_RETRY_ATTEMPTS, 0);
}

#[test]
fn max_fanout_is_nonzero() {
    assert_ne!(MAX_FANOUT, 0);
}

#[test]
fn max_collect_items_is_nonzero() {
    assert_ne!(MAX_COLLECT_ITEMS, 0);
}

#[test]
fn max_queue_depth_is_nonzero() {
    assert_ne!(MAX_QUEUE_DEPTH, 0);
}

#[test]
fn max_journal_batch_bytes_is_nonzero() {
    assert_ne!(MAX_JOURNAL_BATCH_BYTES, 0);
}

// --- Relationship invariants for new limits ---

#[test]
fn max_input_bytes_fits_in_u32() {
    // already u32, sanity check it's not u32::MAX which would indicate unbounded
    assert!(MAX_INPUT_BYTES < u32::MAX);
}

#[test]
fn max_output_bytes_fits_in_u32() {
    assert!(MAX_OUTPUT_BYTES < u32::MAX);
}

#[test]
fn max_blob_bytes_limit_is_reasonable_megabytes() {
    let mb = MAX_BLOB_BYTES / (1024 * 1024);
    assert!(mb >= 1, "max blob bytes must be at least 1 MiB, got {mb}");
    assert!(mb <= 1024, "max blob bytes must be at most 1 GiB, got {mb}");
}

#[test]
fn max_queue_depth_reasonable() {
    assert!(MAX_QUEUE_DEPTH >= 256, "queue depth must be at least 256");
}

#[test]
fn max_journal_batch_bytes_is_reasonable() {
    let mb = MAX_JOURNAL_BATCH_BYTES / (1024 * 1024);
    assert!(
        mb >= 1,
        "journal batch bytes must be at least 1 MiB, got {mb}"
    );
}
