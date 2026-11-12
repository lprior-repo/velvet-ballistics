use super::*;
use vb_core::{ListId, RunId, SlotIdx, SlotValue, StepIdx, SymbolId, Taint};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn workflow() -> TestResult<CompiledWorkflow> {
    crate::run_checkpoint::symbol_prefix_fixture::prefix_workflow(vec![Box::from("key")])
}

fn frame(value: SlotValue) -> TestResult<RunFrame> {
    let mut frame = RunFrame::new(RunId::new(91), StepIdx::ZERO, 2, 2)?;
    frame.write_slot_with_taint(SlotIdx::ZERO, value, Taint::Secret)?;
    Ok(frame)
}

fn store(workflow: &CompiledWorkflow) -> TestResult<ValueStore> {
    let mut store = ValueStore::with_max_slots(workflow.resource_contract().max_slots);
    assert_eq!(store.insert_text("key")?, SymbolId::new(0));
    Ok(store)
}

fn populated(workflow: &CompiledWorkflow) -> TestResult<ValueStore> {
    let mut store = store(workflow)?;
    assert_eq!(store.insert_text("retired")?, SymbolId::new(1));
    assert_eq!(
        store.insert_list(Box::from([SlotValue::I64(7)]))?,
        ListId::new(0)
    );
    Ok(store)
}

fn prefix_error() -> RuntimeError {
    RuntimeError::Recovery {
        error: format!(
            "restored value store failed validation: {}",
            crate::run_checkpoint::RunCheckpointV1Error::SymbolPrefixMismatch { index: 0 }
        ),
    }
}

fn rejects(actual: RuntimeResult<Option<ValueStore>>, expected: RuntimeError) -> TestResult {
    match actual {
        Err(error) => assert_eq!(error, expected),
        Ok(_) => return Err("invalid recovered store was accepted".into()),
    }
    Ok(())
}

#[test]
fn restored_full_arena_preserves_checkpoint_and_frame() -> TestResult {
    let workflow = workflow()?;
    let frame = frame(SlotValue::List(ListId::new(0)))?;
    let before_frame = frame.clone();
    let checkpoint = populated(&workflow)?.checkpoint_v1();
    let restored = restore(Some(checkpoint.clone()), &frame, &workflow, false)?;
    assert_eq!(
        restored.ok_or("missing restored store")?.checkpoint_v1(),
        checkpoint
    );
    assert_eq!(frame, before_frame);
    Ok(())
}

#[test]
fn wrong_compiler_prefix_rejects_after_structural_validation() -> TestResult {
    let workflow = workflow()?;
    let frame = frame(SlotValue::Null)?;
    let before_frame = frame.clone();
    let mut checkpoint = store(&workflow)?.checkpoint_v1();
    *checkpoint.symbols.get_mut(0).ok_or("missing symbol")? = Box::from("wrong");
    rejects(
        restore(Some(checkpoint), &frame, &workflow, false),
        prefix_error(),
    )?;
    assert_eq!(frame, before_frame);
    Ok(())
}

#[test]
fn missing_compiler_prefix_rejects_after_structural_validation() -> TestResult {
    let workflow = workflow()?;
    let frame = frame(SlotValue::Null)?;
    let before_frame = frame.clone();
    let mut checkpoint = store(&workflow)?.checkpoint_v1();
    checkpoint.symbols.clear();
    rejects(
        restore(Some(checkpoint), &frame, &workflow, false),
        prefix_error(),
    )?;
    assert_eq!(frame, before_frame);
    Ok(())
}

#[test]
fn missing_list_arena_rejects_without_mutating_frame() -> TestResult {
    let workflow = workflow()?;
    let frame = frame(SlotValue::List(ListId::new(0)))?;
    let before_frame = frame.clone();
    let checkpoint = store(&workflow)?.checkpoint_v1();
    rejects(
        restore(Some(checkpoint), &frame, &workflow, false),
        RuntimeError::InvalidRecoveryHydration,
    )?;
    assert_eq!(frame, before_frame);
    Ok(())
}

#[test]
fn malformed_list_taints_reject_without_mutating_frame() -> TestResult {
    let workflow = workflow()?;
    let frame = frame(SlotValue::List(ListId::new(0)))?;
    let before_frame = frame.clone();
    let mut checkpoint = populated(&workflow)?.checkpoint_v1();
    checkpoint.list_taints.clear();
    rejects(
        restore(Some(checkpoint), &frame, &workflow, false),
        RuntimeError::InvalidRecoveryHydration,
    )?;
    assert_eq!(frame, before_frame);
    Ok(())
}

#[test]
fn retry_rejects_restored_store_without_mutating_frame() -> TestResult {
    let workflow = workflow()?;
    let frame = frame(SlotValue::List(ListId::new(0)))?;
    let before_frame = frame.clone();
    let checkpoint = populated(&workflow)?.checkpoint_v1();
    rejects(
        restore(Some(checkpoint), &frame, &workflow, true),
        RuntimeError::InvalidRecoveryHydration,
    )?;
    assert_eq!(frame, before_frame);
    Ok(())
}
