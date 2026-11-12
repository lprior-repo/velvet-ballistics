#![forbid(unsafe_code)]

use super::super::emit_single_body_set;
use super::ReduceEdges;
use super::ReduceLowering;
use super::field_error;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::part_14::emit_scoped_flat_body;
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, lower_canonical_set_expression_with_binding,
};
use std::collections::HashMap;
use vb_core::{SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;
pub(super) fn lower_multi_body(
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
                primitive: crate::mod_compile_lowering::canonical_primitive_name(&last.primitive),
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

pub(super) fn lower_body(
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
