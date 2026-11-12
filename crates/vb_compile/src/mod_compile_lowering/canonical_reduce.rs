#![forbid(unsafe_code)]

use super::canonical_reduce_initial::parse_typed_initial;
use super::emit_single_body_set;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::part_02::for_each_capture;
use crate::mod_compile_lowering::part_14::{BodyCtx, emit_scoped_flat_body};
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, body_width, checked_step_offset,
    lower_canonical_set_expression_with_binding, slot_from_text,
};
use std::collections::HashMap;
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

fn lower_multi_body(
    config: &ReduceLowering<'_>,
    index: usize,
    edges: &ReduceEdges,
    accumulator: SlotIdx,
    initial_type: ExprType,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    let last = config
        .body
        .last()
        .ok_or_else(|| field_error(index, "steps", "non-empty body"))?;
    if !matches!(
        last.primitive,
        crate::StepPrimitive::Set { .. } | crate::StepPrimitive::Do { .. }
    ) {
        return Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: index,
                primitive: super::super::canonical_primitive_name(&last.primitive),
            },
        ]));
    }
    // Borrowed overlay: tiny maps + parent chain (no whole-map clone).
    let mut overlay_outputs = HashMap::with_capacity(1);
    overlay_outputs.insert(config.variable.to_owned(), accumulator);
    let mut overlay_types = HashMap::with_capacity(1);
    overlay_types.insert(accumulator, initial_type);
    let scope = CanonicalScope {
        outputs: &overlay_outputs,
        step_outputs: config.scope.step_outputs,
        slot_types: &overlay_types,
        parent: Some(&config.scope),
    };
    let (_, final_type) = emit_scoped_flat_body(
        config.body,
        edges.body,
        edges.next,
        scope,
        config.nested,
        builder,
        "reduce",
        Some(accumulator),
    )?;
    accumulator_type(index, final_type, initial_type, config)
}

fn accumulator_type(
    index: usize,
    final_type: ExprType,
    initial_type: ExprType,
    config: &ReduceLowering<'_>,
) -> Result<ExprType, CompileErrors> {
    let last_is_set = config
        .body
        .last()
        .is_some_and(|step| matches!(step.primitive, crate::StepPrimitive::Set { .. }));
    if last_is_set && final_type != initial_type {
        return Err(field_error(
            index,
            "reduce.steps",
            "a set value preserving the accumulator type",
        ));
    }
    if last_is_set || final_type != ExprType::Unknown {
        Ok(initial_type)
    } else {
        Ok(ExprType::Unknown)
    }
}

fn lower_body(
    config: &ReduceLowering<'_>,
    index: usize,
    id: StepIdx,
    accumulator: SlotIdx,
    next: StepIdx,
    initial_type: ExprType,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    let [step] = config.body else {
        return Err(field_error(index, "steps", "exactly one set step"));
    };
    let crate::StepPrimitive::Set { value, .. } = &step.primitive else {
        emit_single_body_set(
            config.body,
            id,
            index,
            accumulator,
            Some(next),
            builder,
            false,
        )?;
        return Ok(ExprType::Unknown);
    };
    // Single-binding overlay for type only; outputs reused via parent chain.
    let mut overlay_types = HashMap::with_capacity(1);
    overlay_types.insert(accumulator, initial_type);
    let inferred = lower_canonical_set_expression_with_binding(
        id,
        accumulator,
        value,
        Some(next),
        CanonicalScope {
            outputs: config.scope.outputs,
            step_outputs: config.scope.step_outputs,
            slot_types: &overlay_types,
            parent: Some(&config.scope),
        },
        config.variable,
        builder,
    )?;
    if inferred != initial_type {
        return Err(field_error(
            index,
            "reduce.steps",
            "a set value preserving the accumulator type",
        ));
    }
    Ok(inferred)
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
