use crate::{CoreResult, EngineError, RunFrame, SlotIdx, SlotValue};

/// Checks suspension operands without advancing or modifying the frame.
pub fn validate_ask_inputs(
    frame: &RunFrame,
    prompt: SlotIdx,
    timeout: Option<SlotIdx>,
) -> CoreResult<()> {
    validate_prompt(*frame.read_slot(prompt)?)?;
    if let Some(timeout) = timeout {
        validate_numeric(*frame.read_slot(timeout)?, "timeout")?;
    }
    Ok(())
}

/// Checks the numeric operand domain shared by Wait and Ask.
pub fn validate_numeric(value: SlotValue, expected: &'static str) -> CoreResult<()> {
    match value {
        SlotValue::I64(_) | SlotValue::F64(_) => Ok(()),
        other => Err(EngineError::TypeMismatch {
            expected,
            found: other.type_name(),
        }),
    }
}

fn validate_prompt(value: SlotValue) -> CoreResult<()> {
    match value {
        SlotValue::Bool(_) => Err(EngineError::TypeMismatch {
            expected: "prompt",
            found: value.type_name(),
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests;
