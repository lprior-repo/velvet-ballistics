#![forbid(unsafe_code)]

use super::emit_single_body_set;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, checked_step_offset, lower_canonical_set_expression_with_binding,
    parse_i64_field, slot_from_text,
};
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
}

pub(in crate::mod_compile_lowering) fn lower_canonical_aggregate(
    index: usize,
    id: StepIdx,
    config: ReduceLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<(SlotIdx, ExprType), CompileErrors> {
    crate::mod_compile_validation::validate_public_name("reduce.variable", config.variable)
        .map_err(|e| CompileErrors(vec![e]))?;
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
    let initial = parse_i64_field(config.initial, index, "aggregate.initial")?;
    let initial = builder
        .push_constant(ConstValue::I64(initial))
        .map_err(|e| CompileErrors(vec![e]))?;
    let start =
        checked_step_offset(id, 1, "aggregate", "start").map_err(|e| CompileErrors(vec![e]))?;
    let body_step =
        checked_step_offset(id, 2, "aggregate", "body").map_err(|e| CompileErrors(vec![e]))?;
    let next_step =
        checked_step_offset(id, 3, "aggregate", "next").map_err(|e| CompileErrors(vec![e]))?;
    let done =
        checked_step_offset(id, 4, "aggregate", "done").map_err(|e| CompileErrors(vec![e]))?;
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
    let [step] = config.body else {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "steps",
            expected: "exactly one set step",
        }]));
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
