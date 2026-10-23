//! Borrowed lexical environment shared by canonical scalar and action lowering.

use std::collections::HashMap;
use vb_core::SlotIdx;
use vb_expr::expr_typecheck::ExprType;

#[derive(Clone, Copy)]
pub(in crate::mod_compile_lowering) struct CanonicalScope<'a> {
    pub outputs: &'a HashMap<String, SlotIdx>,
    pub step_outputs: &'a HashMap<String, SlotIdx>,
    pub slot_types: &'a HashMap<SlotIdx, ExprType>,
    pub input_list_max: &'a HashMap<String, u32>,
}

/// Declaration roots are not mutable output aliases, in any lexical scope.
pub(in crate::mod_compile_lowering) fn validate_output_alias(
    output: &str,
) -> Result<(), crate::CompileError> {
    let reserved = output.starts_with('$')
        || ["input.", "vars.", "secrets.", "step.", "steps."]
            .iter()
            .any(|root| output.starts_with(root));
    if reserved {
        return Err(crate::CompileError::InvalidName {
            field: "set.output",
            value: output.into(),
        });
    }
    Ok(())
}
