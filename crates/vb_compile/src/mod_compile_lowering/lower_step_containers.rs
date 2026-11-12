#![forbid(unsafe_code)]
//! Container step helpers for `lower_canonical_step` (part_02).
//!
//! The dispatcher matches on the `StepPrimitive` discriminant and
//! delegates each arm here; every helper below is a verbatim relocation
//! of one former arm body, prefixed by a `let`-`else` that destructures
//! the already-matched variant.

use super::*;
use crate::mod_compile_errors::CompileErrors;

pub(super) fn lower_step_for_each(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    step_names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::ForEach {
        input,
        variable,
        at_once,
        body,
    } = &step.primitive
    else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    lower_bounded_for_each(
        ctx.index,
        ctx.id,
        ForEachLowering {
            input,
            variable,
            at_once: *at_once,
            body,
            next: ctx.next,
            scope: CanonicalScope {
                outputs: ctx.outputs,
                step_outputs: ctx.step_outputs,
                slot_types: ctx.slot_types,
                parent: None,
            },
            inputs: ctx.for_each_inputs,
            nested: BodyCtx {
                inputs: ctx.for_each_inputs,
                step_names,
                index: ctx.index,
                depth: 0,
            },
        },
        builder,
    )
    .map(|_| ())
}

pub(super) fn lower_step_collect(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    step_names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Collect {
        source,
        variable,
        pages,
        items,
        body,
        ..
    } = &step.primitive
    else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    let output = lower_bounded_collect(
        ctx.index,
        ctx.id,
        CollectLowering {
            source,
            variable,
            pages: *pages,
            items: *items,
            body,
            next: ctx.next,
            scope: CanonicalScope {
                outputs: ctx.outputs,
                step_outputs: ctx.step_outputs,
                slot_types: ctx.slot_types,
                parent: None,
            },
            nested: BodyCtx {
                inputs: ctx.for_each_inputs,
                step_names,
                index: ctx.index,
                depth: 0,
            },
        },
        builder,
    )?;
    ctx.step_outputs.insert(step.id.to_string(), output.finish);
    ctx.slot_types.insert(output.finish, ExprType::List);
    Ok(())
}

pub(super) fn lower_step_aggregate(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    step_names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Aggregate {
        input,
        variable,
        initial,
        body,
    } = &step.primitive
    else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    let (slot, inferred) = lower_canonical_aggregate(
        ctx.index,
        ctx.id,
        ReduceLowering {
            input,
            variable,
            initial,
            body,
            next: ctx.next,
            scope: CanonicalScope {
                outputs: ctx.outputs,
                step_outputs: ctx.step_outputs,
                slot_types: ctx.slot_types,
                parent: None,
            },
            nested: BodyCtx {
                inputs: ctx.for_each_inputs,
                step_names,
                index: ctx.index,
                depth: 0,
            },
        },
        builder,
    )?;
    ctx.step_outputs.insert(step.id.to_string(), slot);
    ctx.slot_types.insert(slot, inferred);
    Ok(())
}

pub(super) fn lower_step_repeat(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    step_names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Repeat {
        max_attempts, body, ..
    } = &step.primitive
    else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    lower_canonical_repeat(
        ctx.index,
        ctx.id,
        RepeatLowering {
            max_attempts: *max_attempts,
            body,
            next: ctx.next,
            scope: CanonicalScope {
                outputs: ctx.outputs,
                step_outputs: ctx.step_outputs,
                slot_types: ctx.slot_types,
                parent: None,
            },
            inputs: ctx.for_each_inputs,
            step_names,
            depth: 0,
        },
        builder,
    )
    .map(|_| ())
}

pub(super) fn lower_step_choose(
    ctx: &mut StepLowerCtx<'_>,
    step: &crate::StepAst,
    step_names: &[Box<str>],
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let crate::StepPrimitive::Choose {
        branches,
        otherwise,
    } = &step.primitive
    else {
        return Err(unsupported_primitive(ctx.index, &step.primitive));
    };
    lower_canonical_choose_in_scope(
        ctx.id,
        branches,
        otherwise.as_deref(),
        ctx.next,
        ChooseScope {
            index: ctx.index,
            names: step_names,
            bindings: CanonicalScope {
                outputs: ctx.outputs,
                step_outputs: ctx.step_outputs,
                slot_types: ctx.slot_types,
                parent: None,
            },
            inputs: ctx.for_each_inputs,
            depth: 0,
        },
        builder,
    )
}
