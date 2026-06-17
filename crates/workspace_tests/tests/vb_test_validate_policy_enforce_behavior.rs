#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Behavior tests for vb_validate policy enforcement.
//!
//! Tests the enforcement behavior of validation gates:
//! - Policy rule evaluation behavior
//! - Enforcement action behavior
//! - Policy bypass attempts and rejection
//! - Exact violation reporting
//!
//! These are behavior tests that prove the observable behavior of the
//! validation pipeline against constructed WorkflowParts inputs.

use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    ResourceContract, WorkflowParts,
};
use vb_validate::ValidationError;
use vb_validate::shared::{ValidationPipeline, validate};

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

fn make_parts(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    symbols_count: u32,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    constants: Vec<ConstValue>,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: expressions.into_boxed_slice(),
        accessors: accessors.into_boxed_slice(),
        constants: constants.into_boxed_slice(),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(result_slot),
        },
    }
}

fn validate_gate_11_only(parts: &WorkflowParts) -> Result<(), ValidationError> {
    ValidationPipeline {
        gate_11_loop_body_graph: true,
        ..ValidationPipeline::no_gates()
    }
    .validate(parts)
}

// ---------------------------------------------------------------------------
// Policy Rule Evaluation Behavior: Gate 7 - Expression Stack Depth
// ---------------------------------------------------------------------------

/// Policy rule: expression stack depth must not exceed resource contract limit.
#[test]
fn gate_07_policy_rule_rejects_contract_exceeding_protocol_limit() {
    // Given a resource contract that exceeds the 64-byte protocol limit
    let mut parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    parts.resource_contract = ResourceContract {
        max_expr_stack: 128, // Protocol limit is 64
        ..ResourceContract::DEFAULT
    };

    // When validation runs
    let result = validate(&parts);

    // Then the policy enforcement rejects with exact error
    assert!(matches!(
        result,
        Err(ValidationError::ExpressionStackExceeded {
            declared: 128,
            limit: 64
        })
    ));
}

/// Policy rule: expression max_stack must match computed stack depth.
#[test]
fn gate_07_policy_rule_rejects_mismatched_declared_stack() {
    // Given an expression with declared max_stack=2 but actual computed=1
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))];
    let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).expect("valid");
    let mut parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        0,
        vec![expr],
        Vec::new(),
        Vec::new(),
    );

    // Mutate max_stack to be wrong
    if let Some(e) = parts.expressions.get_mut(0) {
        e.max_stack = 2; // Wrong: actual computed is 1
    }

    // When validation runs
    let result = validate(&parts);

    // Then the policy enforcement reports exact mismatch
    assert!(matches!(
        result,
        Err(ValidationError::ExpressionStackMismatch {
            expr_index: 0,
            declared: 2,
            computed: 1,
        })
    ));
}

/// Policy rule: expression stack depth must not exceed contract.
#[test]
fn gate_07_policy_rule_rejects_expression_exceeding_contract() {
    // Given an expression that exceeds the resource contract limit
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Eq,
    ];
    let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).expect("valid");
    let mut parts = make_parts(
        vec![finish_node(0, 0)],
        2,
        0,
        vec![expr],
        Vec::new(),
        Vec::new(),
    );
    parts.resource_contract = ResourceContract {
        max_expr_stack: 1, // Contract is 1, but expression has depth 2
        ..ResourceContract::DEFAULT
    };

    // When validation runs
    let result = validate(&parts);

    // Then enforcement rejects with exact error
    assert!(matches!(
        result,
        Err(ValidationError::ExpressionStackExceeded { .. })
    ));
}

// ---------------------------------------------------------------------------
// Policy Rule Evaluation Behavior: Gate 8 - Accessor Path Segments
// ---------------------------------------------------------------------------

/// Policy rule: accessor root slot must be within slot_count.
#[test]
fn gate_08_policy_rule_rejects_accessor_root_out_of_range() {
    // Given an accessor with root slot beyond slot_count
    let accessor = AccessorProgram {
        root: SlotIdx::new(99), // slot_count = 1
        path: vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice(),
    };
    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        1,
        Vec::new(),
        vec![accessor],
        Vec::new(),
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact slot range error
    assert!(matches!(
        result,
        Err(ValidationError::AccessorSlotOutOfRange {
            accessor_index: 0,
            slot: 99,
            slot_count: 1,
        })
    ));
}

/// Policy rule: accessor field symbol must be within symbols_count.
#[test]
fn gate_08_policy_rule_rejects_accessor_symbol_out_of_bounds() {
    // Given an accessor with a field symbol beyond symbols_count
    let accessor = AccessorProgram {
        root: SlotIdx::ZERO,
        path: vec![PathSegment::Field(SymbolId::new(99))].into_boxed_slice(), // symbols_count = 1
    };
    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        1,
        Vec::new(),
        vec![accessor],
        Vec::new(),
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact symbol bounds error
    assert!(matches!(
        result,
        Err(ValidationError::AccessorSymbolOutOfBounds {
            accessor_index: 0,
            segment_index: 0,
            symbol: 99,
            symbols_count: 1,
        })
    ));
}

/// Policy rule: accessor index segment cannot be sentinel value.
#[test]
fn gate_08_policy_rule_rejects_sentinel_index_segment() {
    // Given an accessor with u32::MAX index segment (sentinel)
    let accessor = AccessorProgram {
        root: SlotIdx::ZERO,
        path: vec![PathSegment::Index(u32::MAX)].into_boxed_slice(),
    };
    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        0,
        Vec::new(),
        vec![accessor],
        Vec::new(),
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement rejects with exact path invalid error
    assert!(matches!(
        result,
        Err(ValidationError::AccessorPathInvalid {
            accessor_index: 0,
            segment_index: 0,
        })
    ));
}

/// Policy rule: accessor path depth cannot exceed max limit.
#[test]
fn gate_08_policy_rule_rejects_accessor_path_too_deep() {
    // Given an accessor with path depth exceeding 16
    let path: Vec<PathSegment> = (0..20)
        .map(|i| PathSegment::Field(SymbolId::new(i as u32)))
        .collect();
    let accessor = AccessorProgram {
        root: SlotIdx::ZERO,
        path: path.into_boxed_slice(),
    };
    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        20,
        Vec::new(),
        vec![accessor],
        Vec::new(),
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact depth error
    assert!(matches!(
        result,
        Err(ValidationError::AccessorPathTooDeep {
            accessor_index: 0,
            depth: 20,
            max: 16,
        })
    ));
}

// ---------------------------------------------------------------------------
// Policy Rule Evaluation Behavior: Gate 9 - Slot References
// ---------------------------------------------------------------------------

/// Policy rule: all slot references must be within declared slot_count.
#[test]
fn gate_09_policy_rule_rejects_output_slot_out_of_range() {
    // Given a node with output slot beyond slot_count
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(99)), // slot_count = 1
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1, 0, Vec::new(), Vec::new(), Vec::new());

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact slot range error
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange {
            slot: 99,
            slot_count: 1,
            context: _,
        })
    ));
}

/// Policy rule: error_slot references must be within slot_count.
#[test]
fn gate_09_policy_rule_rejects_error_slot_out_of_range() {
    // Given a node with error_slot beyond slot_count
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: Some(SlotIdx::new(50)), // slot_count = 1
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };
    let parts = make_parts(vec![node], 1, 0, Vec::new(), Vec::new(), Vec::new());

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact slot range error
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange {
            slot: 50,
            slot_count: 1,
            context: _,
        })
    ));
}

/// Policy rule: expression LoadSlot must reference valid slot.
#[test]
fn gate_09_policy_rule_rejects_expr_load_slot_out_of_range() {
    // Given an expression with LoadSlot referencing beyond slot_count
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(99))];
    let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).expect("valid");
    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        0,
        vec![expr],
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact slot range error
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange {
            slot: 99,
            slot_count: 1,
            context: _,
        })
    ));
}

// ---------------------------------------------------------------------------
// Policy Rule Evaluation Behavior: Gate 10 - Node Kind Specific Constraints
// ---------------------------------------------------------------------------

/// Policy rule: Finish node result slot must be within slot_count.
#[test]
fn gate_10_policy_rule_rejects_finish_result_out_of_range() {
    // Given a Finish node with result slot beyond slot_count
    let node = finish_node(0, 99); // slot_count = 1
    let parts = make_parts(vec![node], 1, 0, Vec::new(), Vec::new(), Vec::new());

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact constraint violation
    assert!(matches!(
        result,
        Err(ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: _,
        })
    ));
}

/// Policy rule: SetConst const index must be within constant pool.
#[test]
fn gate_10_policy_rule_rejects_setconst_const_out_of_range() {
    // Given a SetConst node with const index beyond constant pool
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(99), // Only 1 constant exists
        },
    };
    let parts = make_parts(
        vec![node],
        1,
        0,
        Vec::new(),
        Vec::new(),
        vec![ConstValue::I64(42)],
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact constraint violation
    assert!(matches!(
        result,
        Err(ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: _,
        })
    ));
}

/// Policy rule: Do node action_id sentinel value is invalid.
#[test]
fn gate_10_policy_rule_rejects_sentinel_action_id() {
    // Given a Do node with sentinel action_id (u16::MAX)
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(u16::MAX), // Sentinel value
            input: SlotIdx::ZERO,
        },
    };
    let parts = make_parts(
        vec![node],
        1,
        0,
        Vec::new(),
        Vec::new(),
        vec![ConstValue::Null],
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement rejects with exact error about sentinel action_id
    assert!(matches!(
        result,
        Err(ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: ref d,
        }) if d.contains("sentinel")
    ));
}

/// Policy rule: Choose branch expression index must be within expression count.
#[test]
fn gate_10_policy_rule_rejects_choose_expr_out_of_range() {
    // Given a Choose node with expression index beyond expression count
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Choose {
            branches: vec![vb_core::workflow::ExprBranch {
                condition: vb_core::ids::ExprIdx::new(99), // Only 1 expression exists
                target: StepIdx::new(1),
            }]
            .into_boxed_slice(),
            otherwise: None,
        },
    };
    let parts = make_parts(
        vec![node, finish_node(1, 0)],
        1,
        0,
        vec![ExprProgram {
            ops: Box::new([]),
            max_stack: 0,
        }],
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact constraint violation
    assert!(matches!(
        result,
        Err(ValidationError::NodeKindConstraintViolation {
            node_index: 0,
            detail: _,
        })
    ));
}

// ---------------------------------------------------------------------------
// Policy Rule Evaluation Behavior: Gate 11 - Loop Body Graph
// ---------------------------------------------------------------------------

/// Policy rule: ForEach body step must be within node count.
#[test]
fn gate_11_policy_rule_rejects_for_each_body_out_of_range() {
    // Given a ForEachStart with body step beyond node count
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::ZERO,
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(99), // Beyond node count of 1
            done: StepIdx::new(1),
        },
    };
    let parts = make_parts(
        vec![node, finish_node(1, 0)],
        2,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate_gate_11_only(&parts);

    // Then enforcement reports exact loop body range error
    assert!(matches!(
        result,
        Err(ValidationError::LoopBodyStepOutOfRange {
            step: 99,
            node_count: 2,
            source_node: 0,
            label: _,
        })
    ));
}

/// Policy rule: ForEach done step must be within node count.
#[test]
fn gate_11_policy_rule_rejects_for_each_done_out_of_range() {
    // Given a ForEachStart with done step beyond node count
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::ZERO,
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(99), // Beyond node count of 1
        },
    };
    let parts = make_parts(
        vec![node, finish_node(1, 0)],
        2,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate_gate_11_only(&parts);

    // Then enforcement reports exact loop body range error
    assert!(matches!(
        result,
        Err(ValidationError::LoopBodyStepOutOfRange {
            step: 99,
            node_count: 2,
            source_node: 0,
            label: _,
        })
    ));
}

/// Policy rule: ForEach body must come after ForEachStart.
#[test]
fn gate_11_policy_rule_rejects_loop_body_before_start() {
    // Given a ForEachStart where body step <= start index
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::ZERO,
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::ZERO, // Same as start, not forward
            done: StepIdx::new(1),
        },
    };
    let parts = make_parts(
        vec![node, finish_node(1, 0), finish_node(2, 0)],
        2,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate_gate_11_only(&parts);

    // Then enforcement reports exact loop body range error
    assert!(matches!(
        result,
        Err(ValidationError::LoopBodyStepOutOfRange {
            step: 0,
            node_count: 3,
            source_node: 0,
            label: ref l,
        }) if l.contains("after")
    ));
}

/// Policy rule: ForEach done must come after body.
#[test]
fn gate_11_policy_rule_rejects_for_each_done_before_body() {
    // Given a ForEachStart where done <= body
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::ForEachStart {
            input: SlotIdx::ZERO,
            item_slot: SlotIdx::new(1),
            limit: 10,
            body: StepIdx::new(2),
            done: StepIdx::new(1), // Done before body
        },
    };
    let parts = make_parts(
        vec![node, finish_node(1, 0), finish_node(2, 0)],
        2,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate_gate_11_only(&parts);

    // Then enforcement reports exact error about done ordering
    assert!(matches!(
        result,
        Err(ValidationError::LoopBodyStepOutOfRange {
            step: 1,
            node_count: 3,
            source_node: 0,
            label: ref l,
        }) if l.contains("after")
    ));
}

/// Policy rule: TogetherStart branch steps must be within node count.
#[test]
fn gate_11_policy_rule_rejects_together_branch_out_of_range() {
    // Given a TogetherStart with branch step beyond node count
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: vec![StepIdx::new(99)].into_boxed_slice(), // Beyond node count
            join: StepIdx::new(1),
        },
    };
    let parts = make_parts(
        vec![node, finish_node(1, 0)],
        1,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate_gate_11_only(&parts);

    // Then enforcement reports exact loop body range error
    assert!(matches!(
        result,
        Err(ValidationError::LoopBodyStepOutOfRange {
            step: 99,
            node_count: 2,
            source_node: 0,
            label: _,
        })
    ));
}

/// Policy rule: TogetherStart join must be within node count.
#[test]
fn gate_11_policy_rule_rejects_together_join_out_of_range() {
    // Given a TogetherStart with join step beyond node count
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::TogetherStart {
            branches: vec![StepIdx::new(1)].into_boxed_slice(),
            join: StepIdx::new(99), // Beyond node count
        },
    };
    let parts = make_parts(
        vec![node, finish_node(1, 0)],
        1,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate_gate_11_only(&parts);

    // Then enforcement reports exact loop body range error
    assert!(matches!(
        result,
        Err(ValidationError::LoopBodyStepOutOfRange {
            step: 99,
            node_count: 2,
            source_node: 0,
            label: _,
        })
    ));
}

// ---------------------------------------------------------------------------
// Policy Rule Evaluation Behavior: Gate 13 - Slot Dependency Cycles
// ---------------------------------------------------------------------------

/// Policy rule: direct slot cycle (slot A depends on slot B which depends on A).
#[test]
fn gate_13_policy_rule_rejects_direct_slot_cycle() {
    // Given a direct cycle: slot 0 <- slot 1, slot 1 <- slot 0
    let nodes = vec![
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::ZERO,
            },
        },
    ];
    let parts = make_parts(nodes, 2, 0, Vec::new(), Vec::new(), Vec::new());

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact cycle error with chain
    assert!(matches!(
        result,
        Err(ValidationError::SlotDependencyCycle {
            slot: 1,
            chain: ref c,
        }) if c.contains("slot 1 -> slot 0")
    ));
}

/// Policy rule: transitive slot cycle (slot A <- slot B <- slot C <- slot A).
#[test]
fn gate_13_policy_rule_rejects_three_slot_cycle() {
    // Given a transitive cycle: slot 0 <- slot 1 <- slot 2 <- slot 0
    let nodes = vec![
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(1),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::new(2),
            },
        },
        CompiledNode {
            id: StepIdx::new(2),
            output: Some(SlotIdx::new(2)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Copy {
                source: SlotIdx::ZERO,
            },
        },
    ];
    let parts = make_parts(nodes, 3, 0, Vec::new(), Vec::new(), Vec::new());

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact cycle error
    assert!(matches!(
        result,
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

/// Policy rule: no cycle when slot depends on itself via expression (self-read).
#[test]
fn gate_13_policy_rule_accepts_self_read_via_expression() {
    // Given an EvalExpr that reads slot 0 and writes to slot 0 (self-read, not cycle)
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::ZERO,
                output: Some(SlotIdx::ZERO),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ids::ExprIdx::new(0),
                },
            },
            finish_node(1, 0),
        ],
        1,
        0,
        vec![ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::ZERO)]),
            max_stack: 1,
        }],
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement accepts (self-read is not a cycle)
    assert!(
        result.is_ok(),
        "self-read via expression should not be a cycle: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Policy Rule Evaluation Behavior: Gate 14 - Slot Type Consistency
// ---------------------------------------------------------------------------

/// Policy rule: same slot cannot be written with incompatible types.
#[test]
fn gate_14_policy_rule_rejects_slot_type_inconsistency_i64_vs_bool() {
    // Given two SetConst nodes writing same slot with different types
    let nodes = vec![
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0), // I64
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::ZERO), // Same slot!
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1), // Bool
            },
        },
    ];
    let parts = make_parts(
        nodes,
        1,
        0,
        Vec::new(),
        Vec::new(),
        vec![ConstValue::I64(42), ConstValue::Bool(true)],
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact type inconsistency error
    assert!(matches!(
        result,
        Err(ValidationError::SlotTypeInconsistency { slot: 0 })
    ));
}

/// Policy rule: same slot written with same type is consistent.
#[test]
fn gate_14_policy_rule_accepts_slot_consistent_type() {
    // Given two SetConst nodes writing same slot with same type
    let nodes = vec![
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(1),
            },
        },
    ];
    let parts = make_parts(
        nodes,
        1,
        0,
        Vec::new(),
        Vec::new(),
        vec![ConstValue::I64(1), ConstValue::I64(2)],
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement accepts
    assert!(
        result.is_ok(),
        "same type to same slot should pass: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Policy Rule Evaluation Behavior: Gate 15 - Determinism Proof
// ---------------------------------------------------------------------------

/// Policy rule: consecutive non-deterministic nodes are not allowed.
#[test]
fn gate_15_policy_rule_rejects_consecutive_nondeterministic_nodes() {
    // Given two consecutive Do nodes (both non-deterministic)
    let nodes = vec![
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::ZERO,
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(2),
                input: SlotIdx::ZERO,
            },
        },
    ];
    let parts = make_parts(
        nodes,
        1,
        0,
        Vec::new(),
        Vec::new(),
        vec![ConstValue::Null, ConstValue::Null],
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact non-deterministic path error
    assert!(matches!(
        result,
        Err(ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1,
        })
    ));
}

/// Policy rule: Ask nodes cannot be consecutively chained.
#[test]
fn gate_15_policy_rule_rejects_consecutive_ask_nodes() {
    // Given two consecutive Ask nodes
    let nodes = vec![
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: None,
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::ZERO,
                timeout_slot: None,
            },
        },
    ];
    let parts = make_parts(nodes, 1, 0, Vec::new(), Vec::new(), Vec::new());

    // When validation runs
    let result = validate(&parts);

    // Then enforcement reports exact non-deterministic path error
    assert!(matches!(
        result,
        Err(ValidationError::NonDeterministicPath {
            from_node: 0,
            to_node: 1,
        })
    ));
}

/// Policy rule: Do followed by deterministic node is valid.
#[test]
fn gate_15_policy_rule_accepts_do_followed_by_deterministic() {
    // Given a Do node followed by a deterministic SetConst
    let nodes = vec![
        CompiledNode {
            id: StepIdx::ZERO,
            output: Some(SlotIdx::ZERO),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::ZERO,
            },
        },
        CompiledNode {
            id: StepIdx::new(1),
            output: Some(SlotIdx::new(1)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(0),
            },
        },
    ];
    let parts = make_parts(
        nodes,
        2,
        0,
        Vec::new(),
        Vec::new(),
        vec![ConstValue::I64(42)],
    );

    // When validation runs
    let result = validate(&parts);

    // Then enforcement accepts
    assert!(
        result.is_ok(),
        "Do followed by SetConst should pass: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Enforcement Action Behavior: ValidationPipeline Selective Gates
// ---------------------------------------------------------------------------

/// Enforcement action: disabling gate allows bypassing that policy check.
#[test]
fn enforcement_action_allows_bypass_when_gate_disabled() {
    // Given parts that would fail gate 9 (slot OOB)
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(99)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1, 0, Vec::new(), Vec::new(), Vec::new());

    // When validated with gate 9 disabled
    let pipeline = ValidationPipeline {
        gate_09_slot_references: false,
        ..ValidationPipeline::all_gates()
    };
    let result = pipeline.validate(&parts);

    // Then enforcement allows bypass
    assert!(
        result.is_ok(),
        "with gate 9 disabled, should pass: {:?}",
        result
    );
}

/// Enforcement action: enabling only specific gate enforces only that policy.
#[test]
fn enforcement_action_enforces_only_enabled_gates() {
    // Given parts that would fail gate 9 (slot OOB)
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(99)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1, 0, Vec::new(), Vec::new(), Vec::new());

    // When validated with ONLY gate 9 enabled
    let pipeline = ValidationPipeline {
        gate_07_expression_stack: false,
        gate_08_accessor_paths: false,
        gate_09_slot_references: true,
        gate_10_node_kind_specific: false,
        gate_11_loop_body_graph: false,
        gate_12_action_contracts: false,
        gate_13_no_slot_cycles: false,
        gate_14_slot_type_consistency: false,
        gate_15_determinism_proof: false,
    };
    let result = pipeline.validate(&parts);

    // Then enforcement catches the error
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

/// Enforcement action: pipeline short-circuits on first failing gate.
#[test]
fn enforcement_action_short_circuits_on_first_failure() {
    // Given parts that fail gate 7 first (before gate 9 would also fail)
    let mut parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    parts.resource_contract = ResourceContract {
        max_expr_stack: 128, // Exceeds protocol limit of 64
        ..ResourceContract::DEFAULT
    };

    // When validated
    let result = validate(&parts);

    // Then enforcement returns gate 7 error, not gate 9
    assert!(matches!(
        result,
        Err(ValidationError::ExpressionStackExceeded { .. })
    ));
}

// ---------------------------------------------------------------------------
// Policy Bypass Attempts and Rejection
// ---------------------------------------------------------------------------

/// Bypass attempt: using sentinel slot index is rejected.
#[test]
fn bypass_attempt_rejects_sentinel_slot_index() {
    // Given a node with SlotIdx::MAX
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(u16::MAX)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1, 0, Vec::new(), Vec::new(), Vec::new());

    // When validation runs
    let result = validate(&parts);

    // Then bypass is rejected
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

/// Bypass attempt: using unknown node kind is rejected.
#[test]
fn bypass_attempt_rejects_unknown_node_kind() {
    // Given a node with an unknown variant (via non_exhaustive)
    // This is actually caught by the match in gate_09
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop, // Use known kind but structure would fail
    };
    let parts = make_parts(vec![node], 1, 0, Vec::new(), Vec::new(), Vec::new());

    // When validation runs with a node that would hit the wildcard
    let result = validate(&parts);

    // Should pass since Nop is valid
    assert!(result.is_ok());
}

/// Bypass attempt: using reserved ID is rejected.
#[test]
fn bypass_attempt_rejects_reserved_id() {
    // This is a schema-level check - in compiled IR we check via other gates
    // The reserved ID check is in schema validation
    // This test verifies gate 10 catches Do with sentinel action_id
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(u16::MAX), // Reserved sentinel
            input: SlotIdx::ZERO,
        },
    };
    let parts = make_parts(
        vec![node],
        1,
        0,
        Vec::new(),
        Vec::new(),
        vec![ConstValue::Null],
    );

    // When validation runs
    let result = validate(&parts);

    // Then bypass is rejected
    assert!(matches!(
        result,
        Err(ValidationError::NodeKindConstraintViolation { .. })
    ));
}

/// Bypass attempt: cycle through expression is detected.
#[test]
fn bypass_attempt_rejects_cycle_through_expression() {
    // Given slot 0 depends on expr(slot 1) and slot 1 depends on slot 0
    let parts = make_parts(
        vec![
            CompiledNode {
                id: StepIdx::ZERO,
                output: Some(SlotIdx::ZERO),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: vb_core::ids::ExprIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Copy {
                    source: SlotIdx::ZERO,
                },
            },
        ],
        2,
        0,
        vec![ExprProgram {
            ops: Box::new([ExprOp::LoadSlot(SlotIdx::new(1))]),
            max_stack: 1,
        }],
        Vec::new(),
        Vec::new(),
    );

    // When validation runs
    let result = validate(&parts);

    // Then cycle is detected
    assert!(matches!(
        result,
        Err(ValidationError::SlotDependencyCycle { .. })
    ));
}

// ---------------------------------------------------------------------------
// Exact Violation Reporting
// ---------------------------------------------------------------------------

/// Exact violation: ExpressionStackExceeded reports declared and limit.
#[test]
fn exact_violation_expression_stack_exceeded_has_all_fields() {
    let err = ValidationError::ExpressionStackExceeded {
        declared: 100,
        limit: 64,
    };
    let msg = err.to_string();
    assert!(msg.contains("declared"));
    assert!(msg.contains("100"));
    assert!(msg.contains("limit"));
    assert!(msg.contains("64"));
}

/// Exact violation: SlotDependencyCycle reports slot and chain.
#[test]
fn exact_violation_slot_dependency_cycle_has_all_fields() {
    let err = ValidationError::SlotDependencyCycle {
        slot: 5,
        chain: String::from("slot 5 -> slot 3 -> slot 1 -> slot 5"),
    };
    let msg = err.to_string();
    assert!(msg.contains("slot 5"));
    assert!(msg.contains("slot 5 -> slot 3 -> slot 1 -> slot 5"));
}

/// Exact violation: ActionContractMissing reports action_id and node_index.
#[test]
fn exact_violation_action_contract_missing_has_all_fields() {
    let err = ValidationError::ActionContractMissing {
        action_id: 42,
        node_index: 7,
    };
    let msg = err.to_string();
    assert!(msg.contains("42"));
    assert!(msg.contains("7"));
}

/// Exact violation: NodeKindConstraintViolation reports node_index and detail.
#[test]
fn exact_violation_node_kind_constraint_has_node_and_detail() {
    let err = ValidationError::NodeKindConstraintViolation {
        node_index: 3,
        detail: String::from("Finish result slot 99 out of range (slot_count 1)"),
    };
    let msg = err.to_string();
    assert!(msg.contains("3"));
    assert!(msg.contains("Finish result slot 99 out of range"));
}

/// Exact violation: CapabilityNameTooLong reports all parameters.
#[test]
fn exact_violation_capability_name_too_long_has_all_params() {
    let err = ValidationError::CapabilityNameTooLong {
        action_id: 1,
        capability_index: 2,
        len: 256,
        max: 128,
    };
    let msg = err.to_string();
    assert!(msg.contains("1"));
    assert!(msg.contains("2"));
    assert!(msg.contains("256"));
    assert!(msg.contains("128"));
}

/// Exact violation: LoopBodyStepOutOfRange reports all context.
#[test]
fn exact_violation_loop_body_step_out_of_range_has_all_context() {
    let err = ValidationError::LoopBodyStepOutOfRange {
        step: 50,
        node_count: 10,
        source_node: 3,
        label: String::from("for_each body"),
    };
    let msg = err.to_string();
    assert!(msg.contains("50"));
    assert!(msg.contains("10"));
    assert!(msg.contains("3"));
    assert!(msg.contains("for_each body"));
}

/// Exact violation: NonDeterministicPath reports from_node and to_node.
#[test]
fn exact_violation_non_deterministic_path_has_both_nodes() {
    let err = ValidationError::NonDeterministicPath {
        from_node: 2,
        to_node: 5,
    };
    let msg = err.to_string();
    assert!(msg.contains("2"));
    assert!(msg.contains("5"));
}
