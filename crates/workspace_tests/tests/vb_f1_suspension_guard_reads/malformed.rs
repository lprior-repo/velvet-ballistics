use super::*;

#[test]
fn malformed_suspension_field_context() -> Result<(), String> {
    let cases = [
        (
            false,
            "$alias",
            "bad",
            "ask.prompt",
            "integer string",
            false,
        ),
        (
            false,
            "$alias",
            "",
            "ask.prompt",
            "field shape error: ask.prompt expected non-empty string",
            true,
        ),
        (
            false,
            "$steps.clock",
            "bad",
            "ask.timeout",
            "integer string",
            false,
        ),
        (
            false,
            "$steps.clock",
            "",
            "timeout",
            "a string or integer",
            true,
        ),
        (true, "$alias", "bad", "wait.event", "integer string", false),
        (
            true,
            "$alias",
            "",
            "event",
            "wait.event must be non-empty",
            true,
        ),
        (
            true,
            "$steps.clock",
            "bad",
            "wait.timeout",
            "integer string",
            false,
        ),
        (
            true,
            "$steps.clock",
            "",
            "timeout",
            "a string or integer",
            true,
        ),
    ];
    for (wait, old, text, field, expected, is_empty) in cases.iter() {
        let base = if *wait {
            derive_wait(SOURCE)
        } else {
            SOURCE.to_owned()
        };
        let modified = base.replace(old, text);
        if *is_empty && (*field == "ask.prompt" || *field == "event") {
            expect_err(
                &modified,
                CompileError::CanonicalYaml {
                    category: "field_shape",
                    message: Box::from(*expected),
                },
            )?;
        } else {
            expect_err(
                &modified,
                CompileError::StepFieldShape {
                    step: 2,
                    field: *field,
                    expected: *expected,
                },
            )?;
        }
    }
    Ok(())
}
