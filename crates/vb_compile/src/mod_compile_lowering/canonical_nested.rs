//! Reuse canonical primitive lowering within a lexical body.

use super::BranchBindings;
use crate::mod_compile_lowering::part_01::ForEachInputs;
use crate::mod_compile_lowering::{SlotCompiler, lower_canonical_step};
use crate::{CompileError, CompileErrors, StepAst};
use vb_core::{SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

pub(super) fn lower_nested(
    step: &StepAst,
    id: StepIdx,
    next: StepIdx,
    bindings: &mut BranchBindings,
    names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    let inputs = ForEachInputs::from_bindings(&bindings.outputs, builder);
    let mut names = names.to_vec();
    lower_canonical_step(
        step,
        id.as_usize(),
        id.as_usize(),
        id,
        Some(next),
        &mut bindings.outputs,
        &mut bindings.steps,
        &mut names,
        &mut bindings.types,
        &inputs,
        builder,
    )?;
    let result = bindings.steps.get(&step.id).copied();
    if let Some(slot) = result {
        bindings.steps.insert(step.id.clone(), slot);
        bindings.types.entry(slot).or_insert(ExprType::Unknown);
    }
    if builder.nodes.last().is_none() {
        return Err(CompileErrors(vec![CompileError::StepIndexOutOfRange {
            value: id.as_usize(),
        }]));
    }
    Ok(result)
}
