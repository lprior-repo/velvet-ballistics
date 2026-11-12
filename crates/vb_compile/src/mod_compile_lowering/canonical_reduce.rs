#![forbid(unsafe_code)]

use super::canonical_reduce_initial::parse_typed_initial;
use super::resolve_slot_reference_scoped;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, body_width, checked_step_offset, reduce_direct_body,
};
use vb_core::workflow::node;
use vb_core::{CompiledNodeKind, ConstValue, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

#[path = "canonical_reduce/body.rs"]
mod body;
use body::lower_body;

pub(in crate::mod_compile_lowering) struct ReduceLowering<'a> {
    pub input: &'a str,
    pub variable: &'a str,
    pub initial: &'a str,
    pub body: &'a [crate::StepAst],
    pub next: Option<StepIdx>,
    pub scope: CanonicalScope<'a>,
}

pub(in crate::mod_compile_lowering) fn lower_canonical_aggregate(
    index: usize,
    id: StepIdx,
    config: ReduceLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<(SlotIdx, ExprType), CompileErrors> {
    if config.body.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "reduce.steps",
            expected: "at least one body step",
        }]));
    }
    crate::mod_compile_validation::validate_public_name("reduce.variable", config.variable)
        .map_err(|e| CompileErrors(vec![e]))?;
    super::super::part_02::for_each_capture::validate_reduce_capture(config.variable, config.body)?;
    super::super::part_02::validate_output_alias(config.variable)
        .map_err(|e| CompileErrors(vec![e]))?;
    let input = resolve_slot_reference_scoped(config.input, config.scope)
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.record_slot(input);
    let limit = input_limit(input, builder)?;
    let initial_input = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let next_item = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let iterator_slot = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let accumulator = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let (initial, initial_type) = parse_typed_initial(config.initial, index)?;
    let initial = builder
        .push_constant(initial)
        .map_err(|e| CompileErrors(vec![e]))?;
    let start =
        checked_step_offset(id, 1, "aggregate", "start").map_err(|e| CompileErrors(vec![e]))?;
    let body_width = body_width(config.body, 0).map_err(|e| CompileErrors(vec![e]))?;
    let body_width = checked_width_n(body_width)?;
    let body_step =
        checked_step_offset(id, 2, "aggregate", "body").map_err(|e| CompileErrors(vec![e]))?;
    let copy_step = checked_step_offset(body_step, body_width, "aggregate", "copy")
        .map_err(|e| CompileErrors(vec![e]))?;
    let next_step = if !reduce_direct_body(config.body) {
        checked_step_offset(copy_step, 1, "aggregate", "next")
            .map_err(|e| CompileErrors(vec![e]))?
    } else {
        copy_step
    };
    let done = checked_step_offset(next_step, 1, "aggregate", "done")
        .map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node(
        id,
        CompiledNodeKind::Copy { source: input },
        Some(initial_input),
        Some(start),
    ));
    builder.push_node(node(
        start,
        CompiledNodeKind::ReduceStart {
            limit,
            input: initial_input,
            accumulator,
            initial,
            body: body_step,
            done,
        },
        Some(iterator_slot),
        None,
    ));
    let inferred = lower_body(
        &config,
        index,
        body_step,
        accumulator,
        next_step,
        initial_type,
        builder,
    )?;
    builder.push_node(node(
        next_step,
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body: body_step,
            done,
        },
        Some(next_item),
        None,
    ));
    builder.push_node(node(
        done,
        CompiledNodeKind::ReduceFinish { accumulator },
        Some(accumulator),
        config.next,
    ));
    Ok((accumulator, inferred))
}

use super::list_input::input_limit;

#[cfg(test)]
#[path = "reduce_limit_tests.rs"]
mod limit_tests;
#[cfg(test)]
#[path = "canonical_reduce_tests.rs"]
mod tests;

pub(in crate::mod_compile_lowering) fn checked_width_n(width: usize) -> Result<u16, CompileErrors> {
    u16::try_from(width)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: width }]))
}
