use super::step_once;
use super::test_support::{ensure_equal, test_frame};
use crate::EngineSignal;
use crate::ids::{ConstIdx, ExprIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use crate::value::{ConstValue, SlotValue, Taint};
use crate::value_store::{ObjectField, ValueStore};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts,
};

#[test]
fn step_once_eval_expr_writes_result_to_output_slot() -> Result<(), String> {
    let expr = ExprProgram::try_from_ops(
        vec![
            ExprOp::LoadConst(ConstIdx::new(0)),
            ExprOp::LoadConst(ConstIdx::new(1)),
            ExprOp::Add,
        ]
        .into_boxed_slice(),
    )
    .map_err(crate::WorkflowError::Expression)
    .map_err(|e| e.to_string())?;

    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("eval_step_test"),
        digest: WorkflowDigest::from_bytes([0x66; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::EvalExpr {
                    expr: ExprIdx::new(0),
                },
            },
            finish_node(StepIdx::new(1), SlotIdx::new(0)),
        ]
        .into_boxed_slice(),
        expressions: vec![expr].into_boxed_slice(),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(19), ConstValue::I64(23)].into_boxed_slice(),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    let result = step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?;

    ensure_equal(result, EngineSignal::Continue)?;
    ensure_equal(
        *run.read_slot(SlotIdx::new(0)).map_err(|e| e.to_string())?,
        SlotValue::I64(42),
    )
}

#[test]
fn step_once_build_object_writes_object_to_output_slot() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("build_obj_step"),
        digest: WorkflowDigest::from_bytes([0x77; 32]),
        nodes: vec![
            set_const_node(StepIdx::new(0), SlotIdx::new(0), StepIdx::new(1)),
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildObject {
                    fields: vec![(SymbolId::new(1), SlotIdx::new(0))].into_boxed_slice(),
                },
            },
            finish_node(StepIdx::new(2), SlotIdx::new(1)),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(100)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 2,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    ensure_equal(
        step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?,
        EngineSignal::Continue,
    )?;
    ensure_equal(
        step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?,
        EngineSignal::Continue,
    )?;

    match *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())? {
        SlotValue::Object(handle) => {
            let fields = store.object(handle).map_err(|e| e.to_string())?;
            ensure_equal(fields.len(), 1usize)?;
            let field = fields
                .first()
                .copied()
                .ok_or_else(|| String::from("missing object field"))?;
            ensure_equal(
                field,
                ObjectField::clean(SymbolId::new(1), SlotValue::I64(100)),
            )
        }
        ref other => Err(format!("expected Object, got {other:?}")),
    }
}

#[test]
fn step_once_build_list_writes_list_to_output_slot() -> Result<(), String> {
    let workflow = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("build_list_step"),
        digest: WorkflowDigest::from_bytes([0x88; 32]),
        nodes: vec![
            set_const_node(StepIdx::new(0), SlotIdx::new(0), StepIdx::new(1)),
            CompiledNode {
                id: StepIdx::new(1),
                output: Some(SlotIdx::new(1)),
                next: Some(StepIdx::new(2)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::BuildList {
                    items: vec![SlotIdx::new(0)].into_boxed_slice(),
                },
            },
            finish_node(StepIdx::new(2), SlotIdx::new(1)),
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Bool(true)].into_boxed_slice(),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
        input_slots: Box::new([]),    })
    .map_err(|e| e.to_string())?;
    let mut run = test_frame(&workflow)?;
    let mut store = ValueStore::new();

    ensure_equal(
        step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?,
        EngineSignal::Continue,
    )?;
    ensure_equal(
        step_once(&workflow, &mut run, &mut store).map_err(|e| e.to_string())?,
        EngineSignal::Continue,
    )?;

    match *run.read_slot(SlotIdx::new(1)).map_err(|e| e.to_string())? {
        SlotValue::List(handle) => {
            let items = store.list(handle).map_err(|e| e.to_string())?;
            ensure_equal(items.len(), 1usize)?;
            let item = items
                .first()
                .copied()
                .ok_or_else(|| String::from("missing list item"))?;
            let tainted_item = store
                .list_item_with_taint(handle, 0)
                .map_err(|e| e.to_string())?;
            ensure_equal(item, SlotValue::Bool(true))?;
            ensure_equal(tainted_item, (SlotValue::Bool(true), Taint::Clean))
        }
        ref other => Err(format!("expected List, got {other:?}")),
    }
}

fn set_const_node(id: StepIdx, output: SlotIdx, next: StepIdx) -> CompiledNode {
    CompiledNode {
        id,
        output: Some(output),
        next: Some(next),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    }
}

fn finish_node(id: StepIdx, result: SlotIdx) -> CompiledNode {
    CompiledNode {
        id,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish { result },
    }
}
