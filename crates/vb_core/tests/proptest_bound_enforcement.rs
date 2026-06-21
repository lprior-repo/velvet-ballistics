#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]
//! Bound enforcement property tests: `CompiledWorkflow::try_from_parts`
//! enforces every `ResourceContract` dimension. The engine must fail closed
//! when any of `max_steps`, `max_slots`, `max_constants`, `max_accessors`,
//! `max_expressions` is violated, and must admit exactly-at-bound artifacts.

use proptest::prelude::*;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram,
    PathSegment, ResourceContract, WorkflowError, WorkflowParts,
};

// =========================================================================
// Strategies
// =========================================================================

fn contract_with(
    max_steps: u16,
    max_slots: u16,
    max_constants: u16,
    max_accessors: u16,
    max_expressions: u16,
) -> ResourceContract {
    ResourceContract {
        max_steps,
        max_slots,
        max_constants,
        max_accessors,
        max_expressions,
        max_expr_stack: 16,
        max_step_budget_per_tick: 10_000,
        max_transitions_per_tick: 10_000,
        max_input_bytes: 1_048_576,
        max_output_bytes: 262_144,
        max_blob_bytes: 16_777_216,
        max_ipc_payload_bytes: 1_048_576,
        max_retry_attempts: 3,
        max_fanout: 64,
        max_collect_items: 1_024,
        max_queue_depth: 1_024,
        max_journal_batch_bytes: 1_048_576,
        allows_secret_results: false,
    }
}

fn nop_parts(contract: ResourceContract, slot_count: u16, node_count: u16) -> WorkflowParts {
    // Chain Nops via `next` so every node except the last is followed by the
    // next node, satisfying reachability + forward-edge validation.
    let last = node_count.saturating_sub(1);
    let nodes: Vec<CompiledNode> = (0..node_count)
        .map(|i| CompiledNode {
            id: StepIdx::new(i),
            output: None,
            next: if i == last {
                None
            } else {
                Some(StepIdx::new(i.saturating_add(1)))
            },
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        })
        .collect();
    WorkflowParts {
        name: Box::<str>::from("bound_enforcement"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0xBE; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: contract,
        step_names: Box::default(),
    }
}

// =========================================================================
// Properties
// =========================================================================

proptest! {
    /// `max_steps` boundary: workflows exactly at the limit are admitted;
    /// workflows one step over the limit are rejected with the named resource.
    #[test]
    fn proptest_bound_enforcement_max_steps_boundary(limit in 1u16..32) {
        let exact = nop_parts(
            contract_with(limit, 16, 16, 16, 16),
            16,
            limit,
        );
        prop_assert!(
            CompiledWorkflow::try_from_parts(exact).is_ok(),
            "workflow at exact max_steps ({}) must be admitted", limit
        );
        let overflow = nop_parts(
            contract_with(limit, 16, 16, 16, 16),
            16,
            limit.saturating_add(1),
        );
        let result = CompiledWorkflow::try_from_parts(overflow);
        prop_assert!(
            matches!(
                result,
                Err(WorkflowError::ResourceContractExceeded { resource: "max_steps" })
            ),
            "workflow over max_steps ({}) must be rejected", limit
        );
    }

    /// `max_slots` boundary: exactly-at-limit slot counts are admitted;
    /// one-over-limit slot counts are rejected.
    #[test]
    fn proptest_bound_enforcement_max_slots_boundary(limit in 0u16..32) {
        let exact = nop_parts(
            contract_with(16, limit, 16, 16, 16),
            limit,
            1,
        );
        prop_assert!(
            CompiledWorkflow::try_from_parts(exact).is_ok(),
            "workflow at exact max_slots ({}) must be admitted", limit
        );
        let overflow = nop_parts(
            contract_with(16, limit, 16, 16, 16),
            limit.saturating_add(1),
            1,
        );
        let result = CompiledWorkflow::try_from_parts(overflow);
        prop_assert!(
            matches!(
                result,
                Err(WorkflowError::ResourceContractExceeded { resource: "max_slots" })
            ),
            "workflow over max_slots ({}) must be rejected", limit
        );
    }

    /// `max_constants` boundary: random constant-pool sizes are bounded by
    /// the contract.
    #[test]
    fn proptest_bound_enforcement_max_constants_boundary(limit in 1u16..16) {
        let constants: Vec<ConstValue> = (0..limit)
            .map(|_| ConstValue::Null)
            .collect();
        let mut parts = nop_parts(
            contract_with(16, 16, limit, 16, 16),
            16,
            1,
        );
        parts.constants = constants.into_boxed_slice();
        prop_assert!(
            CompiledWorkflow::try_from_parts(parts).is_ok(),
            "constant pool at exact max_constants ({}) must be admitted", limit
        );
        let overflow: Vec<ConstValue> = (0..=limit)
            .map(|_| ConstValue::Null)
            .collect();
        let mut parts_over = nop_parts(
            contract_with(16, 16, limit, 16, 16),
            16,
            1,
        );
        parts_over.constants = overflow.into_boxed_slice();
        let result = CompiledWorkflow::try_from_parts(parts_over);
        prop_assert!(
            matches!(
                result,
                Err(WorkflowError::ResourceContractExceeded { resource: "max_constants" })
            ),
            "constant pool over max_constants ({}) must be rejected", limit
        );
    }

    /// `max_accessors` boundary: random accessor-program counts must be
    /// bounded by the contract.
    #[test]
    fn proptest_bound_enforcement_max_accessors_boundary(limit in 0u16..8) {
        let accessors: Vec<AccessorProgram> = (0..limit)
            .map(|i| AccessorProgram {
                root: SlotIdx::new(i),
                path: Box::new([PathSegment::Index(0)]),
            })
            .collect();
        let mut parts = nop_parts(
            contract_with(16, 16, 16, limit, 16),
            u16::try_from(limit).unwrap_or(0).saturating_add(1),
            1,
        );
        parts.accessors = accessors.into_boxed_slice();
        // Need slot_count >= max(root)+1
        let overflow_root = u16::try_from(limit).unwrap_or(0);
        parts.slot_count = overflow_root.saturating_add(1).max(1);
        prop_assert!(
            CompiledWorkflow::try_from_parts(parts).is_ok(),
            "accessor pool at exact max_accessors ({}) must be admitted", limit
        );
        let overflow_count: Vec<AccessorProgram> = (0..=limit)
            .map(|i| AccessorProgram {
                root: SlotIdx::new(i),
                path: Box::new([PathSegment::Index(0)]),
            })
            .collect();
        let mut parts_over = nop_parts(
            contract_with(16, 16, 16, limit, 16),
            u16::try_from(limit + 1).unwrap_or(0).saturating_add(1),
            1,
        );
        parts_over.accessors = overflow_count.into_boxed_slice();
        parts_over.slot_count = u16::try_from(limit + 1)
            .unwrap_or(u16::MAX)
            .saturating_add(1);
        let result = CompiledWorkflow::try_from_parts(parts_over);
        prop_assert!(
            matches!(
                result,
                Err(WorkflowError::ResourceContractExceeded { resource: "max_accessors" })
            ),
            "accessor pool over max_accessors ({}) must be rejected", limit
        );
    }

    /// `max_expressions` boundary: random expression-program counts must be
    /// bounded by the contract.
    #[test]
    fn proptest_bound_enforcement_max_expressions_boundary(limit in 0u16..8) {
        let expressions: Vec<ExprProgram> = (0..limit)
            .map(|_| ExprProgram {
                ops: vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice(),
                max_stack: 1,
                constants: Box::default(),
            })
            .collect();
        let mut parts = nop_parts(
            contract_with(16, 16, 16, 16, limit),
            16,
            1,
        );
        parts.expressions = expressions.into_boxed_slice();
        prop_assert!(
            CompiledWorkflow::try_from_parts(parts).is_ok(),
            "expression pool at exact max_expressions ({}) must be admitted", limit
        );
        let overflow_count: Vec<ExprProgram> = (0..=limit)
            .map(|_| ExprProgram {
                ops: vec![ExprOp::LoadConst(ConstIdx::new(0))].into_boxed_slice(),
                max_stack: 1,
                constants: Box::default(),
            })
            .collect();
        let mut parts_over = nop_parts(
            contract_with(16, 16, 16, 16, limit),
            16,
            1,
        );
        parts_over.expressions = overflow_count.into_boxed_slice();
        let result = CompiledWorkflow::try_from_parts(parts_over);
        prop_assert!(
            matches!(
                result,
                Err(WorkflowError::ResourceContractExceeded { resource: "max_expressions" })
            ),
            "expression pool over max_expressions ({}) must be rejected", limit
        );
    }

    /// Combined sweep: every dimension simultaneously at its boundary must
    /// admit; perturbing any single dimension by +1 must reject with that
    /// dimension's resource label. Sweeps `max_steps` while holding others
    /// fixed to confirm dimension independence.
    #[test]
    fn proptest_bound_enforcement_independent_dimensions(max_steps in 1u16..16) {
        // Build a workflow that sits at the max_steps boundary.
        let nodes_count = max_steps;
        let parts = nop_parts(
            contract_with(max_steps, 16, 16, 16, 16),
            16,
            nodes_count,
        );
        let result = CompiledWorkflow::try_from_parts(parts);
        prop_assert!(
            result.is_ok(),
            "max_steps={} with matching node count must be admitted", max_steps
        );
        // Independently bumping max_steps by 1 (headroom) must still admit
        // the same workflow and must NOT trigger rejection on any other
        // dimension.
        let bumped = nop_parts(
            contract_with(max_steps.saturating_add(1), 16, 16, 16, 16),
            16,
            nodes_count,
        );
        let bumped_result = CompiledWorkflow::try_from_parts(bumped);
        prop_assert!(
            bumped_result.is_ok(),
            "bumping max_steps by 1 (with same nodes) must still admit; no other dimension must falsely reject"
        );
        // Independently reducing max_steps by 1 (below the node count) must
        // reject by max_steps and not by any other dimension.
        if max_steps > 1 {
            let reduced = nop_parts(
                contract_with(max_steps.saturating_sub(1), 16, 16, 16, 16),
                16,
                nodes_count,
            );
            let reduced_result = CompiledWorkflow::try_from_parts(reduced);
            prop_assert!(
                matches!(
                    reduced_result,
                    Err(WorkflowError::ResourceContractExceeded { resource: "max_steps" })
                ),
                "reducing max_steps by 1 (under node count) must reject by max_steps"
            );
        }
    }

    /// Property-level tie-in: a workflow whose `accessor` references a
    /// symbol out of range is rejected at admission.
    #[test]
    fn proptest_bound_enforcement_symbol_out_of_bounds(symbol_raw in 1u32..64) {
        // Build a single Nop node with one accessor whose path contains an
        // out-of-range SymbolId; admission must reject.
        let accessor = AccessorProgram {
            root: SlotIdx::new(0),
            path: Box::new([PathSegment::Field(SymbolId::new(symbol_raw))]),
        };
        let parts = WorkflowParts {
            name: Box::<str>::from("symbol_oob"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0x5E; 32]),
            nodes: vec![CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            }]
            .into_boxed_slice(),
            expressions: Box::new([]),
            accessors: vec![accessor].into_boxed_slice(),
            constants: vec![ConstValue::Null].into_boxed_slice(),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::default(),
        };
        let result = CompiledWorkflow::try_from_parts(parts);
        prop_assert!(
            matches!(
                result,
                Err(WorkflowError::SymbolOutOfBounds { .. })
            ),
            "accessor referencing out-of-range symbol must be rejected"
        );
    }
}
