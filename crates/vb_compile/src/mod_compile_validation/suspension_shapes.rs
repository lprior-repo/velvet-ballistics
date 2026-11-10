use super::*;

pub(in crate::mod_compile_validation) fn validate_wait_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "wait", &["until", "event", "timeout"])?;
    let until = optional_text_or_integer_field(body, index, "until")?;
    let event = optional_text_or_integer_field(body, index, "event")?;
    let timeout = optional_text_or_integer_field(body, index, "timeout")?;
    match (until, event, timeout) {
        (Some(()), None, None) | (None, Some(()), _) | (None, None, Some(())) => Ok(()),
        _ => Err(CompileError::StepFieldShape {
            step: index,
            field: "wait",
            expected: "until without timeout, event with optional timeout, or timeout only",
        }),
    }
}

pub(in crate::mod_compile_validation) fn validate_ask_shape(
    body: &Yaml<'_>,
    index: usize,
    last_step: usize,
) -> Result<(), CompileError> {
    reject_last_non_finish(index, last_step)?;
    reject_unknown_primitive_fields(body, index, "ask", &["prompt", "answer", "timeout"])?;
    required_text_or_integer_field(body, index, "prompt", "a non-empty prompt slot")?;
    if body.as_mapping_get("answer").is_some() {
        required_text_or_integer_field(body, index, "answer", "a non-empty answer slot")?;
    }
    optional_text_or_integer_field(body, index, "timeout")?;
    Ok(())
}
