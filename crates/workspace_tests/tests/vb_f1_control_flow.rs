#![forbid(unsafe_code)]

use vb_compile::{CompileError, CompileErrors, compile_workflow};
use vb_core::{
    CapabilitySet, CompiledNodeKind, ConstValue, RunId, SlotValue, StepBudget, ValueStore,
    new_run_frame,
};
use vb_runtime::engine::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

fn run(
    steps: &str,
    expected: i64,
    expected_work: &[(&str, i64, usize)],
) -> Result<(), Box<dyn std::error::Error>> {
    let source = format!(
        "version: velvet-ballistics/v1\nname: controls\nwhen: {{manual: {{}}}}\ninputs:\n  divisor: {{is: number}}\nsteps:\n{steps}"
    );
    let workflow = compile_workflow(source.as_bytes())?;
    let original = workflow.to_parts();
    let input = workflow.input_slots().first().ok_or("missing input")?.slot;
    let mut frame = new_run_frame(RunId::new(1), &workflow)?;
    frame.write_slot(input, SlotValue::I64(0))?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    for _ in 0..64 {
        let signal = drive_deterministic_full(
            &workflow,
            &mut frame,
            &mut StepBudget::new(1),
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect,
            &CapabilitySet::empty(),
            1,
        )?;
        if let RuntimeSignal::Finished(value) = signal {
            assert_eq!(value, SlotValue::I64(expected));
            assert_eq!(workflow.to_parts(), original);
            let events = evidence.drain();
            for (name, expected_value, expected_count) in expected_work {
                let writers = original.nodes.iter().filter(|node| {
                    workflow.step_name(node.id) == Some(*name)
                        && matches!(node.kind, CompiledNodeKind::SetConst { value } if workflow.constant(value) == Some(&ConstValue::I64(*expected_value)))
                }).collect::<Vec<_>>();
                assert_eq!(writers.len(), 1, "writer for {name}");
                let writer = writers.first().ok_or("missing authored writer")?;
                let count = events.iter().filter(|event| matches!(event, EvidenceEvent::StepSucceeded { step, .. } if *step == writer.id)).count();
                assert_eq!(count, *expected_count, "executions of {name}");
                let slot = writer.output.slot().ok_or("writer output missing")?;
                let expected_slot =
                    (*expected_count > 0).then_some(SlotValue::I64(*expected_value));
                assert_eq!(
                    frame.slots_snapshot().get(slot.as_usize()),
                    Some(&expected_slot),
                    "value of {name}"
                );
            }
            return Ok(());
        }
        assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    }
    Err("control workflow did not finish".into())
}

#[test]
fn guards_apply_to_scalars_and_nested_composites() -> Result<(), Box<dyn std::error::Error>> {
    for (condition, scalar_count, nested_count) in [("true", 1, 2), ("false", 0, 0)] {
        run(
            &format!(
                "  - id: answer\n    set: {{output: answer, value: '42'}}\n  - id: bypassed\n    name: guarded scalar\n    if: '{condition}'\n    set: {{output: other, value: '7'}}\n  - id: nested\n    if: '{condition}'\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: local\n          if: '{condition}'\n          set: {{output: local, value: '9'}}\n        - id: tail\n          set: {{output: tail, value: '9'}}\n  - id: done\n    finish: {{result: answer}}\n"
            ),
            42,
            &[
                ("guarded scalar", 7, scalar_count),
                ("local", 9, nested_count),
                ("tail", 9, nested_count),
            ],
        )?;
    }
    Ok(())
}

#[test]
fn then_jumps_forward_in_the_same_lexical_body() -> Result<(), Box<dyn std::error::Error>> {
    run(
        "  - id: answer\n    set: {output: answer, value: '42'}\n  - id: redirect\n    if: 'true'\n    then: done\n    set: {output: redirect, value: '1'}\n  - id: bypassed\n    set: {output: bypassed, value: '1 / $input.divisor'}\n  - id: done\n    finish: {result: answer}\n",
        42,
        &[("redirect", 1, 1)],
    )?;
    run(
        "  - id: answer\n    set: {output: answer, value: '42'}\n  - id: nested\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: first\n          if: 'true'\n          then: last\n          set: {output: first, value: '3'}\n        - id: bypassed\n          set: {output: bypassed, value: '1 / $input.divisor'}\n        - id: last\n          set: {output: last, value: '9'}\n  - id: done\n    finish: {result: answer}\n",
        42,
        &[("first", 3, 2), ("last", 9, 2)],
    )
}

#[test]
fn on_error_routes_expression_failures_to_the_handler() -> Result<(), Box<dyn std::error::Error>> {
    run(
        "  - id: answer\n    set: {output: answer, value: '42'}\n  - id: divide\n    on_error: {handler: done}\n    set: {output: quotient, value: '1 / $input.divisor'}\n  - id: bypassed\n    set: {output: bypassed, value: '7'}\n  - id: done\n    finish: {result: answer}\n",
        42,
        &[],
    )
}

#[test]
fn invalid_control_targets_and_conditions_fail_compilation() {
    for (field, error) in [
        (
            "then: absent",
            CompileError::UnknownStepLabel {
                step: 0,
                label: "absent".into(),
            },
        ),
        (
            "then: first",
            CompileError::StepFieldShape {
                step: 0,
                field: "control target",
                expected: "a later step in the same lexical body",
            },
        ),
        (
            "if: '3'",
            CompileError::TypeMismatch {
                field: "if",
                expected: "boolean",
                found: "non-boolean",
            },
        ),
        (
            "on_error: {handler: absent}",
            CompileError::UnknownStepLabel {
                step: 0,
                label: "absent".into(),
            },
        ),
    ] {
        let source = format!(
            "version: velvet-ballistics/v1\nname: controls\nwhen: {{manual: {{}}}}\nsteps:\n  - id: first\n    {field}\n    set: {{output: answer, value: '42'}}\n  - id: done\n    finish: {{result: answer}}\n"
        );
        assert_eq!(
            compile_workflow(source.as_bytes()).map(|_| ()),
            Err(CompileErrors(vec![error])),
            "{field}"
        );
    }
}
