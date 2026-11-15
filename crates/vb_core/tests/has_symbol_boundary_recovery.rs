use vb_core::engine::eval_expr_with_store;
use vb_core::value_store::{ObjectField, ValueStore};
use vb_core::{
    CompiledNodeKind, CompiledWorkflow, EngineError, ExprIdx, ExprOp, ExprProgram,
    ResourceContract, RunFrame, RunId, SlotIdx, SlotValue, StepIdx, SymbolId, Taint,
    WorkflowDigest, WorkflowParts,
};

fn evaluate_has(
    object: vb_core::ObjectId,
    key: SymbolId,
    store: &mut ValueStore,
) -> Result<(SlotValue, Taint), Box<dyn std::error::Error>> {
    let expression = ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(SlotIdx::new(0)),
        ExprOp::LoadSlot(SlotIdx::new(1)),
        ExprOp::Has,
    ]))?;
    let plan = CompiledWorkflow::try_from_parts(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: "has_symbol_boundary".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        nodes: Box::new([vb_core::workflow::node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            None,
        )]),
        expressions: Box::new([expression]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })?;
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 1, 2)?;
    run.write_slot(SlotIdx::new(0), SlotValue::Object(object))?;
    run.write_slot(SlotIdx::new(1), SlotValue::Symbol(key))?;
    Ok(eval_expr_with_store(&plan, &run, store, ExprIdx::new(0))?)
}

#[test]
fn numeric_has_rejects_dangling_symbol() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = ValueStore::new();
    let object = store.insert_object([])?;
    let key = SymbolId::new(u32::MAX);
    let result = evaluate_has(object, key, &mut store);
    let error = result.err().ok_or("expected dangling symbol failure")?;
    assert_eq!(
        error.downcast_ref::<EngineError>(),
        Some(&EngineError::SymbolOutOfBounds { symbol: key })
    );
    Ok(())
}

#[test]
fn numeric_has_distinguishes_present_and_valid_missing_keys()
-> Result<(), Box<dyn std::error::Error>> {
    let mut store = ValueStore::new();
    let present = store.insert_symbol("present")?;
    let missing = store.insert_symbol("missing")?;
    let object = store.insert_object([ObjectField::clean(present, SlotValue::I64(7))])?;
    assert_eq!(
        evaluate_has(object, present, &mut store)?,
        (SlotValue::Bool(true), Taint::Clean)
    );
    assert_eq!(
        evaluate_has(object, missing, &mut store)?,
        (SlotValue::Bool(false), Taint::Clean)
    );
    Ok(())
}
