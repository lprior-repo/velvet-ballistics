//! Bounded Repeat bodies with lexical bindings.

use crate::mod_compile_errors::{CompileError, CompileErrors};
use crate::mod_compile_lowering::part_14::{emit_body_result, emit_scoped_typed_body};
use crate::mod_compile_lowering::{
    CanonicalScope, SlotCompiler, body_width, checked_step_offset, repeat_body_uses_attempt_number,
};
use std::collections::HashMap;
use vb_core::workflow::{NodeContinuation, NodeOutput, node};
use vb_core::{
    CompiledNode, CompiledNodeKind, ConstValue, ExprIdx, ExprOp, ExprProgram, SlotIdx, StepIdx,
    WorkflowError,
};
use vb_expr::expr_typecheck::ExprType;

pub(in crate::mod_compile_lowering) fn lower_canonical_repeat(
    index: usize,
    id: StepIdx,
    max_attempts: u16,
    body: &[crate::StepAst],
    next: Option<StepIdx>,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<(SlotIdx, ExprType), CompileErrors> {
    if max_attempts == 0 {
        return Err(field_error(
            index,
            "repeat.max_attempts",
            "a value of at least 1 (got 0)",
        ));
    }
    if body.is_empty() {
        return Err(field_error(index, "steps", "non-empty body"));
    }
    let binds_attempt = repeat_body_uses_attempt_number(body);
    let layout = RepeatLayout::new(id, body, binds_attempt).map_err(|e| CompileErrors(vec![e]))?;
    let attempt_slot = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let attempt_binding = binds_attempt
        .then(|| {
            builder
                .allocate_slot()
                .map(|slot| (slot, attempt_slot))
                .map_err(|error| CompileErrors(vec![error]))
        })
        .transpose()?
        .map(|(slot, counter)| {
            repeat_attempt_expression(counter, max_attempts, builder)
                .map(|expression| (slot, expression))
        })
        .transpose()?;
    builder.push_node(node(
        id,
        CompiledNodeKind::RepeatStart {
            max_attempts: max_attempts.into(),
            body: layout.entry.unwrap_or(layout.body),
            done: layout.done,
        },
        Some(attempt_slot),
        None,
    ));
    if let Some((attempt_number, expression)) = attempt_binding {
        let entry = layout.entry.ok_or_else(|| {
            CompileErrors(vec![CompileError::ExpressionLoweringUnsupported {
                feature: "repeat attempt binding layout".into(),
            }])
        })?;
        builder.push_node(node(
            entry,
            CompiledNodeKind::EvalExpr { expr: expression },
            Some(attempt_number),
            Some(layout.body),
        ));
    }
    let attempt_outputs = attempt_binding
        .map(|(slot, _)| HashMap::from([(String::from("attempt.number"), slot)]))
        .unwrap_or_default();
    let attempt_steps = HashMap::new();
    let attempt_types = attempt_binding
        .map(|(slot, _)| HashMap::from([(slot, ExprType::I64)]))
        .unwrap_or_default();
    let attempt_scope = if binds_attempt {
        CanonicalScope {
            outputs: &attempt_outputs,
            step_outputs: &attempt_steps,
            slot_types: &attempt_types,
            parent: Some(&scope),
        }
    } else {
        scope
    };
    let last = emit_scoped_typed_body(
        body,
        layout.body,
        layout.result,
        attempt_scope,
        builder,
        "repeat",
    )?;
    let result = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    emit_body_result(
        layout.result,
        last.map(|value| value.0),
        result,
        layout.check,
        builder,
    )?;
    push_repeat_continuations(layout, attempt_slot, result, next, builder);
    Ok((result, last.map_or(ExprType::Null, |value| value.1)))
}

fn repeat_attempt_expression(
    attempt_slot: SlotIdx,
    max_attempts: u16,
    builder: &mut SlotCompiler,
) -> Result<ExprIdx, CompileErrors> {
    let packed_limit =
        vb_core::engine::repeat::encode_repeat_state(max_attempts, 0).map_err(|error| {
            CompileErrors(vec![CompileError::ExpressionLoweringUnsupported {
                feature: format!("repeat attempt state: {error}").into(),
            }])
        })?;
    let limit = builder
        .push_constant(ConstValue::I64(packed_limit))
        .map_err(|error| CompileErrors(vec![error]))?;
    let one = builder
        .push_constant(ConstValue::I64(1))
        .map_err(|error| CompileErrors(vec![error]))?;
    let program = ExprProgram::try_from_ops(Box::from([
        ExprOp::LoadSlot(attempt_slot),
        ExprOp::LoadConst(limit),
        ExprOp::Sub,
        ExprOp::LoadConst(one),
        ExprOp::Add,
    ]))
    .map_err(|error| {
        CompileErrors(vec![CompileError::Workflow(WorkflowError::Expression(
            error,
        ))])
    })?;
    builder
        .push_expression(program)
        .map_err(|error| CompileErrors(vec![error]))
}

struct RepeatLayout {
    entry: Option<StepIdx>,
    body: StepIdx,
    result: StepIdx,
    check: StepIdx,
    done: StepIdx,
}

impl RepeatLayout {
    fn new(
        id: StepIdx,
        steps: &[crate::StepAst],
        binds_attempt: bool,
    ) -> Result<Self, CompileError> {
        let width = body_width(steps, 0)?;
        let width =
            u16::try_from(width).map_err(|_| CompileError::StepIndexOutOfRange { value: width })?;
        let entry = binds_attempt
            .then(|| checked_step_offset(id, 1, "repeat", "attempt"))
            .transpose()?;
        let body = checked_step_offset(id, if binds_attempt { 2 } else { 1 }, "repeat", "body")?;
        let result = checked_step_offset(body, width, "repeat", "result")?;
        let check = checked_step_offset(result, 1, "repeat", "check")?;
        let done = checked_step_offset(check, 1, "repeat", "done")?;
        Ok(Self {
            entry,
            body,
            result,
            check,
            done,
        })
    }
}

fn push_repeat_continuations(
    layout: RepeatLayout,
    attempt_slot: SlotIdx,
    result: SlotIdx,
    next: Option<StepIdx>,
    builder: &mut SlotCompiler,
) {
    builder.push_node(node(
        layout.check,
        CompiledNodeKind::RepeatCheck {
            attempt_slot,
            done: layout.done,
        },
        Some(attempt_slot),
        Some(layout.entry.unwrap_or(layout.body)),
    ));
    builder.push_node(node(
        layout.done,
        CompiledNodeKind::RepeatFinish { result },
        Some(result),
        next,
    ));
}

fn field_error(step: usize, field: &'static str, expected: &'static str) -> CompileErrors {
    CompileErrors(vec![CompileError::StepFieldShape {
        step,
        field,
        expected,
    }])
}

#[cfg(test)]
#[path = "canonical_repeat_tests.rs"]
mod tests;
