use vb_compile::{CompileError, CompileErrors, compile_source, parse_workflow_source};
use vb_core::engine::{EngineSignal, StepBudget, new_run_frame, run_until_blocked};
use vb_core::value_store::ValueStore;
use vb_core::{
    CompiledInputSlot, CompiledNodeKind, CompiledWorkflow, InputSlotKind, RunId, SlotIdx,
    SlotValue, Taint, WorkflowParts,
};

fn compile(yaml: &str) -> Result<CompiledWorkflow, Box<dyn std::error::Error>> {
    Ok(compile_source(&parse_workflow_source(yaml)?)?)
}

fn run(
    workflow: &CompiledWorkflow,
    values: &[i64],
) -> Result<EngineSignal, Box<dyn std::error::Error>> {
    let mut frame = new_run_frame(RunId::new(1), workflow)?;
    for (port, value) in workflow.input_slots().iter().zip(values.iter()) {
        frame.write_slot_with_taint(port.slot, SlotValue::I64(*value), Taint::Secret)?;
    }
    Ok(run_until_blocked(
        workflow,
        &mut frame,
        StepBudget::MAX,
        &mut ValueStore::new(),
    )?)
}

const BASE: &str = "version: velvet-ballistics/v1\nname: input_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: done\n    finish:\n      result: $input.count\n";

#[test]
fn test_compile_base() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile(BASE)?;
    assert_eq!(
        workflow.input_slots(),
        &[CompiledInputSlot {
            slot: SlotIdx::new(0),
            kind: InputSlotKind::I64
        }]
    );
    for value in [i64::MIN, -1, 0, 42, i64::MAX] {
        let signal = run(&workflow, &[value])?;
        assert_eq!(
            signal,
            EngineSignal::Finished(SlotValue::I64(value), Taint::Secret)
        );
    }
    Ok(())
}

#[test]
fn test_save_before_finish() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: input_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: write_internal\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: $input.count\n";
    let workflow = compile(yaml)?;
    assert_eq!(
        workflow.input_slots(),
        &[CompiledInputSlot {
            slot: SlotIdx::new(1),
            kind: InputSlotKind::I64
        }]
    );
    let signal = run(&workflow, &[99])?;
    assert_eq!(
        signal,
        EngineSignal::Finished(SlotValue::I64(99), Taint::Secret)
    );
    Ok(())
}

#[test]
fn test_two_inputs() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: input_finish\nwhen:\n  manual: {}\ninputs:\n  first: number\n  second: number\nsteps:\n  - id: done\n    finish:\n      result: $input.second\n";
    let workflow = compile(yaml)?;
    assert_eq!(
        workflow.input_slots(),
        &[
            CompiledInputSlot {
                slot: SlotIdx::new(0),
                kind: InputSlotKind::I64
            },
            CompiledInputSlot {
                slot: SlotIdx::new(1),
                kind: InputSlotKind::I64
            }
        ]
    );
    let signal = run(&workflow, &[11, 99])?;
    assert_eq!(
        signal,
        EngineSignal::Finished(SlotValue::I64(99), Taint::Secret)
    );
    Ok(())
}

#[test]
fn test_unknown_ref() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = BASE.replace("$input.count", "$input.missing");
    let parsed = parse_workflow_source(&yaml)?;
    match compile_source(&parsed) {
        Err(errors) => {
            assert_eq!(
                errors,
                CompileErrors(vec![CompileError::UnknownReferenceName {
                    kind: "declaration",
                    reference: Box::from("$input.missing"),
                    name: Box::from("$input.missing")
                }])
            );
        }
        Ok(_) => return Err("expected UnknownReferenceName".into()),
    }
    Ok(())
}

#[test]
fn test_named_internal_finish() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: input_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: write_internal\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: saved\n";
    let workflow = compile(yaml)?;
    assert_eq!(
        workflow.input_slots(),
        &[CompiledInputSlot {
            slot: SlotIdx::new(1),
            kind: InputSlotKind::I64
        }]
    );
    let signal = run(&workflow, &[999])?;
    assert_eq!(
        signal,
        EngineSignal::Finished(SlotValue::I64(42), Taint::Clean)
    );
    Ok(())
}

#[test]
fn test_numeric_slot_finish() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = "version: velvet-ballistics/v1\nname: input_finish\nwhen:\n  manual: {}\ninputs:\n  count: number\nsteps:\n  - id: write_internal\n    save:\n      output: saved\n      value: '42'\n  - id: done\n    finish:\n      result: 3\n";
    let workflow = compile(yaml)?;
    assert_eq!(
        workflow.input_slots(),
        &[CompiledInputSlot {
            slot: SlotIdx::new(4),
            kind: InputSlotKind::I64
        }]
    );
    let parts = workflow.to_parts();
    let terminal = parts.nodes.last().ok_or("missing terminal")?;
    assert_eq!(
        terminal.kind,
        CompiledNodeKind::Finish {
            result: SlotIdx::new(3)
        }
    );
    Ok(())
}

#[test]
fn test_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = compile(BASE)?;
    let parts = workflow.to_parts();
    let serialized = postcard::to_allocvec(&parts)?;
    let restored_parts: WorkflowParts = postcard::from_bytes(&serialized)?;
    assert_eq!(parts, restored_parts);
    let recompiled = compile(BASE)?;
    assert_eq!(parts, recompiled.to_parts());
    let restored_workflow = CompiledWorkflow::try_from_parts(restored_parts)?;
    let signal = run(&restored_workflow, &[42])?;
    assert_eq!(
        signal,
        EngineSignal::Finished(SlotValue::I64(42), Taint::Secret)
    );
    Ok(())
}

#[path = "finish_input_binding/extra.rs"]
mod extra;
