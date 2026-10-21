use std::error::Error;
use vb_core::ids::{StepIdx, WorkflowDigest};
use vb_core::workflow::{
    CompiledInputSlot, CompiledNode, CompiledNodeKind, CompiledWorkflow, InputSlotKind,
    ResourceContract, WorkflowParts,
};
use vb_core::{SlotIdx, SlotValue};

type TestResult<T> = Result<T, Box<dyn Error>>;

fn fixture(input_slots: Box<[CompiledInputSlot]>) -> TestResult<CompiledWorkflow> {
    let node = CompiledNode {
        id: StepIdx::ZERO,
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(2),
        },
    };
    let parts = WorkflowParts {
        semantic_source_identity: None,
        name: Box::from("input-mapping"),
        digest: WorkflowDigest::from_bytes([1; 32]),
        nodes: Box::from([node]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 4,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
        input_slots,
    };
    CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string().into())
}

use vb_core::RunId;
use vb_core::frame::RunFrame;
use vb_runtime::RuntimeError;
use vb_runtime::shard::helpers::seed_input_slots;

#[test]
fn test_any_slot_accepts_all_values() -> Result<(), Box<dyn std::error::Error>> {
    let wf = fixture(Box::from([CompiledInputSlot {
        slot: SlotIdx::new(2),
        kind: InputSlotKind::Any,
    }]))?;
    let values = [
        vb_core::SlotValue::Null,
        vb_core::SlotValue::Bool(false),
        vb_core::SlotValue::Bool(true),
        vb_core::SlotValue::I64(i64::MIN),
        vb_core::SlotValue::I64(i64::MAX),
        vb_core::SlotValue::F64(vb_core::value::FiniteF64::new(1.25)?),
        vb_core::SlotValue::Symbol(vb_core::SymbolId::new(0)),
        vb_core::SlotValue::List(vb_core::ListId::new(0)),
        vb_core::SlotValue::Object(vb_core::ObjectId::new(0)),
        vb_core::SlotValue::Blob(vb_core::BlobId::new(0)),
    ];
    for value in values {
        let mut frame = RunFrame::new(RunId::new(1), wf.entry(), wf.node_count(), wf.slot_count())?;
        seed_input_slots(&mut frame, &[(SlotIdx::new(2), value)], &wf)?;
        let read = frame.read_slot(SlotIdx::new(2))?;
        if read != &value {
            return Err(Box::from("read mismatch"));
        }
    }
    Ok(())
}

#[test]
fn test_i64_slot_rejects_bool() -> Result<(), Box<dyn std::error::Error>> {
    let wf = fixture(Box::from([CompiledInputSlot {
        slot: SlotIdx::new(2),
        kind: InputSlotKind::I64,
    }]))?;
    let mut frame = RunFrame::new(RunId::new(1), wf.entry(), wf.node_count(), wf.slot_count())?;
    let before = frame.read_slot(SlotIdx::new(2)).copied();
    let result = seed_input_slots(&mut frame, &[(SlotIdx::new(2), SlotValue::Bool(true))], &wf);
    let after = frame.read_slot(SlotIdx::new(2)).copied();
    if before != after {
        return Err(Box::from("slot changed"));
    }
    match result {
        Err(RuntimeError::RuntimeInputSlotKindMismatch {
            slot,
            expected,
            actual,
        }) => {
            if slot != SlotIdx::new(2)
                || expected != InputSlotKind::I64
                || actual != InputSlotKind::Bool
            {
                return Err(Box::from("wrong mismatch"));
            }
        }
        other => return Err(Box::from(format!("unexpected: {:?}", other))),
    }
    Ok(())
}
