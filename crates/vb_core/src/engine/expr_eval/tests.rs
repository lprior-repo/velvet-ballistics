//! Tests for expression evaluation.

use crate::errors::EngineError;
use crate::ids::{ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::limits::MAX_EXPRESSION_STACK;
use crate::value::{ConstValue, SlotValue};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts, check_expr_stack_bound,
};

use super::eval_expr_with_store;

fn empty_plan_with_expr(
    ops: Box<[ExprOp]>,
    constants: Box<[ConstValue]>,
) -> Result<CompiledWorkflow, EngineError> {
    let max_stack = check_expr_stack_bound(&ops, MAX_EXPRESSION_STACK).map_err(|_| {
        EngineError::InvalidCompiledWorkflow {
            reason: "stack check failed",
        }
    })?;
    let expr = ExprProgram::try_from_parts(ops, max_stack).map_err(|_| {
        EngineError::InvalidCompiledWorkflow {
            reason: "expr parts",
        }
    })?;
    CompiledWorkflow::try_from_parts(WorkflowParts {
        name: "test".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            kind: CompiledNodeKind::Nop,
            next: None,
            on_error: None,
            error_slot: None,
            output: None,
        }]
        .into(),
        expressions: vec![expr].into(),
        accessors: vec![].into(),
        constants,
        slot_count: 8,
        symbols_count: 10,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|_| EngineError::InvalidCompiledWorkflow {
        reason: "workflow parts",
    })
}

fn run_frame_with_slots(slots: Vec<SlotValue>) -> Result<crate::RunFrame, EngineError> {
    let mut run = crate::RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 8)?;
    for (i, value) in slots.iter().enumerate() {
        let idx = SlotIdx::new(i as u16);
        run.write_slot(idx, *value)?;
    }
    Ok(run)
}

fn eval_expr_ops_with_constants(
    ops: &[ExprOp],
    constants: Vec<ConstValue>,
    store: &mut ValueStore,
) -> Result<SlotValue, EngineError> {
    let plan = empty_plan_with_expr(ops.into(), constants.into())?;
    let run = run_frame_with_slots(vec![])?;
    let (value, _) = eval_expr_with_store(&plan, &run, store, ExprIdx::new(0))?;
    Ok(value)
}

fn eval_expr_ops_with_store(
    ops: &[ExprOp],
    slots: Vec<SlotValue>,
    constants: Vec<ConstValue>,
    store: &mut ValueStore,
) -> Result<SlotValue, EngineError> {
    let plan = empty_plan_with_expr(ops.into(), constants.into())?;
    let run = run_frame_with_slots(slots)?;
    let (value, _) = eval_expr_with_store(&plan, &run, store, ExprIdx::new(0))?;
    Ok(value)
}

#[test]
fn contains_finds_substring() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let haystack = store.insert_symbol("hello world")?;
    let needle = store.insert_symbol("world")?;
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Contains,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Symbol(haystack), ConstValue::Symbol(needle)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn contains_rejects_missing_substring() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let haystack = store.insert_symbol("hello")?;
    let needle = store.insert_symbol("xyz")?;
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Contains,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Symbol(haystack), ConstValue::Symbol(needle)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(false));
    Ok(())
}

#[test]
fn starts_with_matches_prefix() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let text = store.insert_symbol("hello world")?;
    let prefix = store.insert_symbol("hello")?;
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::StartsWith,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Symbol(text), ConstValue::Symbol(prefix)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn ends_with_matches_suffix() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let text = store.insert_symbol("hello world")?;
    let suffix = store.insert_symbol("world")?;
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::EndsWith,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::Symbol(text), ConstValue::Symbol(suffix)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn has_finds_element_in_list() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)].into_boxed_slice(),
    )?;

    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Has,
    ];
    let result = eval_expr_ops_with_store(
        &ops,
        vec![SlotValue::List(list)],
        vec![ConstValue::I64(20)],
        &mut store,
    )?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn exists_checks_object_field() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let sym = store.insert_symbol("key")?;
    let obj = store.insert_object(
        vec![ObjectField {
            key: sym,
            value: SlotValue::Bool(true),
        }]
        .into_boxed_slice(),
    )?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Exists];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::Object(obj)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn length_counts_list_items() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
    )?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Length];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::I64(3));
    Ok(())
}

#[test]
fn empty_detects_empty_list() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![].into_boxed_slice())?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Empty];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::Bool(true));
    Ok(())
}

#[test]
fn append_adds_to_list() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![SlotValue::I64(1)].into_boxed_slice())?;

    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::Append,
    ];
    let result = eval_expr_ops_with_store(
        &ops,
        vec![SlotValue::List(list)],
        vec![ConstValue::I64(2)],
        &mut store,
    )?;
    let result_list_id = match result {
        SlotValue::List(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "list",
                found: other.type_name(),
            });
        }
    };
    let items = store.list(result_list_id)?;
    assert_eq!(items.len(), 2);
    Ok(())
}

#[test]
fn append_if_conditionally_adds() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![SlotValue::I64(1)].into_boxed_slice())?;

    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::AppendIf,
    ];
    let result = eval_expr_ops_with_store(
        &ops,
        vec![SlotValue::List(list)],
        vec![ConstValue::I64(2), ConstValue::Bool(true)],
        &mut store,
    )?;
    let result_list_id = match result {
        SlotValue::List(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "list",
                found: other.type_name(),
            });
        }
    };
    let items = store.list(result_list_id)?;
    assert_eq!(items.len(), 2);
    Ok(())
}

#[test]
fn merge_combines_objects() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let sym1 = store.insert_symbol("a")?;
    let sym2 = store.insert_symbol("b")?;
    let obj1 = store.insert_object(
        vec![ObjectField {
            key: sym1,
            value: SlotValue::I64(1),
        }]
        .into_boxed_slice(),
    )?;
    let obj2 = store.insert_object(
        vec![ObjectField {
            key: sym2,
            value: SlotValue::I64(2),
        }]
        .into_boxed_slice(),
    )?;

    let ops = vec![
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Merge,
    ];
    let result = eval_expr_ops_with_store(
        &ops,
        vec![SlotValue::Object(obj1), SlotValue::Object(obj2)],
        vec![],
        &mut store,
    )?;
    let result_obj_id = match result {
        SlotValue::Object(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "object",
                found: other.type_name(),
            });
        }
    };
    let merged = store.object(result_obj_id)?;
    assert_eq!(merged.len(), 2);
    Ok(())
}

#[test]
fn sum_computes_list_total() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)].into_boxed_slice(),
    )?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::I64(6));
    Ok(())
}

#[test]
fn count_computes_list_length() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice())?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Count];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    assert_eq!(result, SlotValue::I64(2));
    Ok(())
}

#[test]
fn unique_removes_duplicates() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list = store.insert_list(
        vec![SlotValue::I64(1), SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice(),
    )?;

    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Unique];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store)?;
    let result_list_id = match result {
        SlotValue::List(id) => id,
        other => {
            return Err(EngineError::TypeMismatch {
                expected: "list",
                found: other.type_name(),
            });
        }
    };
    let items = store.list(result_list_id)?;
    assert_eq!(items.len(), 2);
    Ok(())
}

// ===== Security regression tests =====

#[test]
fn div_i64_min_div_neg_one_returns_overflow_not_division_by_zero() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(i64::MIN), ConstValue::I64(-1)],
        &mut store,
    );
    let Err(EngineError::InvalidCompiledWorkflow { reason }) = result else {
        return Err(EngineError::TypeMismatch {
            expected: "InvalidCompiledWorkflow (overflow)",
            found: "wrong error variant",
        });
    };
    assert!(
        reason.contains("overflow"),
        "reason should mention overflow, got: {reason}"
    );
    Ok(())
}

#[test]
fn div_by_zero_still_returns_division_by_zero_error() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let ops = vec![
        ExprOp::LoadConst(ConstIdx::new(0)),
        ExprOp::LoadConst(ConstIdx::new(1)),
        ExprOp::Div,
    ];
    let result = eval_expr_ops_with_constants(
        &ops,
        vec![ConstValue::I64(42), ConstValue::I64(0)],
        &mut store,
    );
    assert_eq!(result, Err(EngineError::DivisionByZero));
    Ok(())
}

#[test]
fn sum_overflow_on_individual_element_is_detected() -> Result<(), EngineError> {
    let mut store = ValueStore::new();
    let list =
        store.insert_list(vec![SlotValue::I64(i64::MAX), SlotValue::I64(1)].into_boxed_slice())?;
    let ops = vec![ExprOp::LoadSlot(SlotIdx::new(0)), ExprOp::Sum];
    let result = eval_expr_ops_with_store(&ops, vec![SlotValue::List(list)], vec![], &mut store);
    let Err(EngineError::InvalidCompiledWorkflow { reason }) = result else {
        return Err(EngineError::TypeMismatch {
            expected: "InvalidCompiledWorkflow (sum overflow)",
            found: "no error or wrong error",
        });
    };
    assert!(
        reason.contains("overflow"),
        "reason should mention overflow, got: {reason}"
    );
    Ok(())
}
