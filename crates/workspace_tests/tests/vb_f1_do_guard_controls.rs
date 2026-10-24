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
fn guard_non_do_set_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n    if: \"true\"\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::UnsupportedStepControlField {
            step: 0,
            field: "if".into(),
        },
    )
}

#[test]
fn guard_non_do_finish_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: end\n    finish: { result: \"$seed\" }\n    if: \"true\"",
    );
    expect_err(
        &y,
        CompileError::UnsupportedStepControlField {
            step: 1,
            field: "if".into(),
        },
    )
}

#[test]
fn guard_do_with_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n    with: {}\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::UnsupportedStepControlField {
            step: 1,
            field: "with".into(),
        },
    )
}

#[test]
fn guard_do_try_again_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n    try_again: { max_attempts: 2 }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::UnsupportedStepControlField {
            step: 1,
            field: "try_again".into(),
        },
    )
}

#[test]
fn guard_do_on_error_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n    on_error: { handler: end }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::UnsupportedStepControlField {
            step: 1,
            field: "on_error".into(),
        },
    )
}

#[test]
fn guard_do_then_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n    then: []\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::UnsupportedStepControlField {
            step: 1,
            field: "then".into(),
        },
    )
}

#[test]
fn guard_scoped_repeat_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: inner\n          do: { action: \"1\", input: \"$seed\" }\n          if: \"true\"\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::UnsupportedStepControlField {
            step: 0,
            field: "if".into(),
        },
    )
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
