#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::{CapabilitySet, RunId, SlotValue, StepBudget, ValueStore, new_run_frame};
use vb_runtime::engine::{EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full};
use vb_runtime::primitives::collect::CollectStates;

fn run(steps: &str, expected: i64) -> Result<(), Box<dyn std::error::Error>> {
    let source = format!("version: velvet-ballistics/v1\nname: controls\nwhen: {{manual: {{}}}}\ninputs:\n  divisor: {{is: number}}\nsteps:\n{steps}");
    let workflow = compile_workflow(source.as_bytes())?;
    let original = workflow.to_parts();
    let input = workflow.input_slots().first().ok_or("missing input")?.slot;
    let mut frame = new_run_frame(RunId::new(1), &workflow)?;
    frame.write_slot(input, SlotValue::I64(0))?;
    let mut store = ValueStore::new();
    let mut evidence = EvidenceCollector::new();
    let mut collect = CollectStates::new();
    for _ in 0..64 {
        let signal = drive_deterministic_full(&workflow, &mut frame, &mut StepBudget::new(1),
            &mut store, &[], RetryPolicy::NEVER, &mut evidence, &mut collect,
            &CapabilitySet::empty(), 1)?;
        if let RuntimeSignal::Finished(value) = signal {
            assert_eq!(value, SlotValue::I64(expected));
            assert_eq!(workflow.to_parts(), original);
            return Ok(());
        }
        assert_eq!(signal, RuntimeSignal::StepBudgetExhausted);
    }
    Err("control workflow did not finish".into())
}

#[test]
fn guards_apply_to_scalars_and_nested_composites() -> Result<(), Box<dyn std::error::Error>> {
    for condition in ["true", "false"] {
        run(&format!("  - id: answer\n    set: {{output: answer, value: '42'}}\n  - id: skipped\n    name: guarded scalar\n    if: '{condition}'\n    set: {{output: other, value: '7'}}\n  - id: nested\n    if: '{condition}'\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: local\n          if: '{condition}'\n          set: {{output: local, value: '9'}}\n  - id: done\n    finish: {{result: answer}}\n"), 42)?;
    }
    Ok(())
}

#[test]
fn then_jumps_forward_in_the_same_lexical_body() -> Result<(), Box<dyn std::error::Error>> {
    run("  - id: answer\n    if: 'true'\n    then: done\n    set: {output: answer, value: '42'}\n  - id: skipped\n    set: {output: skipped, value: '7'}\n  - id: done\n    finish: {result: answer}\n", 42)?;
    run("  - id: answer\n    set: {output: answer, value: '42'}\n  - id: nested\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: first\n          if: 'true'\n          then: last\n          set: {output: first, value: '3'}\n        - id: skipped\n          set: {output: skipped, value: '7'}\n        - id: last\n          set: {output: last, value: '9'}\n  - id: done\n    finish: {result: answer}\n", 42)
}

#[test]
fn on_error_routes_expression_failures_to_the_handler() -> Result<(), Box<dyn std::error::Error>> {
    run("  - id: answer\n    set: {output: answer, value: '42'}\n  - id: divide\n    on_error: {handler: done}\n    set: {output: quotient, value: '1 / $input.divisor'}\n  - id: skipped\n    set: {output: skipped, value: '7'}\n  - id: done\n    finish: {result: answer}\n", 42)
}

#[test]
fn invalid_control_targets_and_conditions_fail_compilation() {
    for field in ["then: absent", "then: first", "if: '3'", "on_error: {handler: absent}"] {
        let source = format!("version: velvet-ballistics/v1\nname: controls\nwhen: {{manual: {{}}}}\nsteps:\n  - id: first\n    {field}\n    set: {{output: answer, value: '42'}}\n  - id: done\n    finish: {{result: answer}}\n");
        assert!(compile_workflow(source.as_bytes()).is_err(), "{field}");
    }
}
