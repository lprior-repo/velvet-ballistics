#![forbid(unsafe_code)]

use vb_compile::{CompileError, compile_workflow};
use vb_core::SlotIdx;

fn yaml(steps: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\nname: guards\nwhen: {{ manual: {{}} }}\nsteps:\n{steps}"
    )
}

fn expect_err(yaml: &str, expected: CompileError) -> Result<(), String> {
    match compile_workflow(yaml.as_bytes()) {
        Ok(_) => Err(format!("expected {:?}, got Ok", expected)),
        Err(errors) => {
            for err in errors.0.iter() {
                if *err == expected {
                    return Ok(());
                }
            }
            Err(format!("expected {:?}, got {:?}", expected, errors.0))
        }
    }
}

#[test]
fn guard_non_do_set_executes() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: guarded\n    set: { output: other, value: \"7\" }\n    if: \"true\"\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    execute(&y)
}

#[test]
fn guard_non_do_finish_executes() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: end\n    finish: { result: \"$seed\" }\n    if: \"true\"",
    );
    execute(&y)
}

#[test]
fn guard_do_with_and_input_are_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"false\"\n    with: {}\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::CanonicalYaml {
            category: "field_shape",
            message: "field shape error: do.input expected exactly one of input or with".into(),
        },
    )
}

#[test]
fn guard_do_try_again_executes() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"false\"\n    try_again: { max_attempts: 2 }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    execute(&y)
}

#[test]
fn guard_do_on_error_executes() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"false\"\n    on_error: { handler: end }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    execute(&y)
}

#[test]
fn guard_do_then_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n    then: []\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::CanonicalYaml {
            category: "field_shape",
            message: "field shape error: then expected a non-empty string".into(),
        },
    )
}

#[test]
fn guard_scoped_repeat_executes() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: inner\n          set: { output: inner_result, value: \"1\" }\n          if: \"true\"\n        - id: tail\n          set: { output: repeated_result, value: \"2\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    execute(&y)
}

#[test]
fn guard_consumer_ask_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: a\n    ask: { prompt: \"1\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "ask.prompt",
        },
    )
}

#[test]
fn guard_consumer_wait_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: w\n    wait: { event: \"1\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "waitevent.event",
        },
    )
}

#[test]
fn guard_consumer_choose_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: c\n    choose:\n      branches:\n        - when: \"1\"\n          steps:\n            - id: s\n              set: { output: local, value: \"1\" }\n      otherwise: end\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "chooseslot.condition",
        },
    )
}

fn execute(source: &str) -> Result<(), String> {
    use vb_core::{CapabilitySet, RunId, SlotValue, StepBudget, ValueStore, new_run_frame};
    use vb_runtime::engine::{
        EvidenceCollector, RetryPolicy, RuntimeSignal, drive_deterministic_full,
    };
    use vb_runtime::primitives::collect::CollectStates;
    let workflow = compile_workflow(source.as_bytes()).map_err(|error| format!("{error:?}"))?;
    let original = workflow.to_parts();
    let mut frame = new_run_frame(RunId::new(1), &workflow).map_err(|error| error.to_string())?;
    let signal = drive_deterministic_full(
        &workflow,
        &mut frame,
        &mut StepBudget::new(128),
        &mut ValueStore::new(),
        &[],
        RetryPolicy::NEVER,
        &mut EvidenceCollector::new(),
        &mut CollectStates::new(),
        &CapabilitySet::empty(),
        1,
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(signal, RuntimeSignal::Finished(SlotValue::I64(42)));
    assert_eq!(workflow.to_parts(), original);
    Ok(())
}
