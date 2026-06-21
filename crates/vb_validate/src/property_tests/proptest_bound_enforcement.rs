#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::as_conversions,
    clippy::panic,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing
)]
#![forbid(unsafe_code)]
//! Section 38 property test: `bound_enforcement`.
//!
//! Master plan §38, row "Bound enforcement":
//! "Retry attempts never exceed limit; collect never exceeds
//!  page/item/time limits".
//!
//! This file asserts the bound-enforcement invariants of
//! `validate_resource_limits`:
//! - `LimitRequired` is returned when a declared bound is `0`.
//! - `LimitExceeded` is returned when a declared bound exceeds the
//!   corresponding hard limit, or when actual usage exceeds the
//!   declared bound (for `max_steps` and `max_slots`).
//! - For any in-range `(actual, declared, hard_limit)` triple, the
//!   validator returns `Ok(())`.

use proptest::prelude::*;

use crate::ValidationError;
use crate::type_taint::{
    ResourceLimits, StepKind, StepTypes, TypedValue, ValueType, WorkflowTypes,
    validate_resource_limits,
};

fn empty_wf_with_contract(contract: ResourceLimits) -> WorkflowTypes {
    WorkflowTypes {
        inputs: Vec::new(),
        vars: Vec::new(),
        secrets: Vec::new(),
        steps: Vec::new(),
        resource_contract: contract,
    }
}

fn n_step_wf(n: usize, contract: ResourceLimits) -> WorkflowTypes {
    let steps: Vec<StepTypes> = (0..n)
        .map(|i| StepTypes {
            id: format!("s{i}"),
            kind: StepKind::Finish {
                result: TypedValue::Literal(ValueType::Any),
            },
        })
        .collect();
    WorkflowTypes {
        inputs: Vec::new(),
        vars: Vec::new(),
        secrets: Vec::new(),
        steps,
        resource_contract: contract,
    }
}

proptest! {
    /// Zero declared limit always returns `LimitRequired` for the
    /// selected field. The other 15 fields are non-zero, so the error
    /// unambiguously identifies the zero-bound field.
    #[test]
    fn be_zero_declared_returns_limit_required(field in 0u8..16u8) {
        let hard = ResourceLimits::default();
        let mut contract = ResourceLimits::default();
        match field {
            0 => contract.max_steps = 0,
            1 => contract.max_slots = 0,
            2 => contract.max_constants = 0,
            3 => contract.max_accessors = 0,
            4 => contract.max_expressions = 0,
            5 => contract.max_expr_stack = 0,
            6 => contract.max_step_budget_per_tick = 0,
            7 => contract.max_input_bytes = 0,
            8 => contract.max_output_bytes = 0,
            9 => contract.max_blob_bytes = 0,
            10 => contract.max_ipc_payload_bytes = 0,
            11 => contract.max_retry_attempts = 0,
            12 => contract.max_fanout = 0,
            13 => contract.max_collect_items = 0,
            14 => contract.max_queue_depth = 0,
            _ => contract.max_journal_batch_bytes = 0,
        }
        let wf = empty_wf_with_contract(contract);
        let result = validate_resource_limits(&wf, &hard);
        match result {
            Err(ValidationError::LimitRequired { resource }) => {
                // The field name must match the bound we zeroed.
                let expected = match field {
                    0 => "max_steps",
                    1 => "max_slots",
                    2 => "max_constants",
                    3 => "max_accessors",
                    4 => "max_expressions",
                    5 => "max_expr_stack",
                    6 => "max_step_budget_per_tick",
                    7 => "max_input_bytes",
                    8 => "max_output_bytes",
                    9 => "max_blob_bytes",
                    10 => "max_ipc_payload_bytes",
                    11 => "max_retry_attempts",
                    12 => "max_fanout",
                    13 => "max_collect_items",
                    14 => "max_queue_depth",
                    _ => "max_journal_batch_bytes",
                };
                prop_assert_eq!(resource, expected, "wrong resource name");
            }
            other => prop_assert!(false, "expected LimitRequired, got {other:?}"),
        }
    }

    /// When `actual == 0` and `declared >= 1` and `declared <= hard`,
    /// the validator returns `Ok(())` for a per-field sweep.
    #[test]
    fn be_in_range_empty_wf_passes(field in 0u8..16u8, value in 1usize..128usize) {
        let mut contract = ResourceLimits::default();
        let value_usize = value;
        match field {
            0 => contract.max_steps = value_usize,
            1 => contract.max_slots = value_usize,
            2 => contract.max_constants = value_usize,
            3 => contract.max_accessors = value_usize,
            4 => contract.max_expressions = value_usize,
            5 => contract.max_expr_stack = value_usize,
            6 => contract.max_step_budget_per_tick = value_usize,
            7 => contract.max_input_bytes = value_usize,
            8 => contract.max_output_bytes = value_usize,
            9 => contract.max_blob_bytes = value_usize,
            10 => contract.max_ipc_payload_bytes = value_usize,
            11 => contract.max_retry_attempts = value_usize,
            12 => contract.max_fanout = value_usize,
            13 => contract.max_collect_items = value_usize,
            14 => contract.max_queue_depth = value_usize,
            _ => contract.max_journal_batch_bytes = value_usize,
        }
        // Bump all 16 hard limits to usize::MAX so the contract's
        // chosen field is comfortably inside the hard envelope,
        // regardless of the default values. Otherwise fields like
        // `max_retry_attempts` (default hard=10) trip
        // `LimitExceeded` for any chosen value > 10.
        let hard = ResourceLimits {
            max_steps: usize::MAX,
            max_slots: usize::MAX,
            max_constants: usize::MAX,
            max_accessors: usize::MAX,
            max_expressions: usize::MAX,
            max_expr_stack: usize::MAX,
            max_step_budget_per_tick: usize::MAX,
            max_input_bytes: usize::MAX,
            max_output_bytes: usize::MAX,
            max_blob_bytes: usize::MAX,
            max_ipc_payload_bytes: usize::MAX,
            max_retry_attempts: usize::MAX,
            max_fanout: usize::MAX,
            max_collect_items: usize::MAX,
            max_queue_depth: usize::MAX,
            max_journal_batch_bytes: usize::MAX,
            allows_secret_results: false,
        };
        let wf = empty_wf_with_contract(contract);
        let result = validate_resource_limits(&wf, &hard);
        prop_assert_eq!(result, Ok(()));
    }

    /// When `declared > hard`, the validator returns `LimitExceeded`
    /// with the field name of the violated bound.
    #[test]
    fn be_declared_exceeds_hard_rejected(
        declared in 1usize..1024usize,
        hard in 0usize..1023usize,
    ) {
        prop_assume!(declared > hard);
        let mut contract = ResourceLimits::default();
        contract.max_retry_attempts = declared;
        let mut hard_limits = ResourceLimits::default();
        hard_limits.max_retry_attempts = hard;
        let wf = empty_wf_with_contract(contract);
        let result = validate_resource_limits(&wf, &hard_limits);
        match result {
            Err(ValidationError::LimitExceeded { resource }) => {
                prop_assert_eq!(resource, "max_retry_attempts");
            }
            other => prop_assert!(
                false,
                "expected LimitExceeded(max_retry_attempts), got {other:?}"
            ),
        }
    }

    /// Boundary case: `declared == hard` returns `Ok(())`.
    #[test]
    fn be_declared_equals_hard_is_ok(value in 1usize..1024usize) {
        let mut contract = ResourceLimits::default();
        contract.max_retry_attempts = value;
        let mut hard = ResourceLimits::default();
        hard.max_retry_attempts = value;
        let wf = empty_wf_with_contract(contract);
        let result = validate_resource_limits(&wf, &hard);
        prop_assert_eq!(result, Ok(()));
    }

    /// `actual > declared` for `max_steps` returns `LimitExceeded` with
    /// `resource = "max_steps"`. The contract has `max_slots` set high
    /// so it never fires first.
    #[test]
    fn be_actual_steps_exceed_declared_rejected(
        declared_steps in 1usize..4usize,
        extra in 1usize..4usize,
    ) {
        let actual = declared_steps.saturating_add(extra);
        prop_assume!(actual > declared_steps);
        let contract = ResourceLimits {
            max_steps: declared_steps,
            max_slots: 1024, // large enough so the slots check passes
            ..ResourceLimits::default()
        };
        let hard = ResourceLimits {
            max_steps: 1024,
            max_slots: 1024,
            ..ResourceLimits::default()
        };
        let wf = n_step_wf(actual, contract);
        let result = validate_resource_limits(&wf, &hard);
        match result {
            Err(ValidationError::LimitExceeded { resource }) => {
                prop_assert_eq!(resource, "max_steps");
            }
            other => prop_assert!(
                false,
                "expected LimitExceeded(max_steps), got {other:?}"
            ),
        }
    }

    /// `actual == declared` for `max_steps` is the boundary that
    /// returns `Ok(())`.
    #[test]
    fn be_actual_steps_equal_declared_is_ok(count in 1usize..4usize) {
        let contract = ResourceLimits {
            max_steps: count,
            max_slots: 1024,
            ..ResourceLimits::default()
        };
        let hard = ResourceLimits {
            max_steps: 1024,
            max_slots: 1024,
            ..ResourceLimits::default()
        };
        let wf = n_step_wf(count, contract);
        let result = validate_resource_limits(&wf, &hard);
        prop_assert_eq!(result, Ok(()));
    }

    /// Empty workflow with default `ResourceLimits` always validates
    /// against default hard limits.
    #[test]
    fn be_default_contract_passes_default_hard(_unit in 0u8..1u8) {
        let wf = empty_wf_with_contract(ResourceLimits::default());
        let result = validate_resource_limits(&wf, &ResourceLimits::default());
        prop_assert_eq!(result, Ok(()));
    }

    /// Validator is deterministic over `(workflow, hard_limits)`:
    /// repeated invocations return identical results.
    #[test]
    fn be_deterministic(
        declared_steps in 0usize..32usize,
        hard_steps in 0usize..64usize,
    ) {
        let contract = ResourceLimits {
            max_steps: declared_steps,
            ..ResourceLimits::default()
        };
        let hard = ResourceLimits {
            max_steps: hard_steps,
            ..ResourceLimits::default()
        };
        let wf = empty_wf_with_contract(contract);
        let r1 = validate_resource_limits(&wf, &hard);
        let r2 = validate_resource_limits(&wf, &hard);
        let r3 = validate_resource_limits(&wf, &hard);
        prop_assert_eq!(r1.clone(), r2.clone());
        prop_assert_eq!(r2, r3);
    }

    /// The validator never panics for arbitrary step counts against
    /// arbitrary hard limits.
    #[test]
    fn be_never_panics(
        steps_count in 0usize..16usize,
        declared_steps in 0usize..32usize,
        hard_steps in 0usize..64usize,
    ) {
        let contract = ResourceLimits {
            max_steps: declared_steps,
            max_slots: 1024,
            ..ResourceLimits::default()
        };
        let hard = ResourceLimits {
            max_steps: hard_steps,
            max_slots: 1024,
            ..ResourceLimits::default()
        };
        let wf = n_step_wf(steps_count, contract);
        let _ = validate_resource_limits(&wf, &hard);
    }

    /// Every error returned by the validator is one of the two typed
    /// bound errors: `LimitRequired` or `LimitExceeded`. No exotic
    /// variants leak.
    #[test]
    fn be_error_variants_are_typed(
        declared_steps in 0usize..32usize,
        hard_steps in 0usize..64usize,
    ) {
        let contract = ResourceLimits {
            max_steps: declared_steps,
            ..ResourceLimits::default()
        };
        let hard = ResourceLimits {
            max_steps: hard_steps,
            ..ResourceLimits::default()
        };
        let wf = empty_wf_with_contract(contract);
        let result = validate_resource_limits(&wf, &hard);
        match result {
            Ok(())
            | Err(ValidationError::LimitRequired { .. })
            | Err(ValidationError::LimitExceeded { .. }) => {}
            other => prop_assert!(false, "unexpected error variant: {other:?}"),
        }
    }
}
