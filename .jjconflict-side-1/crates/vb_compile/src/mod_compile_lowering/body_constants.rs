use super::*;

pub(in crate::mod_compile_lowering) fn body_constant_index(
    builder: &mut SlotCompiler,
    value: &str,
    step: usize,
    reuse_first_constant: bool,
) -> Result<ConstIdx, CompileErrors> {
    let constant = parse_i64_field(value, step, "set.value")?;
    if reuse_first_constant
        && let Some(index) = builder
            .constants
            .iter()
            .position(|entry| *entry == ConstValue::I64(constant))
    {
        let idx = u16::try_from(index).map_err(|_| {
            CompileErrors(vec![CompileError::Workflow(
                WorkflowError::ConstOutOfBounds {
                    constant: ConstIdx::new(u16::MAX),
                },
            )])
        })?;
        return Ok(ConstIdx::new(idx));
    }
    builder
        .push_constant(ConstValue::I64(constant))
        .map_err(|e| CompileErrors(vec![e]))
}

#[cfg(test)]
#[path = "body_constants_tests.rs"]
mod tests;
