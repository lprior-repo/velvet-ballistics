#![forbid(unsafe_code)]
//! Scalar and leaf step helpers for `lower_canonical_step` (part_02).
//!
//! The dispatcher matches on the `StepPrimitive` discriminant and
//! delegates each arm here; every helper below is a verbatim relocation
//! of one former arm body, prefixed by a `let`-`else` that destructures
//! the already-matched variant.

use super::*;
use crate::mod_compile_errors::{CompileError, CompileErrors};

pub(super) fn lower_step_set(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Set { output, value } = &step.primitive else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    validate_output_alias(output).map_err(|e| CompileErrors(vec![e]))?;
    let slot = slot_idx_for_step(ctx.index).map_err(|e| CompileErrors(vec![e]))?;
    if ctx.outputs.contains_key(output) {
        return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
            name: Box::from(output.as_str()),
        }]));
    }
    let inferred = lower_canonical_set_expression(
        ctx.id,
        slot,
        value,
        ctx.next,
        CanonicalScope {
            outputs: ctx.outputs,
            step_outputs: ctx.step_outputs,
            slot_types: ctx.slot_types,
        },
        builder,
    )?;
    ctx.outputs.insert(output.to_owned(), slot);
    ctx.step_outputs.insert(step.id.to_string(), slot);
    builder.record_slot(slot);
    ctx.slot_types.insert(slot, inferred);
    Ok(())
}

pub(super) fn lower_step_do(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Do { .. } = &step.primitive else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    let output_slot = lower_top_level_do(
        step,
        ctx.index,
        ctx.id,
        ctx.next,
        CanonicalScope {
            outputs: ctx.outputs,
            step_outputs: ctx.step_outputs,
            slot_types: ctx.slot_types,
        },
        builder,
    )?;
    if let Some(slot) = output_slot {
        ctx.step_outputs.insert(step.id.to_string(), slot);
        // Do action outputs are dynamic; no static type inference
    }
    Ok(())
}

pub(super) fn lower_step_finish(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Finish { result } = &step.primitive else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    lower_canonical_finish(
        ctx.index,
        ctx.last,
        ctx.id,
        result,
        ctx.outputs,
        ctx.step_outputs,
        builder,
    )
}

pub(super) fn lower_step_wait(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Wait { event, timeout } = &step.primitive else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    lower_canonical_wait(
        ctx.index,
        ctx.id,
        event.as_deref(),
        timeout.as_deref(),
        ctx.next,
        CanonicalScope {
            outputs: ctx.outputs,
            step_outputs: ctx.step_outputs,
            slot_types: ctx.slot_types,
        },
        builder,
    )
}

pub(super) fn lower_step_ask(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Ask { prompt, timeout } = &step.primitive else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    lower_canonical_ask(
        ctx.index,
        ctx.id,
        prompt,
        timeout.as_deref(),
        ctx.next,
        CanonicalScope {
            outputs: ctx.outputs,
            step_outputs: ctx.step_outputs,
            slot_types: ctx.slot_types,
        },
        builder,
    )
}

pub(super) fn lower_step_together(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Together { branches } = &step.primitive else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    lower_canonical_parallel(ctx.index, ctx.id, branches, ctx.next, builder)
}
