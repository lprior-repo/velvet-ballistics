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

fn expect_unknown(yaml: &str, reference: &str) -> Result<(), String> {
    let name = reference
        .strip_prefix('$')
        .ok_or_else(|| format!("reference must start with $: {}", reference))?;
    match compile_workflow(yaml.as_bytes()) {
        Err(errors) => {
            for err in errors.0.iter() {
                if let CompileError::UnknownReferenceName {
                    reference: r,
                    name: n,
                    ..
                } = err
                {
                    if r.as_ref() == reference && n.as_ref() == name {
                        return Ok(());
                    }
                }
            }
            Err(format!(
                "expected UnknownReferenceName for {} (name={}), got {:?}",
                reference, name, errors.0
            ))
        }
        _ => Err(format!("expected error for {}", reference)),
    }
}

#[test]
fn guard_nonbool_types() -> Result<(), String> {
    let cases = [("0", "integer"), ("42", "integer"), ("null", "null")];
    for (value, found) in cases.iter() {
        let y = yaml(&format!(
            "  - id: seed\n    set: {{ output: x, value: \"42\" }}\n  - id: g\n    do: {{ action: \"1\", input: \"$seed\" }}\n    if: {:?}\n  - id: end\n    finish: {{ result: \"$seed\" }}",
            value
        ));
        expect_err(
            &y,
            CompileError::TypeMismatch {
                field: "if",
                expected: "boolean",
                found: *found,
            },
        )?;
    }
    Ok(())
}

#[test]
fn guard_unknown_prior_do() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: prior\n    do: { action: \"0\", input: \"$seed\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"$prior\"\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::TypeMismatch {
            field: "if",
            expected: "boolean",
            found: "unknown",
        },
    )
}

#[test]
fn guard_undefined_name() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"$missing\"\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_unknown(&y, "$missing")
}

#[test]
fn guard_self_reference() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"$g\"\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_unknown(&y, "$g")
}

#[test]
fn guard_forward_reference() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"$later\"\n  - id: later\n    set: { output: derived_result, value: \"true\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_unknown(&y, "$later")
}

#[test]
fn guarded_output_finish_aliases() -> Result<(), String> {
    let cases = ["$g", "$step.g", "$steps.g", "1"];
    for alias in cases.iter() {
        let result_field = if *alias == "1" {
            "1".to_string()
        } else {
            format!("\"{}\"", alias)
        };
        let y = yaml(&format!(
            "  - id: seed\n    set: {{ output: x, value: \"42\" }}\n  - id: g\n    do: {{ action: \"1\", input: \"$seed\" }}\n    if: \"true\"\n  - id: end\n    finish: {{ result: {} }}",
            result_field
        ));
        expect_err(
            &y,
            CompileError::PossiblySkippedReference {
                slot: SlotIdx::new(1),
                consumer: "finish.result",
            },
        )?;
    }
    Ok(())
}

#[test]
fn guarded_output_later_do_input() -> Result<(), String> {
    let cases = ["1", "$g"];
    for input in cases.iter() {
        let y = yaml(&format!(
            "  - id: seed\n    set: {{ output: x, value: \"42\" }}\n  - id: g\n    do: {{ action: \"1\", input: \"$seed\" }}\n    if: \"true\"\n  - id: later\n    do: {{ action: \"2\", input: {:?} }}\n  - id: end\n    finish: {{ result: \"$seed\" }}",
            input
        ));
        expect_err(
            &y,
            CompileError::PossiblySkippedReference {
                slot: SlotIdx::new(1),
                consumer: "do.input",
            },
        )?;
    }
    Ok(())
}

#[test]
fn guarded_output_set_expression() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: s\n    set: { output: derived_result, value: \"$g == false\" }\n  - id: end\n    finish: { result: \"$seed\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "expression",
        },
    )
}

#[test]
fn guard_true_still_rejected() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: end\n    finish: { result: \"$g\" }",
    );
    expect_err(
        &y,
        CompileError::PossiblySkippedReference {
            slot: SlotIdx::new(1),
            consumer: "finish.result",
        },
    )
}

#[test]
fn later_guarded_do_identical_guard() -> Result<(), String> {
    let y = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: g2\n    do: { action: \"2\", input: \"$g\" }\n    if: \"true\"\n  - id: end\n    finish: { result: \"$seed\" }",
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
fn guard_requires_terminal_finish() -> Result<(), String> {
    let do_last = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"",
    );
    expect_err(&do_last, CompileError::LastStepMustFinish)?;

    let set_last = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: g\n    do: { action: \"1\", input: \"$seed\" }\n    if: \"true\"\n  - id: s\n    set: { output: derived_result, value: \"1\" }",
    );
    expect_err(&set_last, CompileError::LastStepMustFinish)?;

    let unguarded_do_last = yaml(
        "  - id: seed\n    set: { output: x, value: \"42\" }\n  - id: d\n    do: { action: \"1\", input: \"$seed\" }",
    );
    expect_err(&unguarded_do_last, CompileError::LastStepMustFinish)
}

#[test]
fn guard_text_expression() -> Result<(), String> {
    let y = yaml(
        r#"  - id: seed
    set: { output: x, value: "42" }
  - id: g
    do: { action: "1", input: "$seed" }
    if: '"hello"'
  - id: end
    finish: { result: "$seed" }"#,
    );
    expect_err(
        &y,
        CompileError::ExpressionLoweringUnsupported {
            feature: Box::from("text constants"),
        },
    )
}
