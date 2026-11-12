#![forbid(unsafe_code)]

use super::ReduceLowering;
use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::part_05::canonical_primitive_name;
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, body_width, checked_step_offset, checked_width_n,
    emit_scoped_typed_body, lower_canonical_set_expression_with_binding, reduce_direct_body,
};
use vb_core::workflow::node;
use vb_core::{CompiledNodeKind, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

pub(super) fn lower_body(
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
        other => {
            let name = canonical_primitive_name(other).map_err(|e| CompileErrors(vec![e]))?;
            Err(CompileErrors(vec![
                CompileError::UnsupportedStepPrimitive {
                    step: index,
                    primitive: name,
                },
            ]))
        }
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
