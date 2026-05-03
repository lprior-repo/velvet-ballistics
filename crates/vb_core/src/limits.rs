#![forbid(unsafe_code)]

//! Compile-time resource limits for the hot runtime.
//!
//! These constants establish hard boundaries used in both the compiler (as upper
//! bounds during validation) and the runtime (for allocation and overflow checks).
//! Changing any value constitutes a protocol change requiring a major version bump.

/// Maximum number of steps allowed in a single compiled workflow.
///
pub const MAX_STEPS_PER_WORKFLOW: usize = 65_535;

/// Maximum number of slots allowed in a single compiled workflow.
///
pub const MAX_SLOTS_PER_WORKFLOW: usize = 65_535;

/// Maximum number of named slots that may be live within a single step activation.
///
pub const MAX_SLOTS_PER_STEP: usize = 256;

/// Maximum size of the constant pool in a compiled workflow.
///
pub const MAX_CONSTANTS: usize = 65_535;

/// Maximum recursive expression-evaluation depth (for safety in the bytecode engine).
///
pub const MAX_EXPRESSION_DEPTH: usize = 64;

/// Maximum number of bytecode operations allowed in one expression program.
///
pub const MAX_EXPRESSION_OPS: usize = 256;

/// Maximum number of expression programs in one compiled workflow.
///
pub const MAX_EXPRESSIONS: usize = 4_096;

/// Maximum number of accessor programs in one compiled workflow.
///
pub const MAX_ACCESSORS: usize = 8_192;

/// Maximum stack entries allowed while evaluating one expression program.
///
pub const MAX_EXPRESSION_STACK: u8 = 64;

/// `usize` form of [`MAX_EXPRESSION_STACK`] for fixed-size runtime scratch arrays.
///
pub const MAX_EXPRESSION_STACK_USIZE: usize = 64;

/// Maximum byte-length of a run name string supplied by the caller.
///
pub const MAX_RUN_NAME_LENGTH: usize = 1_024;

/// Maximum number of bytecode operations per compiled expression.
///
pub const MAX_BYTECODE_OPS_PER_EXPRESSION: usize = 256;

/// Maximum depth of accessor path segments.
///
pub const MAX_PATH_DEPTH: usize = 16;

/// Maximum nesting depth for language constructs (for_each, together, etc.).
///
pub const MAX_LANGUAGE_NESTING_DEPTH: u8 = 8;

/// Maximum number of slots in a single run frame.
///
pub const MAX_SLOTS: u16 = u16::MAX;

/// Maximum items in one runtime list arena value.
///
pub const MAX_LIST_ITEMS_PER_VALUE: usize = 65_535;

/// Maximum fields in one runtime object arena value.
///
pub const MAX_OBJECT_FIELDS_PER_VALUE: usize = 65_535;

/// Maximum bytes in one interned runtime symbol.
///
pub const MAX_SYMBOL_BYTES_PER_VALUE: usize = 4_096;

/// Maximum bytes in one runtime blob arena value.
///
pub const MAX_BLOB_BYTES_PER_VALUE: usize = 16_777_216;

/// Maximum total arena values (symbols + lists + objects + blobs) per run.
///
/// This cap prevents unbounded memory growth from nested ForEach x Together
/// compositions where individual value limits are respected but total
/// count is not bounded.
pub const MAX_VALUES_PER_RUN: usize = 1_000_000;

/// Maximum deterministic transitions per runtime tick.
///
pub const MAX_STEP_BUDGET: u64 = 10_000;

#[cfg(test)]
mod tests {
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
}
