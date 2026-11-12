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
fn numeric_self_bypass() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"1\" }\n    if: \"true\"\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "do.input",
        },
    )
}

#[test]
fn numeric_forward_bypass() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: consumer\n    do: { action: \"2\", input: \"2\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(2),
            consumer: "do.input",
        },
    )
}

#[test]
fn required_finish_mixed_case() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: last\n    do: { action: \"2\", input: \"$seed\" }",
    );
    expect_err(&y, CompileError::LastStepMustFinish)
}

#[test]
fn nested_scoped_lexical_read() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: bounded_repeat\n    repeat:\n      max_attempts: 2\n      steps:\n        - id: nested\n          do: { action: \"2\", input: \"$g\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "do.input",
        },
    )
}
