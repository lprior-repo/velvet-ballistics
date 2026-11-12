use super::part_01::ForEachInputs;
use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use std::collections::HashMap;
use vb_core::{SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

#[cfg(test)]
pub(crate) use super::part_14::lower_canonical_choose;

#[path = "canonical_scope.rs"]
mod canonical_scope;
pub(super) use canonical_scope::{CanonicalScope, validate_output_alias};

#[path = "canonical_references.rs"]
mod canonical_references;
pub(super) use canonical_references::{resolve_slot_reference, resolve_slot_reference_scoped};

#[path = "canonical_types.rs"]
mod canonical_types;

#[path = "canonical_scalar.rs"]
mod canonical_scalar;
pub(super) use canonical_scalar::{
    compile_canonical_expression, lower_canonical_set_expression,
    lower_canonical_set_expression_with_binding,
};

#[path = "canonical_do.rs"]
mod canonical_do;

#[path = "canonical_guarded_do.rs"]
mod canonical_guarded_do;
use canonical_guarded_do::lower_top_level_do;

#[path = "bounded_for_each.rs"]
mod bounded_for_each;
pub(super) use bounded_for_each::{ForEachLowering, lower_bounded_for_each};

#[path = "for_each_capture.rs"]
pub(super) mod for_each_capture;

#[path = "bounded_collect.rs"]
pub(super) mod bounded_collect;
pub(super) use bounded_collect::{CollectLowering, CollectOutput, lower_bounded_collect};

#[path = "for_each_strict.rs"]
mod for_each_strict;
pub(super) use for_each_strict::validate_for_each_expression;

#[path = "lower_step_scalars.rs"]
mod lower_step_scalars;
use lower_step_scalars::{
    lower_step_ask, lower_step_do, lower_step_finish, lower_step_set, lower_step_together,
    lower_step_wait,
};

#[path = "lower_step_containers.rs"]
mod lower_step_containers;
use lower_step_containers::{
    lower_step_aggregate, lower_step_choose, lower_step_collect, lower_step_for_each,
    lower_step_repeat,
};

/// Borrowed lowering context threaded through one top-level step dispatch.
/// The builder and step names stay with the caller: the dispatcher pads
/// names after the arm's nodes are emitted, so it must still own them.
pub(super) struct StepLowerCtx<'a> {
    pub index: usize,
    pub last: usize,
    pub id: StepIdx,
    pub next: Option<StepIdx>,
    pub outputs: &'a mut HashMap<String, SlotIdx>,
    pub step_outputs: &'a mut HashMap<String, SlotIdx>,
    pub slot_types: &'a mut HashMap<SlotIdx, ExprType>,
    pub for_each_inputs: &'a ForEachInputs,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn lower_canonical_step(
    step: &crate::StepAst,
    index: usize,
    last: usize,
    id: StepIdx,
    next: Option<StepIdx>,
    outputs: &mut HashMap<String, SlotIdx>,
    step_outputs: &mut HashMap<String, SlotIdx>,
    step_names: &mut Vec<Box<str>>,
    slot_types: &mut HashMap<SlotIdx, ExprType>,
    for_each_inputs: &ForEachInputs,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let mut ctx = StepLowerCtx {
        index,
        last,
        id,
        next,
        outputs,
        step_outputs,
        slot_types,
        for_each_inputs,
    };
    let result = dispatch_step(&mut ctx, step, step_names, builder);
    result?;
    extend_step_names_for_generated(step_names, step.id.as_str(), builder.nodes.len());
    Ok(())
}

fn dispatch_step(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    step_names: &mut [Box<str>],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    match &step.primitive {
        crate::StepPrimitive::Set { .. } => lower_step_set(ctx, step, builder),
        crate::StepPrimitive::Do { .. } => lower_step_do(ctx, step, builder),
        crate::StepPrimitive::Finish { .. } => lower_step_finish(ctx, step, builder),
        crate::StepPrimitive::ForEach { .. } => lower_step_for_each(ctx, step, step_names, builder),
        crate::StepPrimitive::Together { .. } => lower_step_together(ctx, step, builder),
        crate::StepPrimitive::Collect { .. } => lower_step_collect(ctx, step, step_names, builder),
        crate::StepPrimitive::Aggregate { .. } => {
            lower_step_aggregate(ctx, step, step_names, builder)
        }
        crate::StepPrimitive::Repeat { .. } => lower_step_repeat(ctx, step, step_names, builder),
        crate::StepPrimitive::Wait { .. } => lower_step_wait(ctx, step, builder),
        crate::StepPrimitive::Ask { .. } => lower_step_ask(ctx, step, builder),
        crate::StepPrimitive::Choose { .. } => lower_step_choose(ctx, step, step_names, builder),
        other => Err(unsupported_primitive(ctx.index, other)),
    }
}

fn unsupported_primitive(step: usize, primitive: &crate::StepPrimitive) -> CompileErrors {
    CompileErrors(vec![CompileError::UnsupportedStepPrimitive {
        step,
        primitive: canonical_primitive_name(primitive),
    }])
}

#[path = "canonical_finish.rs"]
mod canonical_finish;
pub(super) use canonical_finish::{extend_step_names_for_generated, lower_canonical_finish};
