#![forbid(unsafe_code)]

use super::{
    emit_scoped_typed_body, lower_canonical_set_expression_with_binding,
    parse_i64_field, resolve_slot_reference,
};
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, body_width, checked_step_offset, reduce_direct_body,
};
use vb_core::workflow::node;
use vb_core::{CompiledNodeKind, ConstValue, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

use crate::mod_compile_lowering::part_05::canonical_primitive_name;

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
        return Err(CompileErrors(vec![CompileError::StepFieldShape { step: index, field: "reduce.steps", expected: "at least one body step" }]));
    }
    crate::mod_compile_validation::validate_public_name("reduce.variable", config.variable)
        .map_err(|e| CompileErrors(vec![e]))?;
    let input = resolve_slot_reference(config.input, config.scope.outputs, config.scope.step_outputs)
        .map_err(|e| CompileErrors(vec![e]))?;
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
    let initial = parse_i64_field(config.initial, index, "aggregate.initial")?;
    let initial = builder
        .push_constant(ConstValue::I64(initial))
        .map_err(|e| CompileErrors(vec![e]))?;
    let start =
        checked_step_offset(id, 1, "aggregate", "start").map_err(|e| CompileErrors(vec![e]))?;
    let body_width = body_width(config.body, 0)
        .map_err(|e| CompileErrors(vec![e]))?;
    let body_width: u16 = body_width
        .try_into()
        .map_err(|_| {
            CompileErrors(vec![CompileError::StepIndexOutOfRange {
                value: body_width,
            }])
        })?;
    let body_step = checked_step_offset(id, 2, "aggregate", "body")
        .map_err(|e| CompileErrors(vec![e]))?;
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
            input: initial_input,
            accumulator,
            initial,
            body: body_step,
            done,
        },
        Some(iterator_slot),
        None,
    ));
    let inferred = lower_body(&config, index, body_step, accumulator, next_step, builder)?;
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

fn lower_body(
    config: &ReduceLowering<'_>,
    index: usize,
    id: StepIdx,
    accumulator: SlotIdx,
    next: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    if reduce_direct_body(config.body) {
        lower_single_step_body(config, index, id, accumulator, next, builder)
    } else {
        lower_multi_step_body(config, index, id, accumulator, next, builder)
    }
}

fn lower_single_step_body(
    config: &ReduceLowering<'_>,
    index: usize,
    id: StepIdx,
    accumulator: SlotIdx,
    next: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    if config.body.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "reduce.steps",
            expected: "at least one body step",
        }]));
    }
    let step = config.body.first().ok_or_else(|| CompileErrors(vec![CompileError::StepFieldShape {
        step: index, field: "reduce.steps", expected: "at least one body step",
    }]))?;
    match &step.primitive {
        crate::StepPrimitive::Do { action: _, input: _ } => {
            return Err(CompileErrors(vec![CompileError::StepFieldShape {
                step: index,
                field: "reduce.steps",
                expected: "a set value preserving the i64 accumulator type",
            }]));
        }
        crate::StepPrimitive::Set { value, .. } => {
            let mut types = config.scope.slot_types.clone();
            types.insert(accumulator, ExprType::I64);
            let inferred = lower_canonical_set_expression_with_binding(
                id,
                accumulator,
                value,
                Some(next),
                CanonicalScope {
                    outputs: config.scope.outputs,
                    step_outputs: config.scope.step_outputs,
                    slot_types: &types,
                },
                config.variable,
                builder,
            )?;
            if inferred != ExprType::I64 {
                return Err(CompileErrors(vec![CompileError::StepFieldShape {
                    step: index,
                    field: "reduce.steps",
                    expected: "a set value preserving the i64 accumulator type",
                }]));
            }
            Ok(inferred)
        }
        other => Err(CompileErrors(vec![CompileError::UnsupportedStepPrimitive {
            step: index,
            primitive: canonical_primitive_name(other),
        }])),
    }
}

fn lower_multi_step_body(
    config: &ReduceLowering<'_>,
    index: usize,
    id: StepIdx,
    accumulator: SlotIdx,
    next: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    let mut types = config.scope.slot_types.clone();
    types.insert(accumulator, ExprType::I64);
    let mut outputs = config.scope.outputs.clone();
    outputs.insert(config.variable.to_owned(), accumulator);
    let scope = CanonicalScope {
        outputs: &outputs,
        step_outputs: config.scope.step_outputs,
        slot_types: &types,
    };
    let width = body_width(config.body, 0).map_err(|e| CompileErrors(vec![e]))?;
    let width = u16::try_from(width).map_err(|_| CompileErrors(vec![
        CompileError::StepIndexOutOfRange { value: width },
    ]))?;
    let final_step = checked_step_offset(id, width, "reduce", "copy")
        .map_err(|e| CompileErrors(vec![e]))?;
    let last = emit_scoped_typed_body(config.body, id, final_step, scope, builder, "reduce")?;
    let (result_slot, inferred) = last.ok_or(CompileErrors(vec![
        CompileError::StepFieldShape {
            step: index,
            field: "reduce.steps",
            expected: "at least one set or do step",
        },
    ]))?;
    if !matches!(inferred, ExprType::I64 | ExprType::Unknown) {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "reduce.steps",
            expected: "a set value preserving the i64 accumulator type",
        }]));
    }
    builder.push_node(node(
        final_step,
        CompiledNodeKind::Copy {
            source: result_slot,
        },
        Some(accumulator),
        Some(next),
    ));
    Ok(inferred)
}

#[cfg(test)]
#[path = "canonical_reduce_tests.rs"]
mod tests;
