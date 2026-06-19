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
    clippy::enum_variant_names,
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
    unused_variables
)]
#![forbid(unsafe_code)]
//! Integration tests for vb_validate policy enforcement edge cases.
//!
//! Tests validation gate enforcement on edge cases that require full
//! WorkflowParts construction and pipeline execution.

use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, SymbolId};
use vb_core::value::ConstValue;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, ExprOp, ExprProgram, PathSegment,
    ResourceContract, WorkflowParts,
};
use vb_validate::ValidationError;
use vb_validate::shared::{ValidationPipeline, validate};

// ---------------------------------------------------------------------------
// Helpers
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

fn nop_node(index: u16) -> CompiledNode {
    CompiledNode {
        id: StepIdx::new(index),
        output: None,
        next: Some(StepIdx::new(index.saturating_add(1))),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    }
}

// ---------------------------------------------------------------------------
// Gate 7: Expression stack depth edge cases
// ---------------------------------------------------------------------------

/// Gate 7: expression stack depth mismatch at declared boundary.
#[test]
fn gate_7_rejects_expression_stack_mismatch_at_boundary() {
    // Build an expression with actual max_stack = 2 but declared = 1
    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Eq,
    ];
    let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).expect("valid");
    let parts = make_parts(
        vec![finish_node(0, 0)],
        2,
        0,
        vec![expr],
        Vec::new(),
        Vec::new(),
    );

    // Mutate max_stack to be wrong
    let mut parts = parts;
    if let Some(e) = parts.expressions.get_mut(0) {
        e.max_stack = 1; // Wrong: actual is 2
    }

    let result = validate(&parts);
    assert!(matches!(
        result,
        Err(ValidationError::ExpressionStackMismatch { .. })
    ));
}

/// Gate 7: empty expressions pass.
#[test]
fn gate_7_accepts_empty_expressions() {
    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let result = validate(&parts);
    assert!(
        result.is_ok(),
        "empty expressions should pass gate 7: {:?}",
        result
    );
}

/// Gate 7: single-slot load passes.
#[test]
fn gate_7_accepts_single_slot_load() {
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0))];
    let expr = ExprProgram::try_from_ops(ops.into_boxed_slice()).expect("valid");
    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        0,
        vec![expr],
        Vec::new(),
        Vec::new(),
    );
    let result = validate(&parts);
    assert!(result.is_ok(), "single slot load should pass: {:?}", result);
}

// ---------------------------------------------------------------------------
// Gate 8: Accessor path edge cases
// ---------------------------------------------------------------------------

/// Gate 8: accessor path depth exactly at limit passes.
#[test]
fn gate_8_accepts_accessor_path_at_max_depth() {
    // Build an accessor with depth = 16 (exactly at typical max)
    let path: Vec<PathSegment> = (0..16)
        .map(|i| PathSegment::Field(SymbolId::new(i as u32)))
        .collect();

    let accessor = AccessorProgram {
        root: SlotIdx::ZERO,
        path: path.into_boxed_slice(),
    };

    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        16, // symbols_count matches path length
        Vec::new(),
        vec![accessor],
        Vec::new(),
    );

    let result = validate(&parts);
    assert!(
        result.is_ok(),
        "accessor at max depth should pass: {:?}",
        result
    );
}

/// Gate 8: accessor root slot out of bounds fails.
#[test]
fn gate_8_rejects_accessor_root_out_of_bounds() {
    let accessor = AccessorProgram {
        root: SlotIdx::new(99), // slot_count = 1, so this is OOB
        path: vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice(),
    };

    let parts = make_parts(
        vec![finish_node(0, 0)],
        1, // slot_count
        1,
        Vec::new(),
        vec![accessor],
        Vec::new(),
    );

    let result = validate(&parts);
    assert!(matches!(
        result,
        Err(ValidationError::AccessorSlotOutOfRange { .. })
    ));
}

/// Gate 8: accessor symbol out of bounds fails.
#[test]
fn gate_8_rejects_accessor_symbol_out_of_bounds() {
    let accessor = AccessorProgram {
        root: SlotIdx::ZERO,
        path: vec![PathSegment::Field(SymbolId::new(99))].into_boxed_slice(),
    };

    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        1, // symbols_count = 1, so symbol 99 is OOB
        Vec::new(),
        vec![accessor],
        Vec::new(),
    );

    let result = validate(&parts);
    assert!(matches!(
        result,
        Err(ValidationError::AccessorSymbolOutOfBounds { .. })
    ));
}

// ---------------------------------------------------------------------------
// Gate 9: Slot reference edge cases
// ---------------------------------------------------------------------------

/// Gate 9: slot reference exactly at slot_count boundary passes.
#[test]
fn gate_9_accepts_slot_reference_at_upper_bound() {
    // slot_count = 3, valid slots are 0, 1, 2
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(2)), // Last valid slot
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };

    let parts = make_parts(vec![node], 3, 0, Vec::new(), Vec::new(), Vec::new());
    let result = validate(&parts);
    assert!(
        result.is_ok(),
        "slot at upper bound should pass: {:?}",
        result
    );
}

/// Gate 9: slot reference out of bounds fails.
#[test]
fn gate_9_rejects_slot_reference_out_of_bounds() {
    // slot_count = 2, slot 3 is OOB
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(3)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };

    let parts = make_parts(vec![node], 2, 0, Vec::new(), Vec::new(), Vec::new());
    let result = validate(&parts);
    assert!(matches!(
        result,
        Err(ValidationError::SlotReferenceOutOfRange { .. })
    ));
}

/// Gate 9: error_slot reference out of bounds fails.
#[test]
fn gate_9_rejects_error_slot_reference_out_of_bounds() {
    // slot_count = 1, error_slot = 5 is OOB
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: Some(SlotIdx::new(5)),
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::ZERO,
        },
    };

    let parts = make_parts(vec![node], 1, 0, Vec::new(), Vec::new(), Vec::new());
    let result = validate(&parts);
    assert!(
        matches!(result, Err(ValidationError::SlotReferenceOutOfRange { .. })),
        "expected Gate 9 slot reference error, got {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Gate 10: Node-kind-specific edge cases
// ---------------------------------------------------------------------------

/// Gate 10: finish node with valid result slot passes.
#[test]
fn gate_10_accepts_finish_node_with_valid_result() {
    let parts = make_parts(
        vec![finish_node(0, 0)],
        1,
        0,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let result = validate(&parts);
    assert!(
        result.is_ok(),
        "finish with valid slot should pass: {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Gate 13: Slot dependency cycle edge cases
// ---------------------------------------------------------------------------

/// Gate 13: no cycles in linear workflow passes.
#[test]
fn gate_13_accepts_linear_workflow_with_no_cycles() {
    let nodes = vec![nop_node(0), nop_node(1), finish_node(2, 0)];
    let parts = make_parts(nodes, 1, 0, Vec::new(), Vec::new(), Vec::new());
    let result = validate(&parts);
    assert!(result.is_ok(), "linear workflow should pass: {:?}", result);
}

// ---------------------------------------------------------------------------
// Gate 14: Slot type consistency edge cases
// ---------------------------------------------------------------------------

/// Gate 14: single slot written with consistent type passes.
#[test]
fn gate_14_accepts_single_slot_consistent_type() {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
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
    let result = validate(&parts);
    assert!(
        result.is_ok(),
        "consistent slot type should pass: {:?}",
        result
    );
}

/// Gate 14: slot written with mixed types fails.
#[test]
fn gate_14_rejects_slot_written_with_mixed_types() {
    // Two nodes writing the same slot with different types
    let node_a = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::ZERO),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0), // I64
        },
    };
    let node_b = CompiledNode {
        id: StepIdx::new(1),
        output: Some(SlotIdx::ZERO), // Same slot!
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(1), // Bool (different type)
        },
    };

    let parts = make_parts(
        vec![node_a, node_b],
        1,
        0,
        Vec::new(),
        Vec::new(),
        vec![ConstValue::I64(1), ConstValue::Bool(true)],
    );

    let result = validate(&parts);
    assert!(matches!(
        result,
        Err(ValidationError::SlotTypeInconsistency { .. })
    ));
}

// ---------------------------------------------------------------------------
// ValidationPipeline edge cases
// ---------------------------------------------------------------------------

/// ValidationPipeline can disable specific gates individually.
#[test]
fn validation_pipeline_selective_gate_disable() {
    // Build parts that would fail gate 9 (slot OOB)
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: Some(SlotIdx::new(99)),
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Nop,
    };
    let parts = make_parts(vec![node], 1, 0, Vec::new(), Vec::new(), Vec::new());

    // With gate 9 disabled, validation should pass
    let pipeline = ValidationPipeline {
        gate_09_slot_references: false,
        ..ValidationPipeline::all_gates()
    };
    let result = pipeline.validate(&parts);
    assert!(
        result.is_ok(),
        "with gate 9 disabled, OOB slot should pass: {:?}",
        result
    );
}

/// ValidationPipeline::all_gates enables all gates.
#[test]
fn validation_pipeline_all_gates_enabled() {
    let pipeline = ValidationPipeline::all_gates();
    assert!(pipeline.gate_07_expression_stack);
    assert!(pipeline.gate_08_accessor_paths);
    assert!(pipeline.gate_09_slot_references);
    assert!(pipeline.gate_10_node_kind_specific);
    assert!(pipeline.gate_11_loop_body_graph);
    assert!(pipeline.gate_12_action_contracts);
    assert!(pipeline.gate_13_no_slot_cycles);
    assert!(pipeline.gate_14_slot_type_consistency);
    assert!(pipeline.gate_15_determinism_proof);
}

/// ValidationPipeline::no_gates disables all gates.
#[test]
fn validation_pipeline_no_gates_disabled() {
    let pipeline = ValidationPipeline::no_gates();
    assert!(!pipeline.gate_07_expression_stack);
    assert!(!pipeline.gate_08_accessor_paths);
    assert!(!pipeline.gate_09_slot_references);
    assert!(!pipeline.gate_10_node_kind_specific);
    assert!(!pipeline.gate_11_loop_body_graph);
    assert!(!pipeline.gate_12_action_contracts);
    assert!(!pipeline.gate_13_no_slot_cycles);
    assert!(!pipeline.gate_14_slot_type_consistency);
    assert!(!pipeline.gate_15_determinism_proof);
}

/// ValidationPipeline copy is independent.
#[test]
fn validation_pipeline_copy_is_independent() {
    let pipeline_a = ValidationPipeline::all_gates();
    let mut pipeline_b = pipeline_a;
    pipeline_b.gate_09_slot_references = false;

    assert!(pipeline_a.gate_09_slot_references);
    assert!(!pipeline_b.gate_09_slot_references);
}

/// ValidationPipeline default is all_gates.
#[test]
fn validation_pipeline_default_is_all_gates() {
    let default = ValidationPipeline::default();
    let all = ValidationPipeline::all_gates();
    assert_eq!(
        default.gate_07_expression_stack,
        all.gate_07_expression_stack
    );
    assert_eq!(default.gate_09_slot_references, all.gate_09_slot_references);
    assert_eq!(
        default.gate_15_determinism_proof,
        all.gate_15_determinism_proof
    );
}

// ---------------------------------------------------------------------------
// ValidationError Display edge cases
// ---------------------------------------------------------------------------

/// ValidationError::DuplicateKey Display.
#[test]
fn validation_error_duplicate_key_display() {
    let err = ValidationError::DuplicateKey;
    let msg = err.to_string();
    assert!(
        msg.contains("DUPLICATE_KEY"),
        "should contain DUPLICATE_KEY: {msg}"
    );
}

/// ValidationError::ForbiddenYamlFeature Display.
#[test]
fn validation_error_forbidden_yaml_feature_display() {
    let err = ValidationError::ForbiddenYamlFeature;
    let msg = err.to_string();
    assert!(
        msg.contains("FORBIDDEN_YAML_FEATURE"),
        "should contain FORBIDDEN_YAML_FEATURE: {msg}"
    );
}

/// ValidationError::MissingRequiredField Display with field name.
#[test]
fn validation_error_missing_required_field_display() {
    let err = ValidationError::MissingRequiredField {
        field: String::from("name"),
    };
    let msg = err.to_string();
    assert!(msg.contains("name"), "should contain field name: {msg}");
}

/// ValidationError::TypeMismatch Display with expected and found.
#[test]
fn validation_error_type_mismatch_display() {
    let err = ValidationError::TypeMismatch {
        expected: String::from("i64"),
        found: String::from("bool"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("i64") && msg.contains("bool"),
        "should contain types: {msg}"
    );
}

/// ValidationError::SlotDependencyCycle Display with slot and chain.
#[test]
fn validation_error_slot_dependency_cycle_display() {
    let err = ValidationError::SlotDependencyCycle {
        slot: 3,
        chain: String::from("0→1→2→3"),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("3") && msg.contains("0→1→2→3"),
        "should contain cycle info: {msg}"
    );
}

/// ValidationError::ExpressionStackExceeded Display.
#[test]
fn validation_error_expression_stack_exceeded_display() {
    let err = ValidationError::ExpressionStackExceeded {
        declared: 5,
        limit: 4,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("5") && msg.contains("4"),
        "should contain stack info: {msg}"
    );
}

/// ValidationError::ActionContractMissing Display.
#[test]
fn validation_error_action_contract_missing_display() {
    let err = ValidationError::ActionContractMissing {
        action_id: 7,
        node_index: 3,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("7") && msg.contains("3"),
        "should contain action/node info: {msg}"
    );
}

/// ValidationError::NonDeterministicPath Display.
#[test]
fn validation_error_non_deterministic_path_display() {
    let err = ValidationError::NonDeterministicPath {
        from_node: 2,
        to_node: 5,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("2") && msg.contains("5"),
        "should contain node info: {msg}"
    );
}
