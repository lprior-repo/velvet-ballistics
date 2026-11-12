use std::{num::NonZeroUsize, sync::Arc};
use vb_compile::compile_workflow;
use vb_core::{InputSlotKind, RunId, RuntimePolicy, SlotValue, Taint};
use vb_runtime::{
    RuntimeError, journal::VolatileRuntimeJournal, runtime::Runtime, shard::ShardConfig,
};

fn runtime() -> (Runtime, Arc<VolatileRuntimeJournal>) {
    let journal = Arc::new(VolatileRuntimeJournal::new());
    let config = ShardConfig {
        policy: RuntimePolicy::Relaxed,
        step_budget_per_tick: 1,
        ..ShardConfig::default()
    };
    (
        Runtime::new(NonZeroUsize::MIN, config, journal.clone()),
        journal,
    )
}

fn source(schema: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: input_admission\nwhen: {{manual: {{}}}}\ninputs:\n  value: {schema}\nsteps:\n  - id: first\n    set: {{output: answer, value: '42'}}\n  - id: done\n    finish: {{result: answer}}\n"
    )
}

#[test]
fn invalid_inputs_are_rejected_before_queue_or_journal_effects()
-> Result<(), Box<dyn std::error::Error>> {
    for (schema, supplied) in [
        ("{is: number, min: 3, max: 5}", Some(SlotValue::I64(2))),
        ("{is: number, min: 3, max: 5}", Some(SlotValue::I64(6))),
        ("{is: number}", Some(SlotValue::Bool(true))),
        ("{is: number}", None),
        ("{is: number}", Some(SlotValue::Null)),
    ] {
        let workflow = compile_workflow(source(schema).as_bytes())?;
        let slot = workflow.input_slots().first().ok_or("missing input")?.slot;
        let (mut runtime, journal) = runtime();
        let inputs = supplied
            .map(|value| vec![(slot, value)])
            .unwrap_or_default();
        let expected = match supplied {
            Some(SlotValue::Bool(_)) => RuntimeError::RuntimeInputSlotKindMismatch {
                slot,
                expected: InputSlotKind::I64,
                actual: InputSlotKind::Bool,
            },
            Some(SlotValue::Null) => RuntimeError::RuntimeInputSlotKindMismatch {
                slot,
                expected: InputSlotKind::I64,
                actual: InputSlotKind::Null,
            },
            _ => RuntimeError::UnsupportedOperation {
                operation: "input_schema_mismatch",
            },
        };
        assert_eq!(
            runtime.submit_compiled_with_inputs(RunId::new(1), workflow, inputs.into_boxed_slice()),
            Err(expected)
        );
        assert_eq!(journal.snapshot()?, Vec::new());
        runtime.tick_all()?;
        assert_eq!(
            runtime.inspect_run_state(RunId::new(1)).map(|_| ()),
            Err(RuntimeError::RunNotFound)
        );
        assert_eq!(journal.snapshot()?, Vec::new());
    }
    Ok(())
}

#[test]
fn nullable_and_secret_defaults_reach_the_actual_runtime_frame()
-> Result<(), Box<dyn std::error::Error>> {
    for (schema, supplied, expected, taint) in [
        (
            "{is: number, nullable: true}",
            Some(SlotValue::Null),
            SlotValue::Null,
            Taint::Clean,
        ),
        (
            "{is: number, default: 4, secret: true}",
            None,
            SlotValue::I64(4),
            Taint::Secret,
        ),
        (
            "{is: number, min: 3, max: 5}",
            Some(SlotValue::I64(3)),
            SlotValue::I64(3),
            Taint::Clean,
        ),
        (
            "{is: number, min: 3, max: 5}",
            Some(SlotValue::I64(5)),
            SlotValue::I64(5),
            Taint::Clean,
        ),
    ] {
        let workflow = compile_workflow(source(schema).as_bytes())?;
        let slot = workflow.input_slots().first().ok_or("missing input")?.slot;
        let (mut runtime, _) = runtime();
        let inputs = supplied
            .map(|value| vec![(slot, value)])
            .unwrap_or_default();
        runtime.submit_compiled_with_inputs(RunId::new(1), workflow, inputs.into_boxed_slice())?;
        runtime.tick_all()?;
        let state = runtime.inspect_run_state(RunId::new(1))?;
        assert_eq!(*state.frame.read_slot(slot)?, expected);
        assert_eq!(state.frame.read_taint(slot)?, taint);
    }
    Ok(())
}
