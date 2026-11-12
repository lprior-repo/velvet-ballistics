#![forbid(unsafe_code)]

use super::canonical_reduce_initial::parse_typed_initial;
use super::field_error;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::part_02::for_each_capture;
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, body_width, checked_step_offset, reduce_direct_body,
    resolve_slot_reference_scoped,
};
use vb_core::workflow::node;
use vb_core::{CompiledNodeKind, ConstIdx, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

#[path = "canonical_reduce/body.rs"]
mod body;

pub(in crate::mod_compile_lowering) struct ReduceLowering<'a> {
    pub input: &'a str,
    pub variable: &'a str,
    pub initial: &'a str,
    pub body: &'a [crate::StepAst],
    pub next: Option<StepIdx>,
    pub scope: CanonicalScope<'a>,
}

struct ReduceInput<'a> {
    config: ReduceLowering<'a>,
    index: usize,
    id: StepIdx,
    slot: SlotIdx,
    limit: u32,
}

struct ReduceSlots {
    initial_input: SlotIdx,
    next_item: SlotIdx,
    iterator_slot: SlotIdx,
    accumulator: SlotIdx,
}

struct ReduceEdges {
    start: StepIdx,
    body: StepIdx,
    next: StepIdx,
    done: StepIdx,
}

struct ReducePlan<'a> {
    source: ReduceInput<'a>,
    slots: ReduceSlots,
    edges: ReduceEdges,
    initial: ConstIdx,
    initial_type: ExprType,
}

pub(in crate::mod_compile_lowering) fn lower_canonical_aggregate(
    index: usize,
    id: StepIdx,
    config: ReduceLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<(SlotIdx, ExprType), CompileErrors> {
    let input = prepare_input(index, id, config, builder)?;
    let plan = prepare_plan(input, builder)?;
    emit_start(&plan, builder);
    let inferred = body::lower_body(
        &plan.source.config,
        plan.source.index,
        plan.edges.body,
        plan.slots.accumulator,
        plan.edges.next,
        plan.initial_type,
        builder,
    )?;
    emit_tail(&plan, builder);
    Ok((plan.slots.accumulator, inferred))
}

fn prepare_input<'a>(
    index: usize,
    id: StepIdx,
    config: ReduceLowering<'a>,
    builder: &mut SlotCompiler,
) -> Result<ReduceInput<'a>, CompileErrors> {
    validate_body(index, &config)?;
    let slot = resolve_slot_reference_scoped(config.input, config.scope)?;
    builder.record_slot(slot);
    let limit = input_limit(slot, builder)?;
    Ok(ReduceInput {
        config,
        index,
        id,
        slot,
        limit,
    })
}

fn validate_body(index: usize, config: &ReduceLowering<'_>) -> Result<(), CompileErrors> {
    if config.body.is_empty() {
        return Err(field_error(index, "reduce.steps", "at least one body step"));
    }
    crate::mod_compile_validation::validate_public_name("reduce.variable", config.variable)?;
    for_each_capture::validate_reduce_capture(config.variable, config.body)?;
    crate::mod_compile_lowering::part_02::validate_output_alias(config.variable)?;
    Ok(())
}

fn prepare_plan<'a>(
    source: ReduceInput<'a>,
    builder: &mut SlotCompiler,
) -> Result<ReducePlan<'a>, CompileErrors> {
    let slots = allocate_slots(builder)?;
    let (initial_value, initial_type) = parse_typed_initial(source.config.initial, source.index)?;
    let initial = builder
        .push_constant(initial_value)
        .map_err(|e| CompileErrors(vec![e]))?;
    let edges = plan_edges(source.id, source.config.body)?;
    Ok(ReducePlan {
        source,
        slots,
        edges,
        initial,
        initial_type,
    })
}

fn allocate_slots(builder: &mut SlotCompiler) -> Result<ReduceSlots, CompileErrors> {
    Ok(ReduceSlots {
        initial_input: builder.allocate_slot()?,
        next_item: builder.allocate_slot()?,
        iterator_slot: builder.allocate_slot()?,
        accumulator: builder.allocate_slot()?,
    })
}

fn plan_edges(id: StepIdx, body: &[crate::StepAst]) -> Result<ReduceEdges, CompileErrors> {
    let start = checked_step_offset(id, 1, "aggregate", "start")?;
    let width = checked_width_n(body_width(body, 0)?)?;
    let body_step = checked_step_offset(id, 2, "aggregate", "body")?;
    let copy = checked_step_offset(body_step, width, "aggregate", "copy")?;
    let next = if !reduce_direct_body(body) {
        checked_step_offset(copy, 1, "aggregate", "next")?
    } else {
        copy
    };
    let done = checked_step_offset(next, 1, "aggregate", "done")?;
    Ok(ReduceEdges {
        start,
        body: body_step,
        next,
        done,
    })
}

fn emit_initial_copy(plan: &ReducePlan<'_>, builder: &mut SlotCompiler) {
    builder.push_node(node(
        plan.source.id,
        CompiledNodeKind::Copy {
            source: plan.source.slot,
        },
        Some(plan.slots.initial_input),
        Some(plan.edges.start),
    ));
}

fn emit_start(plan: &ReducePlan<'_>, builder: &mut SlotCompiler) {
    emit_initial_copy(plan, builder);
    builder.push_node(node(
        plan.edges.start,
        CompiledNodeKind::ReduceStart {
            limit: plan.source.limit,
            input: plan.slots.initial_input,
            accumulator: plan.slots.accumulator,
            initial: plan.initial,
            body: plan.edges.body,
            done: plan.edges.done,
        },
        Some(plan.slots.iterator_slot),
        None,
    ));
}

fn emit_tail(plan: &ReducePlan<'_>, builder: &mut SlotCompiler) {
    builder.push_node(node(
        plan.edges.next,
        CompiledNodeKind::ReduceNext {
            iterator_slot: plan.slots.iterator_slot,
            accumulator: plan.slots.accumulator,
            body: plan.edges.body,
            done: plan.edges.done,
        },
        Some(plan.slots.next_item),
        None,
    ));
    builder.push_node(node(
        plan.edges.done,
        CompiledNodeKind::ReduceFinish {
            accumulator: plan.slots.accumulator,
        },
        Some(plan.slots.accumulator),
        plan.source.config.next,
    ));
}

pub(in crate::mod_compile_lowering) fn checked_width_n(width: usize) -> Result<u16, CompileErrors> {
    u16::try_from(width)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: width }]))
}

use super::list_input::input_limit;

#[cfg(test)]
#[path = "reduce_limit_tests.rs"]
mod limit_tests;
#[cfg(test)]
#[path = "canonical_reduce_tests.rs"]
mod tests;
