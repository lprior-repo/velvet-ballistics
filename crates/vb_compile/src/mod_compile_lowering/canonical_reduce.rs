#![forbid(unsafe_code)]

use super::{
    emit_scoped_typed_body, lower_canonical_set_expression_with_binding,
    resolve_slot_reference_scoped,
};
use super::canonical_reduce_initial::parse_typed_initial;
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
    let (initial, initial_type) =
        parse_typed_initial(config.initial, index)?;
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
            limit: limit.into(),
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

fn lower_body(
    config: &ReduceLowering<'_>,
    index: usize,
    id: StepIdx,
    accumulator: SlotIdx,
    next: StepIdx,
    initial_type: ExprType,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    if reduce_direct_body(config.body) {
        lower_single_step_body(config, index, id, accumulator, next, initial_type, builder)
    } else {
        lower_multi_step_body(config, index, id, accumulator, next, initial_type, builder)
    }
}

fn lower_single_step_body(
    config: &ReduceLowering<'_>,
    index: usize,
    id: StepIdx,
    accumulator: SlotIdx,
    next: StepIdx,
    initial_type: ExprType,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    if config.body.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "reduce.steps",
            expected: "at least one body step",
        }]));
    }
    let step = config.body.first().ok_or_else(|| {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "reduce.steps",
            expected: "at least one body step",
        }])
    })?;
    match &step.primitive {
        crate::StepPrimitive::Do {
            action: _,
            input: _,
        } => Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "reduce.steps",
            expected: "a set value preserving the accumulator type",
        }])),
        crate::StepPrimitive::Set { value, .. } => {
            let mut types = std::collections::HashMap::new();
            types.insert(accumulator, initial_type);
            let inferred = lower_canonical_set_expression_with_binding(
                id,
                accumulator,
                value,
                Some(next),
                CanonicalScope {
                    outputs: config.scope.outputs,
                    step_outputs: config.scope.step_outputs,
                    slot_types: &types,
                    parent: Some(&config.scope),
                },
                config.variable,
                builder,
            )?;
            if inferred != initial_type {
                return Err(CompileErrors(vec![CompileError::StepFieldShape {
                    step: index,
                    field: "reduce.steps",
                    expected: "a set value preserving the accumulator type",
                }]));
            }
            Ok(inferred)
        }
        other => Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: index,
                primitive: canonical_primitive_name(other),
            },
        ])),
    }
}

fn lower_multi_step_body(
    config: &ReduceLowering<'_>,
    index: usize,
    id: StepIdx,
    accumulator: SlotIdx,
    next: StepIdx,
    initial_type: ExprType,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    let mut types = std::collections::HashMap::new();
    types.insert(accumulator, initial_type);
    let mut outputs = std::collections::HashMap::new();
    outputs.insert(config.variable.to_owned(), accumulator);
    let scope = CanonicalScope {
        outputs: &outputs,
        step_outputs: config.scope.step_outputs,
        slot_types: &types,
        parent: Some(&config.scope),
    };
    let width = body_width(config.body, 0).map_err(|e| CompileErrors(vec![e]))?;
    let width = checked_width_n(width)?;
    let final_step =
        checked_step_offset(id, width, "reduce", "copy").map_err(|e| CompileErrors(vec![e]))?;
    let last = emit_scoped_typed_body(config.body, id, final_step, scope, builder, "reduce")?;
    let (result_slot, inferred) =
        last.ok_or(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "reduce.steps",
            expected: "at least one set or do step",
        }]))?;
    if inferred != initial_type && inferred != ExprType::Unknown {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "reduce.steps",
            expected: "a set value preserving the accumulator type",
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
#[path = "reduce_limit_tests.rs"]
mod limit_tests;
#[cfg(test)]
#[path = "canonical_reduce_tests.rs"]
mod tests;

pub(in crate::mod_compile_lowering) fn checked_width_n(width: usize) -> Result<u16, CompileErrors> {
    u16::try_from(width)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: width }]))
}
