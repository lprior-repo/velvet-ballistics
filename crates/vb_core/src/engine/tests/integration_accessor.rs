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
    unused_variables,
)]

#![forbid(unsafe_code)]
//! Integration tests for accessor evaluation.

use crate::RunFrame;
use crate::errors::EngineError;
use crate::ids::{
    AccessorIdx, BlobId, ExprIdx, ListId, ObjectId, RunId, SlotIdx, StepIdx, SymbolId,
    WorkflowDigest,
};
use crate::value::{SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram,
    PathSegment, WorkflowParts,
};

use crate::engine::{
    EngineSignal, StepBudget, eval_accessor, eval_accessor_with_store, new_run_frame,
    run_until_blocked,
};

fn test_store() -> ValueStore {
    ValueStore::new()
}

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

fn test_frame(run_id: RunId, workflow: &CompiledWorkflow) -> Result<crate::RunFrame, String> {
    new_run_frame(run_id, workflow).map_err(|error| error.to_string())
}

#[test]
fn public_eval_accessor_loads_root_value() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(24), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
        .map_err(|error| error.to_string())?;

    let value =
        eval_accessor(&workflow, &run, AccessorIdx::new(0)).map_err(|error| error.to_string())?;

    ensure_equal(value, SlotValue::I64(77))?;
    Ok(())
}

#[test]
fn eval_accessor_identity_path_returns_root_handle_without_store() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(120), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(42)),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    let value =
        eval_accessor(&workflow, &run, AccessorIdx::new(0)).map_err(|error| error.to_string())?;

    ensure_equal(value, SlotValue::Object(ObjectId::new(42)))?;
    Ok(())
}

#[test]
fn public_eval_accessor_rejects_invalid_accessor_index() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
    let run = test_frame(RunId::new(27), &workflow)?;

    match eval_accessor(&workflow, &run, AccessorIdx::new(1)) {
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "accessor index out of bounds",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn load_accessor_with_empty_path_loads_root_slot() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(15), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(77), Taint::Clean)
        .map_err(|error| error.to_string())?;
    let mut store = test_store();

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    if result == EngineSignal::Finished(SlotValue::I64(77), Taint::Clean) {
        Ok(())
    } else {
        Err(format!("unexpected result: {result:?}"))
    }
}

#[test]
fn public_eval_accessor_reports_typed_error_without_store() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(16), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(0)),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    match eval_accessor(&workflow, &run, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "object",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn load_accessor_reads_object_field_through_store() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(7))].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(28), &workflow)?;
    let mut store = test_store();
    let object = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(7),
                value: SlotValue::I64(123),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(object), Taint::Clean)
        .map_err(|error| error.to_string())?;

    let result = run_until_blocked(&workflow, &mut run, StepBudget::MAX, &mut store)
        .map_err(|error| error.to_string())?;

    ensure_equal(
        result,
        EngineSignal::Finished(SlotValue::I64(123), Taint::Clean),
    )?;
    Ok(())
}

#[test]
fn eval_accessor_reads_list_item_through_store() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(1)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(29), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|error| error.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|error| error.to_string())?;

    ensure_equal(value, SlotValue::I64(2))?;
    Ok(())
}

#[test]
fn eval_accessor_reports_missing_field_precisely() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(9))].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(30), &workflow)?;
    let mut store = test_store();
    let object = store
        .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(object), Taint::Clean)
        .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ObjectFieldNotFound { field }) if field == SymbolId::new(9) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_reports_list_index_precisely() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(4)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(31), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ListIndexOutOfBounds { index: 4 }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_rejects_field_traversal_on_scalar_value() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(7))].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(121), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(11), Taint::Clean)
        .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_reports_object_handle_bounds() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(3))].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(122), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Object(ObjectId::new(99)),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ObjectOutOfBounds { object }) if object == ObjectId::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_reports_list_handle_bounds() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())
        .map_err(|error| error.to_string())?;
    let mut run = test_frame(RunId::new(123), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::List(ListId::new(88)),
        Taint::Clean,
    )
    .map_err(|error| error.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ListOutOfBounds { list }) if list == ListId::new(88) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

fn accessor_workflow(path: Box<[PathSegment]>) -> Result<CompiledWorkflow, crate::WorkflowError> {
    accessor_workflow_with_opts(path, 2, SlotIdx::new(0), 100)
}

fn accessor_workflow_with_opts(
    path: Box<[PathSegment]>,
    slot_count: u16,
    root: SlotIdx,
    symbols_count: u32,
) -> Result<CompiledWorkflow, crate::WorkflowError> {
    let expression = ExprProgram::try_from_ops(
        vec![ExprOp::LoadAccessor(AccessorIdx::new(0))].into_boxed_slice(),
    )
    .map_err(crate::WorkflowError::Expression)?;
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("accessor"),
        digest: WorkflowDigest::from_bytes([8; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(1),
                },
            },
        ]
        .into_boxed_slice(),
        expressions: vec![expression].into_boxed_slice(),
        accessors: vec![AccessorProgram { root, path }].into_boxed_slice(),
        constants: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: crate::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
}

// =====================================================================
// 1. Empty path returns root for all scalar types
// =====================================================================

#[test]
fn eval_accessor_with_store_empty_path_returns_root_null() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(50), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Null, Taint::Clean)
        .map_err(|e| e.to_string())?;
    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Null)
}

#[test]
fn eval_accessor_with_store_empty_path_returns_root_bool() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(51), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Bool(false), Taint::Clean)
        .map_err(|e| e.to_string())?;
    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Bool(false))
}

#[test]
fn eval_accessor_with_store_empty_path_returns_root_f64() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(52), &workflow)?;
    let mut store = test_store();
    let finite = crate::FiniteF64::new(3.14).map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::F64(finite), Taint::Clean)
        .map_err(|e| e.to_string())?;
    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::F64(finite))
}

#[test]
fn eval_accessor_with_store_empty_path_returns_root_symbol() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(53), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Symbol(SymbolId::new(7)),
        Taint::Clean,
    )
    .map_err(|e| e.to_string())?;
    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Symbol(SymbolId::new(7)))
}

#[test]
fn eval_accessor_with_store_empty_path_returns_root_blob() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(54), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Blob(BlobId::new(3)),
        Taint::Clean,
    )
    .map_err(|e| e.to_string())?;
    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Blob(BlobId::new(3)))
}

#[test]
fn eval_accessor_empty_path_returns_root_null_without_store() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(55), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Null, Taint::Clean)
        .map_err(|e| e.to_string())?;
    let value = eval_accessor(&workflow, &run, AccessorIdx::new(0)).map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Null)
}

#[test]
fn eval_accessor_empty_path_returns_root_symbol_without_store() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(56), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Symbol(SymbolId::new(5)),
        Taint::Clean,
    )
    .map_err(|e| e.to_string())?;
    let value = eval_accessor(&workflow, &run, AccessorIdx::new(0)).map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Symbol(SymbolId::new(5)))
}

#[test]
fn eval_accessor_empty_path_returns_root_blob_without_store() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(57), &workflow)?;
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Blob(BlobId::new(9)),
        Taint::Clean,
    )
    .map_err(|e| e.to_string())?;
    let value = eval_accessor(&workflow, &run, AccessorIdx::new(0)).map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Blob(BlobId::new(9)))
}

// =====================================================================
// 2. Object field retrieval: nested, sub-object
// =====================================================================

#[test]
fn eval_accessor_nested_object_field_traversal() -> Result<(), String> {
    let workflow = accessor_workflow(
        vec![
            PathSegment::Field(SymbolId::new(1)),
            PathSegment::Field(SymbolId::new(2)),
        ]
        .into_boxed_slice(),
    )
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(60), &workflow)?;
    let mut store = test_store();

    let inner_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(2),
                value: SlotValue::I64(999),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let outer_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(1),
                value: SlotValue::Object(inner_obj),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(outer_obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(999))
}

#[test]
fn eval_accessor_sub_object_returns_the_object() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(1))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(61), &workflow)?;
    let mut store = test_store();

    let inner_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(2),
                value: SlotValue::I64(42),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let outer_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(1),
                value: SlotValue::Object(inner_obj),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(outer_obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Object(inner_obj))
}

// =====================================================================
// 3. List index: last
// =====================================================================

#[test]
fn eval_accessor_list_last_index() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(2)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(62), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(3))
}

// =====================================================================
// 4. Deep nested traversal
// =====================================================================

#[test]
fn eval_accessor_three_level_field_traversal_abc() -> Result<(), String> {
    let workflow = accessor_workflow(
        vec![
            PathSegment::Field(SymbolId::new(10)),
            PathSegment::Field(SymbolId::new(11)),
            PathSegment::Field(SymbolId::new(12)),
        ]
        .into_boxed_slice(),
    )
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(63), &workflow)?;
    let mut store = test_store();

    let leaf_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(12),
                value: SlotValue::I64(555),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let mid_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(11),
                value: SlotValue::Object(leaf_obj),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let root_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(10),
                value: SlotValue::Object(mid_obj),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(root_obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(555))
}

#[test]
fn eval_accessor_field_then_index_traversal() -> Result<(), String> {
    let workflow = accessor_workflow(
        vec![PathSegment::Field(SymbolId::new(5)), PathSegment::Index(1)].into_boxed_slice(),
    )
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(64), &workflow)?;
    let mut store = test_store();

    let inner_list = store
        .insert_list(
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(5),
                value: SlotValue::List(inner_list),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(20))
}

#[test]
fn eval_accessor_index_then_field_traversal() -> Result<(), String> {
    let workflow = accessor_workflow(
        vec![PathSegment::Index(0), PathSegment::Field(SymbolId::new(3))].into_boxed_slice(),
    )
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(65), &workflow)?;
    let mut store = test_store();

    let inner_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(3),
                value: SlotValue::I64(777),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let list = store
        .insert_list(vec![SlotValue::Object(inner_obj)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(777))
}

// =====================================================================
// 5. Error: null.field, symbol.field, blob.field
// =====================================================================

#[test]
fn eval_accessor_rejects_field_on_null() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(66), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Null, Taint::Clean)
        .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "null",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_rejects_field_on_symbol() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(67), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Symbol(SymbolId::new(1)),
        Taint::Clean,
    )
    .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "symbol",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_rejects_field_on_blob() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(68), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Blob(BlobId::new(1)),
        Taint::Clean,
    )
    .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "field",
            found: "blob",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =====================================================================
// 6. Error: null[0], i64[0], blob[0]
// =====================================================================

#[test]
fn eval_accessor_rejects_index_on_null() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(69), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Null, Taint::Clean)
        .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "index",
            found: "null",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_rejects_index_on_i64() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(70), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::I64(42), Taint::Clean)
        .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "index",
            found: "number",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_rejects_index_on_blob() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(71), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(
        SlotIdx::new(0),
        SlotValue::Blob(BlobId::new(2)),
        Taint::Clean,
    )
    .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::UnsupportedAccessorTraversal {
            segment: "index",
            found: "blob",
        }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =====================================================================
// 7. Error: missing field in nested object, OOB index in nested list
// =====================================================================

#[test]
fn eval_accessor_missing_field_in_nested_object_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(
        vec![
            PathSegment::Field(SymbolId::new(1)),
            PathSegment::Field(SymbolId::new(99)),
        ]
        .into_boxed_slice(),
    )
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(72), &workflow)?;
    let mut store = test_store();

    let inner_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(2),
                value: SlotValue::I64(1),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let outer_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(1),
                value: SlotValue::Object(inner_obj),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(outer_obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ObjectFieldNotFound { field }) if field == SymbolId::new(99) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_oob_index_in_nested_list_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(
        vec![PathSegment::Field(SymbolId::new(1)), PathSegment::Index(5)].into_boxed_slice(),
    )
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(73), &workflow)?;
    let mut store = test_store();

    let inner_list = store
        .insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(1),
                value: SlotValue::List(inner_list),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ListIndexOutOfBounds { index: 5 }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

// =====================================================================
// 8. Error: large index on empty list
// =====================================================================

#[test]
fn eval_accessor_large_index_on_empty_list_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(1_000_000)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(74), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(vec![SlotValue::I64(1)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ListIndexOutOfBounds { .. }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_large_index_on_truly_empty_list_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(1_000_000)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(75), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(Vec::<SlotValue>::new().into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ListIndexOutOfBounds { .. }) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn accessor_workflow_rejects_u32_max_index_as_reserved_value() -> Result<(), String> {
    match accessor_workflow(vec![PathSegment::Index(u32::MAX)].into_boxed_slice()) {
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("u32::MAX") || msg.contains("reserved") {
                Ok(())
            } else {
                Err(format!("unexpected error: {msg}"))
            }
        }
        Ok(_) => Err("expected u32::MAX index rejection".into()),
    }
}

// =====================================================================
// 9. Determinism: same input → same output
// =====================================================================

#[test]
fn eval_accessor_is_deterministic_for_field_access() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(3))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(76), &workflow)?;
    let mut store = test_store();
    let obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(3),
                value: SlotValue::I64(42),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let mut store2 = test_store();
    // Re-insert the same object into store2
    let obj2 = store2
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(3),
                value: SlotValue::I64(42),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let mut run2 = test_frame(RunId::new(77), &workflow)?;
    run2.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj2), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let r1 = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    let r2 = eval_accessor_with_store(&workflow, &run2, &mut store2, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(r1, r2)?;
    ensure_equal(r1, SlotValue::I64(42))
}

#[test]
fn eval_accessor_is_deterministic_for_list_access() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(78), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(vec![SlotValue::I64(99)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let r1 = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    let r2 = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(r1, r2)?;
    ensure_equal(r1, SlotValue::I64(99))
}

#[test]
fn eval_accessor_deterministic_error_same_as_same_error() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(9))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(79), &workflow)?;
    let mut store = test_store();
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Null, Taint::Clean)
        .map_err(|e| e.to_string())?;

    let r1 = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
    let r2 = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
    match (&r1, &r2) {
        (
            Err(EngineError::UnsupportedAccessorTraversal {
                segment: "field",
                found: "null",
            }),
            Err(EngineError::UnsupportedAccessorTraversal {
                segment: "field",
                found: "null",
            }),
        ) => Ok(()),
        other => Err(format!("unexpected determinism failure: {other:?}")),
    }
}

// =====================================================================
// 10. Chain of fields returns leaf (max depth), 32 fields rejected
// =====================================================================

#[test]
fn eval_accessor_chain_of_16_fields_returns_leaf() -> Result<(), String> {
    let chain_len: usize = 16;
    let mut store = test_store();
    let leaf_value = SlotValue::I64(1337);

    let mut current = leaf_value;
    let field_ids: Vec<SymbolId> = (0..chain_len).map(|i| SymbolId::new(i as u32)).collect();

    for i in (0..chain_len).rev() {
        let obj = store
            .insert_object(
                vec![ObjectField {
                    key: field_ids[i],
                    value: current,
                    taint: Taint::Clean,
                }]
                .into_boxed_slice(),
            )
            .map_err(|e| e.to_string())?;
        current = SlotValue::Object(obj);
    }

    let root_obj = if let SlotValue::Object(o) = current {
        o
    } else {
        return Err("expected object".into());
    };

    let path: Vec<PathSegment> = field_ids.into_iter().map(PathSegment::Field).collect();
    let workflow = accessor_workflow(path.into_boxed_slice()).map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(80), &workflow)?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(root_obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(1337))
}

#[test]
fn accessor_workflow_rejects_path_depth_32_exceeds_maximum() -> Result<(), String> {
    let path: Vec<PathSegment> = (0..32)
        .map(|i| PathSegment::Field(SymbolId::new(i)))
        .collect();
    match accessor_workflow(path.into_boxed_slice()) {
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("accessor path depth") && msg.contains("16") {
                Ok(())
            } else {
                Err(format!("unexpected error: {msg}"))
            }
        }
        Ok(_) => Err("expected path depth rejection".into()),
    }
}

// =====================================================================
// 11. Two indexes in a row: list[i][j]
// =====================================================================

#[test]
fn eval_accessor_two_indexes_in_a_row() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Index(1), PathSegment::Index(0)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(81), &workflow)?;
    let mut store = test_store();

    let inner_list = store
        .insert_list(
            vec![
                SlotValue::I64(100),
                SlotValue::I64(200),
                SlotValue::I64(300),
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let outer_list = store
        .insert_list(
            vec![
                SlotValue::I64(10),
                SlotValue::List(inner_list),
                SlotValue::I64(30),
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(outer_list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(100))
}

#[test]
fn eval_accessor_two_indexes_index_0_then_index_1() -> Result<(), String> {
    let workflow =
        accessor_workflow(vec![PathSegment::Index(0), PathSegment::Index(1)].into_boxed_slice())
            .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(82), &workflow)?;
    let mut store = test_store();

    let inner_list = store
        .insert_list(
            vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let outer_list = store
        .insert_list(vec![SlotValue::List(inner_list), SlotValue::I64(1)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(outer_list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(20))
}

// =====================================================================
// 12. Null inside field/list returns null
// =====================================================================

#[test]
fn eval_accessor_null_inside_field_returns_null() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(1))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(83), &workflow)?;
    let mut store = test_store();
    let obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(1),
                value: SlotValue::Null,
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Null)
}

#[test]
fn eval_accessor_null_inside_list_returns_null() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(0)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(84), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(vec![SlotValue::Null, SlotValue::I64(1)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Null)
}

#[test]
fn eval_accessor_null_inside_nested_object_returns_null() -> Result<(), String> {
    let workflow = accessor_workflow(
        vec![
            PathSegment::Field(SymbolId::new(1)),
            PathSegment::Field(SymbolId::new(2)),
        ]
        .into_boxed_slice(),
    )
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(85), &workflow)?;
    let mut store = test_store();

    let inner_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(2),
                value: SlotValue::Null,
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let outer_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(1),
                value: SlotValue::Object(inner_obj),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(outer_obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Null)
}

// =====================================================================
// 13. Root slot out of bounds
// =====================================================================

#[test]
fn eval_accessor_root_slot_out_of_bounds_returns_error() -> Result<(), String> {
    // Root slot 10 with slot_count 10 is OOB at workflow construction time.
    let workflow_result = accessor_workflow_with_opts(Box::new([]), 10, SlotIdx::new(10), 10);
    match workflow_result {
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("outside slot_count") {
                return Ok(());
            }
            Err(format!("unexpected error: {msg}"))
        }
        Ok(workflow) => {
            let run = test_frame(RunId::new(86), &workflow)?;
            match eval_accessor(&workflow, &run, AccessorIdx::new(0)) {
                Err(_) => Ok(()),
                other => Err(format!("unexpected result: {other:?}")),
            }
        }
    }
}

#[test]
fn eval_accessor_with_store_root_slot_out_of_bounds_returns_error() -> Result<(), String> {
    let workflow_result = accessor_workflow_with_opts(Box::new([]), 10, SlotIdx::new(10), 10);
    match workflow_result {
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("outside slot_count") {
                return Ok(());
            }
            Err(format!("unexpected error: {msg}"))
        }
        Ok(workflow) => {
            let run = test_frame(RunId::new(87), &workflow)?;
            let mut store = test_store();
            match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
                Err(_) => Ok(()),
                other => Err(format!("unexpected result: {other:?}")),
            }
        }
    }
}

// =====================================================================
// Additional edge-case tests
// =====================================================================

#[test]
fn eval_accessor_non_empty_path_on_uninitialized_slot_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(0))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let run = test_frame(RunId::new(88), &workflow)?;
    let mut store = test_store();

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_empty_path_on_uninitialized_slot_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
    let run = test_frame(RunId::new(89), &workflow)?;
    let mut store = test_store();

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(0) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_index_beyond_list_len_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(3)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(90), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(
            vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ListIndexOutOfBounds { index }) if index == 3 => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_field_on_empty_object_returns_error() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(5))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(91), &workflow)?;
    let mut store = test_store();
    let obj = store
        .insert_object(Vec::<ObjectField>::new().into_boxed_slice())
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    match eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0)) {
        Err(EngineError::ObjectFieldNotFound { field }) if field == SymbolId::new(5) => Ok(()),
        other => Err(format!("unexpected result: {other:?}")),
    }
}

#[test]
fn eval_accessor_many_field_object_retrieves_correct_value() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(4))].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(92), &workflow)?;
    let mut store = test_store();
    let obj = store
        .insert_object(
            vec![
                ObjectField {
                    key: SymbolId::new(0),
                    value: SlotValue::I64(10),
                    taint: Taint::Clean,
                },
                ObjectField {
                    key: SymbolId::new(1),
                    value: SlotValue::I64(20),
                    taint: Taint::Clean,
                },
                ObjectField {
                    key: SymbolId::new(2),
                    value: SlotValue::I64(30),
                    taint: Taint::Clean,
                },
                ObjectField {
                    key: SymbolId::new(3),
                    value: SlotValue::I64(40),
                    taint: Taint::Clean,
                },
                ObjectField {
                    key: SymbolId::new(4),
                    value: SlotValue::I64(50),
                    taint: Taint::Clean,
                },
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(50))
}

#[test]
fn eval_accessor_list_with_many_items_retrieves_correct_element() -> Result<(), String> {
    let workflow = accessor_workflow(vec![PathSegment::Index(4)].into_boxed_slice())
        .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(93), &workflow)?;
    let mut store = test_store();
    let list = store
        .insert_list(
            vec![
                SlotValue::I64(0),
                SlotValue::I64(1),
                SlotValue::I64(2),
                SlotValue::I64(3),
                SlotValue::I64(4),
                SlotValue::I64(5),
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::I64(4))
}

#[test]
fn eval_accessor_multiple_field_list_interleaving() -> Result<(), String> {
    let workflow = accessor_workflow(
        vec![
            PathSegment::Field(SymbolId::new(1)),
            PathSegment::Index(2),
            PathSegment::Field(SymbolId::new(8)),
        ]
        .into_boxed_slice(),
    )
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(RunId::new(94), &workflow)?;
    let mut store = test_store();

    let leaf_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(8),
                value: SlotValue::Bool(true),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let inner_list = store
        .insert_list(
            vec![
                SlotValue::Null,
                SlotValue::Null,
                SlotValue::Object(leaf_obj),
            ]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    let root_obj = store
        .insert_object(
            vec![ObjectField {
                key: SymbolId::new(1),
                value: SlotValue::List(inner_list),
                taint: Taint::Clean,
            }]
            .into_boxed_slice(),
        )
        .map_err(|e| e.to_string())?;
    run.write_slot_with_taint(SlotIdx::new(0), SlotValue::Object(root_obj), Taint::Clean)
        .map_err(|e| e.to_string())?;

    let value = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0))
        .map_err(|e| e.to_string())?;
    ensure_equal(value, SlotValue::Bool(true))
}

// =====================================================================
// 14. Proptest: valid access never panics, deterministic, empty=identity, invalid always errors
// =====================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::engine::eval_accessor;
    use crate::engine::eval_accessor_with_store;
    use crate::ids::RunId;
    use proptest::prelude::*;

    fn build_test_run_store(
        run_id_val: u64,
        root_value: SlotValue,
    ) -> Result<(CompiledWorkflow, RunFrame, ValueStore), String> {
        let workflow = accessor_workflow(Box::new([])).map_err(|e| e.to_string())?;
        let mut run = test_frame(RunId::new(run_id_val), &workflow)?;
        let store = test_store();
        run.write_slot_with_taint(SlotIdx::new(0), root_value, Taint::Clean)
            .map_err(|e| e.to_string())?;
        Ok((workflow, run, store))
    }

    fn arb_scalar_value() -> impl proptest::strategy::Strategy<Value = SlotValue> {
        prop_oneof![
            Just(SlotValue::Null),
            any::<bool>().prop_map(SlotValue::Bool),
            any::<i64>().prop_map(SlotValue::I64),
            (0u32..1000).prop_map(|id| SlotValue::Symbol(SymbolId::new(id))),
            (0u32..1000).prop_map(|id| SlotValue::List(ListId::new(id))),
            (0u32..1000).prop_map(|id| SlotValue::Object(ObjectId::new(id))),
            (0u64..1000).prop_map(|id| SlotValue::Blob(BlobId::new(id))),
        ]
    }

    fn arb_bool_i64_null() -> impl proptest::strategy::Strategy<Value = SlotValue> {
        prop_oneof![
            Just(SlotValue::Null),
            any::<bool>().prop_map(SlotValue::Bool),
            any::<i64>().prop_map(SlotValue::I64),
        ]
    }

    proptest! {
        #[test]
        fn empty_path_is_identity_for_all_scalar_types(val in arb_scalar_value()) {
            let (workflow, run, mut store) = build_test_run_store(1, val).unwrap();
            let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
            prop_assert!(
                matches!(result, Ok(ref v) if v == &val),
                "empty path access must return Ok({val:?}), got {:?}",
                result
            );
        }
    }

    proptest! {
        #[test]
        fn empty_path_identity_without_store(val in arb_scalar_value()) {
            let (workflow, run, _store) = build_test_run_store(2, val).unwrap();
            let result = eval_accessor(&workflow, &run, AccessorIdx::new(0));
            prop_assert!(
                matches!(result, Ok(ref v) if v == &val),
                "empty path access without store must return Ok({val:?}), got {:?}",
                result
            );
        }
    }

    proptest! {
        #[test]
        fn empty_path_is_deterministic(val in arb_bool_i64_null()) {
            let (workflow, run, mut store) = build_test_run_store(3, val).unwrap();
            let r1 = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
            let r2 = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
            prop_assert_eq!(r1, r2, "two accesses with same input must produce same result");
        }
    }

    proptest! {
        #[test]
        fn invalid_index_always_errors(
            index in (1u32..u32::MAX),
        ) {
            let workflow = accessor_workflow(vec![PathSegment::Index(index)].into_boxed_slice()).unwrap();
            let mut run = test_frame(RunId::new(4), &workflow).unwrap();
            let mut store = test_store();
            let list = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice()).unwrap();
            run.write_slot_with_taint(SlotIdx::new(0), SlotValue::List(list), Taint::Clean).unwrap();

            let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
            prop_assert!(
                matches!(result, Err(EngineError::ListIndexOutOfBounds { index: idx }) if idx == index),
                "out-of-bounds index {} on empty list must return ListIndexOutOfBounds, got {:?}",
                index, result
            );
        }
    }

    proptest! {
        #[test]
        fn field_on_non_object_always_errors(
            val in prop_oneof![
                Just(SlotValue::Null),
                any::<bool>().prop_map(SlotValue::Bool),
                any::<i64>().prop_map(SlotValue::I64),
                (0u32..1000).prop_map(|id| SlotValue::Symbol(SymbolId::new(id))),
                (0u32..1000).prop_map(|id| SlotValue::List(ListId::new(id))),
                (0u64..1000).prop_map(|id| SlotValue::Blob(BlobId::new(id))),
            ]
        ) {
            let workflow = accessor_workflow(vec![PathSegment::Field(SymbolId::new(1))].into_boxed_slice()).unwrap();
            let mut run = test_frame(RunId::new(5), &workflow).unwrap();
            let mut store = test_store();
            run.write_slot_with_taint(SlotIdx::new(0), val, Taint::Clean).unwrap();

            let result = eval_accessor_with_store(&workflow, &run, &mut store, AccessorIdx::new(0));
            prop_assert!(
                matches!(result, Err(EngineError::UnsupportedAccessorTraversal { segment: "field", .. })),
                "field access on non-object must return UnsupportedAccessorTraversal, got {:?}",
                result
            );
        }
    }
}
