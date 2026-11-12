#![forbid(unsafe_code)]

#[path = "canonical_reduce/body.rs"]
mod body;

use super::canonical_reduce_initial::parse_typed_initial;
use super::emit_single_body_set;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::part_02::for_each_capture;
use crate::mod_compile_lowering::part_14::BodyCtx;
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, body_width, checked_step_offset, slot_from_text,
};
use body::{lower_body, lower_multi_body};
use vb_core::workflow::node;
use vb_core::{CompiledNodeKind, ConstValue, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

pub(in crate::mod_compile_lowering) struct ReduceLowering<'a> {
    pub input: &'a str,
    pub variable: &'a str,
    pub initial: &'a str,
    pub body: &'a [crate::StepAst],
    pub next: Option<StepIdx>,
    pub scope: CanonicalScope<'a>,
    pub nested: BodyCtx<'a>,
}

struct ReduceEdges {
    body: StepIdx,
    next: StepIdx,
    done: StepIdx,
}

pub(in crate::mod_compile_lowering) fn lower_canonical_aggregate(
    index: usize,
    id: StepIdx,
    config: ReduceLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<(SlotIdx, ExprType), CompileErrors> {
    crate::mod_compile_validation::validate_public_name("reduce.variable", config.variable)
        .map_err(|e| CompileErrors(vec![e]))?;
    validate_body(index, &config)?;
    let (initial_value, initial_type) = parse_typed_initial(config.initial, index)?;
    let edges = plan_edges(id, config.body)?;
    emit_reduce(
        index,
        id,
        &config,
        &edges,
        initial_value,
        initial_type,
        builder,
    )
}

pub(in crate::mod_compile_lowering) fn checked_width_n(width: usize) -> Result<u16, CompileErrors> {
    u16::try_from(width)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: width }]))
}

fn plan_edges(id: StepIdx, body: &[crate::StepAst]) -> Result<ReduceEdges, CompileErrors> {
    let body_edge =
        checked_step_offset(id, 2, "aggregate", "body").map_err(|e| CompileErrors(vec![e]))?;
    let width = body_width(body, 0).map_err(|e| CompileErrors(vec![e]))?;
    let width_n = checked_width_n(width)?;
    let next = checked_step_offset(body_edge, width_n, "aggregate", "next")
        .map_err(|e| CompileErrors(vec![e]))?;
    let done =
        checked_step_offset(next, 1, "aggregate", "done").map_err(|e| CompileErrors(vec![e]))?;
    Ok(ReduceEdges {
        body: body_edge,
        next,
        done,
    })
}

#[allow(clippy::too_many_arguments)]
fn emit_reduce(
    index: usize,
    id: StepIdx,
    config: &ReduceLowering<'_>,
    edges: &ReduceEdges,
    initial_value: ConstValue,
    initial_type: ExprType,
    builder: &mut SlotCompiler,
) -> Result<(SlotIdx, ExprType), CompileErrors> {
    let input = slot_from_text(config.input, index, "aggregate.input")?;
    builder.record_slot(input);
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
    let initial = builder
        .push_constant(initial_value)
        .map_err(|e| CompileErrors(vec![e]))?;
    let start =
        checked_step_offset(id, 1, "aggregate", "start").map_err(|e| CompileErrors(vec![e]))?;
    builder.push_node(node(
        id,
        CompiledNodeKind::Copy { source: input },
        Some(initial_input),
        Some(start),
    ));
    builder.push_node(node(
        start,
        CompiledNodeKind::ReduceStart {
            input: initial_input,
            accumulator,
            initial,
            body: edges.body,
            done: edges.done,
        },
        Some(iterator_slot),
        None,
    ));
    let inferred = if config.body.len() >= 2 {
        lower_multi_body(config, index, edges, accumulator, initial_type, builder)?
    } else {
        lower_body(
            config,
            index,
            edges.body,
            accumulator,
            edges.next,
            initial_type,
            builder,
        )?
    };
    emit_reduce_tail(
        config,
        edges,
        iterator_slot,
        accumulator,
        next_item,
        builder,
    );
    Ok((accumulator, inferred))
}

fn emit_reduce_tail(
    config: &ReduceLowering<'_>,
    edges: &ReduceEdges,
    iterator_slot: SlotIdx,
    accumulator: SlotIdx,
    next_item: SlotIdx,
    builder: &mut SlotCompiler,
) {
    builder.push_node(node(
        edges.next,
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator,
            body: edges.body,
            done: edges.done,
        },
        Some(next_item),
        None,
    ));
    builder.push_node(node(
        edges.done,
        CompiledNodeKind::ReduceFinish { accumulator },
        Some(accumulator),
        config.next,
    ));
}

fn validate_body(index: usize, config: &ReduceLowering<'_>) -> Result<(), CompileErrors> {
    if config.body.is_empty() {
        return Err(field_error(index, "steps", "at least one set or do step"));
    }
    for_each_capture::validate_reduce_capture(config.variable, config.body)
}

fn field_error(step: usize, field: &'static str, expected: &'static str) -> CompileErrors {
    CompileErrors(vec![CompileError::StepFieldShape {
        step,
        field,
        expected,
    }])
}

#[cfg(test)]
#[path = "canonical_reduce_tests.rs"]
mod tests;
