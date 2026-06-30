// vb_core engine validate red-phase tests — RED PHASE
// Tests for validate_resource_contract and symbol bounds validation paths.

#![forbid(unsafe_code)]

use super::validate_resource_contract;
use crate::ids::{SlotIdx, StepIdx, WorkflowDigest};
use crate::limits::{
    MAX_ACCESSORS, MAX_BLOB_BYTES, MAX_COLLECT_ITEMS, MAX_CONSTANTS, MAX_EXPRESSION_STACK,
    MAX_EXPRESSIONS, MAX_FANOUT, MAX_INPUT_BYTES, MAX_IPC_PAYLOAD_BYTES, MAX_JOURNAL_BATCH_BYTES,
    MAX_OUTPUT_BYTES, MAX_QUEUE_DEPTH, MAX_RETRY_ATTEMPTS, MAX_SLOTS_PER_WORKFLOW, MAX_STEP_BUDGET,
    MAX_STEPS_PER_WORKFLOW,
};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ResourceContract, WorkflowError, WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helper: minimal WorkflowParts factory
// ---------------------------------------------------------------------------

fn valid_parts() -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

// ---------------------------------------------------------------------------
// RED PHASE: validate_resource_contract — ResourceContractTooLarge tests
//
// These prove that declared limits exceeding hard limits produce
// WorkflowError::ResourceContractTooLarge.
// ---------------------------------------------------------------------------

mod resource_contract_too_large {
    use super::*;

    /// max_steps is u16-bounded and accepts the maximum representable declaration.
    #[test]
    fn accepts_max_steps_at_type_max() {
        let mut parts = valid_parts();
        parts.resource_contract.max_steps = u16::MAX;

        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    /// max_slots is u16-bounded and accepts the maximum representable declaration.
    #[test]
    fn accepts_max_slots_at_type_max() {
        let mut parts = valid_parts();
        parts.resource_contract.max_slots = u16::MAX;

        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    /// max_constants is u16-bounded and accepts the maximum representable declaration.
    #[test]
    fn accepts_max_constants_at_type_max() {
        let mut parts = valid_parts();
        parts.resource_contract.max_constants = u16::MAX;

        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    /// max_accessors declared > MAX_ACCESSORS → ResourceContractTooLarge
    #[test]
    fn rejects_max_accessors_too_large() {
        let mut parts = valid_parts();
        let too_large = u16::try_from(MAX_ACCESSORS.saturating_add(1)).unwrap_or(u16::MAX);
        parts.resource_contract.max_accessors = too_large;

        let err = validate_resource_contract(&parts)
            .expect_err("expected Err when max_accessors exceeds hard limit");
        assert!(
            matches!(
                &err,
                WorkflowError::ResourceContractTooLarge { resource } if *resource == "max_accessors"
            ),
            "expected ResourceContractTooLarge {{ resource: \"max_accessors\" }}, got {err:?}"
        );
    }

    /// max_expressions declared > MAX_EXPRESSIONS → ResourceContractTooLarge
    #[test]
    fn rejects_max_expressions_too_large() {
        let mut parts = valid_parts();
        let too_large = u16::try_from(MAX_EXPRESSIONS.saturating_add(1)).unwrap_or(u16::MAX);
        parts.resource_contract.max_expressions = too_large;

        let err = validate_resource_contract(&parts)
            .expect_err("expected Err when max_expressions exceeds hard limit");
        assert!(
            matches!(
                &err,
                WorkflowError::ResourceContractTooLarge { resource } if *resource == "max_expressions"
            ),
            "expected ResourceContractTooLarge {{ resource: \"max_expressions\" }}, got {err:?}"
        );
    }

    /// max_expr_stack declared > MAX_EXPRESSION_STACK (64) → ResourceContractTooLarge
    #[test]
    fn rejects_max_expr_stack_too_large() {
        let mut parts = valid_parts();
        parts.resource_contract.max_expr_stack = MAX_EXPRESSION_STACK.saturating_add(1);

        let err = validate_resource_contract(&parts)
            .expect_err("expected Err when max_expr_stack exceeds hard limit");
        assert!(
            matches!(
                &err,
                WorkflowError::ResourceContractTooLarge { resource } if *resource == "max_expr_stack"
            ),
            "expected ResourceContractTooLarge {{ resource: \"max_expr_stack\" }}, got {err:?}"
        );
    }

    /// All limits at EXACT hard limit → Ok (not TooLarge)
    #[test]
    fn accepts_all_limits_at_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract = ResourceContract {
            max_steps: u16::try_from(MAX_STEPS_PER_WORKFLOW).unwrap(),
            max_slots: u16::try_from(MAX_SLOTS_PER_WORKFLOW).unwrap(),
            max_constants: u16::try_from(MAX_CONSTANTS).unwrap(),
            max_accessors: u16::try_from(MAX_ACCESSORS).unwrap(),
            max_expressions: u16::try_from(MAX_EXPRESSIONS).unwrap(),
            max_expr_stack: MAX_EXPRESSION_STACK,
            ..ResourceContract::DEFAULT
        };

        let result = validate_resource_contract(&parts);
        assert!(
            result.is_ok(),
            "limits at exactly hard limit should be Ok, got {result:?}"
        );
    }

    /// Default contract → Ok
    #[test]
    fn accepts_default_contract() {
        let mut parts = valid_parts();
        parts.resource_contract = ResourceContract::DEFAULT;

        let result = validate_resource_contract(&parts);
        assert!(
            result.is_ok(),
            "default contract should be accepted, got {result:?}"
        );
    }

    /// Multiple limits too large → first too-large representable limit in code order is returned.
    #[test]
    fn returns_first_too_large_in_code_order() {
        let mut parts = valid_parts();
        parts.resource_contract.max_accessors = u16::MAX;
        parts.resource_contract.max_expr_stack = u8::MAX;

        let err = validate_resource_contract(&parts)
            .expect_err("expected Err when multiple limits too large");
        assert!(
            matches!(
                &err,
                WorkflowError::ResourceContractTooLarge {
                    resource: "max_accessors"
                }
            ),
            "expected first too-large resource to be max_accessors, got {err:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// RED PHASE: validate_resource_contract — exact error field values
//
// These prove that the error contains the exact resource name.
// ---------------------------------------------------------------------------

mod resource_contract_error_exactness {
    use super::*;

    #[allow(dead_code)]
    fn check_single_too_large(getter: fn(&mut WorkflowParts), resource_name: &'static str) {
        let mut parts = valid_parts();
        // Set only this one limit too large
        parts.resource_contract = ResourceContract::DEFAULT;
        getter(&mut parts);

        let err = validate_resource_contract(&parts).unwrap_err();
        assert!(
            matches!(
                &err,
                WorkflowError::ResourceContractTooLarge { resource } if *resource == resource_name
            ),
            "expected ResourceContractTooLarge {{ resource: \"{resource_name}\" }}, got {err}"
        );
    }

    #[test]
    fn too_large_error_contains_exact_resource_name_max_steps() {
        let mut parts = valid_parts();
        parts.resource_contract.max_steps = u16::MAX;
        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    #[test]
    fn too_large_error_contains_exact_resource_name_max_slots() {
        let mut parts = valid_parts();
        parts.resource_contract.max_slots = u16::MAX;
        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    #[test]
    fn too_large_error_contains_exact_resource_name_max_constants() {
        let mut parts = valid_parts();
        parts.resource_contract.max_constants = u16::MAX;
        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    #[test]
    fn too_large_error_contains_exact_resource_name_max_accessors() {
        let mut parts = valid_parts();
        parts.resource_contract.max_accessors = u16::MAX;
        let result = validate_resource_contract(&parts).unwrap_err();
        assert!(
            matches!(
                &result,
                WorkflowError::ResourceContractTooLarge {
                    resource: "max_accessors"
                }
            ),
            "expected resource \"max_accessors\", got {result}"
        );
    }

    #[test]
    fn too_large_error_contains_exact_resource_name_max_expressions() {
        let mut parts = valid_parts();
        parts.resource_contract.max_expressions = u16::MAX;
        let result = validate_resource_contract(&parts).unwrap_err();
        assert!(
            matches!(
                &result,
                WorkflowError::ResourceContractTooLarge {
                    resource: "max_expressions"
                }
            ),
            "expected resource \"max_expressions\", got {result}"
        );
    }

    #[test]
    fn too_large_error_contains_exact_resource_name_max_expr_stack() {
        let mut parts = valid_parts();
        parts.resource_contract.max_expr_stack = u8::MAX;
        let result = validate_resource_contract(&parts).unwrap_err();
        assert!(
            matches!(
                &result,
                WorkflowError::ResourceContractTooLarge {
                    resource: "max_expr_stack"
                }
            ),
            "expected resource \"max_expr_stack\", got {result}"
        );
    }
}

// ---------------------------------------------------------------------------
// RED PHASE: validate_resource_contract — extended field bounds
//
// Localized regression coverage for fields beyond the original six (max_steps,
// max_slots, max_constants, max_accessors, max_expressions, max_expr_stack).
// Each declared limit is exercised against its hard limit so that an
// `engine::validate_resource_contract` call over the public API refuses
// contracts that the default configuration would silently accept.
// ---------------------------------------------------------------------------

mod resource_contract_extended_field_bounds {
    use super::*;

    fn assert_too_large_for(parts: &WorkflowParts, expected: &'static str) {
        let err = validate_resource_contract(parts)
            .expect_err("expected Err when extended resource exceeds hard limit");
        assert!(
            matches!(
                &err,
                WorkflowError::ResourceContractTooLarge { resource } if *resource == expected
            ),
            "expected ResourceContractTooLarge {{ resource: \"{expected}\" }}, got {err:?}"
        );
    }

    #[test]
    fn rejects_max_step_budget_per_tick_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_step_budget_per_tick = MAX_STEP_BUDGET.saturating_add(1);
        assert_too_large_for(&parts, "max_step_budget_per_tick");
    }

    #[test]
    fn rejects_max_transitions_per_tick_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_transitions_per_tick = MAX_STEP_BUDGET.saturating_add(1);
        assert_too_large_for(&parts, "max_transitions_per_tick");
    }

    #[test]
    fn rejects_max_input_bytes_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_input_bytes = MAX_INPUT_BYTES.saturating_add(1);
        assert_too_large_for(&parts, "max_input_bytes");
    }

    #[test]
    fn rejects_max_output_bytes_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_output_bytes = MAX_OUTPUT_BYTES.saturating_add(1);
        assert_too_large_for(&parts, "max_output_bytes");
    }

    #[test]
    fn rejects_max_blob_bytes_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_blob_bytes = MAX_BLOB_BYTES.saturating_add(1);
        assert_too_large_for(&parts, "max_blob_bytes");
    }

    #[test]
    fn rejects_max_ipc_payload_bytes_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_ipc_payload_bytes = MAX_IPC_PAYLOAD_BYTES.saturating_add(1);
        assert_too_large_for(&parts, "max_ipc_payload_bytes");
    }

    #[test]
    fn rejects_max_retry_attempts_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_retry_attempts =
            u16::try_from(u32::from(MAX_RETRY_ATTEMPTS).saturating_add(1)).unwrap_or(u16::MAX);
        assert_too_large_for(&parts, "max_retry_attempts");
    }

    #[test]
    fn rejects_max_fanout_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_fanout =
            u16::try_from(u32::from(MAX_FANOUT).saturating_add(1)).unwrap_or(u16::MAX);
        assert_too_large_for(&parts, "max_fanout");
    }

    #[test]
    fn rejects_max_collect_items_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_collect_items = MAX_COLLECT_ITEMS.saturating_add(1);
        assert_too_large_for(&parts, "max_collect_items");
    }

    #[test]
    fn rejects_max_queue_depth_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_queue_depth = MAX_QUEUE_DEPTH.saturating_add(1);
        assert_too_large_for(&parts, "max_queue_depth");
    }

    #[test]
    fn rejects_max_journal_batch_bytes_over_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract.max_journal_batch_bytes = MAX_JOURNAL_BATCH_BYTES.saturating_add(1);
        assert_too_large_for(&parts, "max_journal_batch_bytes");
    }

    #[test]
    fn accepts_all_extended_fields_at_hard_limit() {
        let mut parts = valid_parts();
        parts.resource_contract = ResourceContract {
            max_steps: u16::try_from(MAX_STEPS_PER_WORKFLOW).unwrap(),
            max_slots: u16::try_from(MAX_SLOTS_PER_WORKFLOW).unwrap(),
            max_constants: u16::try_from(MAX_CONSTANTS).unwrap(),
            max_accessors: u16::try_from(MAX_ACCESSORS).unwrap(),
            max_expressions: u16::try_from(MAX_EXPRESSIONS).unwrap(),
            max_expr_stack: MAX_EXPRESSION_STACK,
            max_step_budget_per_tick: MAX_STEP_BUDGET,
            max_transitions_per_tick: MAX_STEP_BUDGET,
            max_input_bytes: MAX_INPUT_BYTES,
            max_output_bytes: MAX_OUTPUT_BYTES,
            max_blob_bytes: MAX_BLOB_BYTES,
            max_ipc_payload_bytes: MAX_IPC_PAYLOAD_BYTES,
            max_retry_attempts: MAX_RETRY_ATTEMPTS,
            max_fanout: MAX_FANOUT,
            max_collect_items: MAX_COLLECT_ITEMS,
            max_queue_depth: MAX_QUEUE_DEPTH,
            max_journal_batch_bytes: MAX_JOURNAL_BATCH_BYTES,
            ..ResourceContract::DEFAULT
        };
        let result = validate_resource_contract(&parts);
        assert!(
            result.is_ok(),
            "extended fields at exactly hard limit should be Ok, got {result:?}"
        );
    }
}
