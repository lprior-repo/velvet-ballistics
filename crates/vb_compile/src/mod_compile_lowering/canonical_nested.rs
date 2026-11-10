//! Reuse canonical primitive lowering within a lexical body.

use std::collections::HashMap;
use vb_core::{SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;
use crate::{CompileError, CompileErrors, StepAst};
use crate::mod_compile_lowering::{SlotCompiler, lower_canonical_step};
use crate::mod_compile_lowering::part_01::ForEachInputs;

pub(super) fn lower_nested(
    step: &StepAst,
    id: StepIdx,
    next: StepIdx,
    outputs: &mut HashMap<String, SlotIdx>,
    steps: &mut HashMap<String, SlotIdx>,
    types: &mut HashMap<SlotIdx, ExprType>,
    names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    let inputs = ForEachInputs::from_bindings(outputs, builder);
    let mut names = names.to_vec();
    lower_canonical_step(step, id.as_usize(), id.as_usize(), id, Some(next),
        outputs, steps, &mut names, types, &inputs, builder)?;
    let result = steps.get(&step.id).copied();
    if let Some(slot) = result {
        steps.insert(step.id.clone(), slot);
        types.entry(slot).or_insert(ExprType::Unknown);
    }
    if builder.nodes.last().is_none() {
        return Err(CompileErrors(vec![CompileError::StepIndexOutOfRange { value: id.as_usize() }]));
    }
    Ok(result)
}
