#![forbid(unsafe_code)]

use super::super::canonical_reduce_initial::accumulator_type;
use super::super::field_error;
use super::ReduceLowering;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, body_width, checked_step_offset, checked_width_n,
    emit_scoped_typed_body, lower_canonical_set_expression_with_binding, reduce_direct_body,
};
use std::collections::HashMap;
use vb_core::workflow::node;
use vb_core::{CompiledNodeKind, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

struct Body<'a, 's> {
    config: &'a ReduceLowering<'s>,
    index: usize,
    id: StepIdx,
    accumulator: SlotIdx,
    next: StepIdx,
    initial_type: ExprType,
}

struct Overlay {
    outputs: HashMap<String, SlotIdx>,
    types: HashMap<SlotIdx, ExprType>,
}

impl Overlay {
    fn new(state: &Body<'_, '_>) -> Self {
        let mut outputs = HashMap::with_capacity(1);
        outputs.insert(state.config.variable.to_owned(), state.accumulator);
        let mut types = HashMap::with_capacity(1);
        types.insert(state.accumulator, state.initial_type);
        Self { outputs, types }
    }

    fn scope<'a>(&'a self, parent: &'a CanonicalScope<'a>) -> CanonicalScope<'a> {
        CanonicalScope {
            outputs: &self.outputs,
            step_outputs: parent.step_outputs,
            slot_types: &self.types,
            parent: Some(parent),
        }
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
    let state = Body {
        config,
        index,
        id,
        accumulator,
        next,
        initial_type,
    };
    if reduce_direct_body(config.body) {
        lower_single(&state, builder)
    } else {
        lower_multi(&state, builder)
    }
}

fn lower_single(
    state: &Body<'_, '_>,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    let value = direct_value(state.config.body, state.index)?;
    let overlay = Overlay::new(state);
    let inferred = lower_canonical_set_expression_with_binding(
        state.id,
        state.accumulator,
        value,
        Some(state.next),
        overlay.scope(&state.config.scope),
        state.config.variable,
        builder,
    )?;
    accumulator_type(state.index, inferred, state.initial_type, state.config)
}

fn direct_value(steps: &[crate::StepAst], index: usize) -> Result<&str, CompileErrors> {
    let [step] = steps else {
        return Err(field_error(index, "steps", "exactly one set step"));
    };
    match &step.primitive {
        crate::StepPrimitive::Set { value, .. } => Ok(value),
        other => Err(CompileErrors(vec![
            CompileError::UnsupportedStepPrimitive {
                step: index,
                primitive: crate::mod_compile_lowering::canonical_primitive_name(other)?,
            },
        ])),
    }
}

fn lower_multi(
    state: &Body<'_, '_>,
    builder: &mut SlotCompiler,
) -> Result<ExprType, CompileErrors> {
    let width = checked_width_n(body_width(state.config.body, 0)?)?;
    let copy = checked_step_offset(state.id, width, "reduce", "copy")?;
    let result = emit_multi(state, copy, builder)?;
    let (slot, inferred) = result
        .ok_or_else(|| field_error(state.index, "reduce.steps", "at least one set or do step"))?;
    let inferred = validate_result(state, inferred)?;
    emit_copy(state, copy, slot, builder);
    Ok(inferred)
}

fn emit_multi(
    state: &Body<'_, '_>,
    copy: StepIdx,
    builder: &mut SlotCompiler,
) -> Result<Option<(SlotIdx, ExprType)>, CompileErrors> {
    let overlay = Overlay::new(state);
    emit_scoped_typed_body(
        state.config.body,
        state.id,
        copy,
        overlay.scope(&state.config.scope),
        builder,
        "reduce",
    )
}

fn validate_result(state: &Body<'_, '_>, inferred: ExprType) -> Result<ExprType, CompileErrors> {
    if inferred != state.initial_type && inferred != ExprType::Unknown {
        return Err(field_error(
            state.index,
            "reduce.steps",
            "a set value preserving the accumulator type",
        ));
    }
    accumulator_type(state.index, inferred, state.initial_type, state.config)
}

fn emit_copy(state: &Body<'_, '_>, copy: StepIdx, slot: SlotIdx, builder: &mut SlotCompiler) {
    builder.push_node(node(
        copy,
        CompiledNodeKind::Copy { source: slot },
        Some(state.accumulator),
        Some(state.next),
    ));
}
