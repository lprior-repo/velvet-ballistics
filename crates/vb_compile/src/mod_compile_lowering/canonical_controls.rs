//! Shared control edges around a complete authored step.

use super::*;
use vb_core::{CompiledNodeKind, ExprBranch, workflow::{node, ErrorRouting}};
use vb_expr::expr_typecheck::ExprType;

pub(in crate::mod_compile_lowering) struct PreparedControl {
    pub step: crate::StepAst,
    pub body: StepIdx,
    pub next: Option<StepIdx>,
    pub handler: Option<StepIdx>,
    pub first_node: usize,
}

pub(in crate::mod_compile_lowering) fn prepare(
    step: &crate::StepAst,
    id: StepIdx,
    next: Option<StepIdx>,
    names: &[Box<str>],
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<PreparedControl, CompileErrors> {
    let success = step.then.as_deref().map(|label| target(label, id, names)).transpose()?.or(next);
    let handler = step.on_error.as_ref().map(|error| target(&error.handler, id, names)).transpose()?;
    let body = if let Some(condition) = &step.condition {
        let (condition, inferred) = compile_canonical_expression(condition, scope, builder)?;
        if inferred != ExprType::Bool {
            return Err(CompileErrors(vec![CompileError::TypeMismatch {
                field: "if", expected: "boolean", found: "non-boolean",
            }]));
        }
        let body = checked_step_offset(id, 1, "if", "body").map_err(|e| CompileErrors(vec![e]))?;
        builder.push_node(node(id, CompiledNodeKind::Choose {
            branches: Box::from([ExprBranch { condition, target: body }]), otherwise: next,
        }, None, None));
        body
    } else { id };
    let mut plain = step.clone();
    plain.condition = None;
    plain.then = None;
    plain.on_error = None;
    Ok(PreparedControl { step: plain, body, next: success, handler, first_node: builder.nodes.len() })
}

pub(in crate::mod_compile_lowering) fn complete(control: &PreparedControl, builder: &mut SlotCompiler) {
    if let Some(handler) = control.handler {
        for node in builder.nodes.iter_mut().skip(control.first_node) {
            if matches!(node.routing.error, ErrorRouting::None) {
                node.routing.error = ErrorRouting::Handler { handler, error_slot: None };
            }
        }
    }
}

fn target(label: &str, id: StepIdx, names: &[Box<str>]) -> Result<StepIdx, CompileErrors> {
    let index = names.iter().position(|name| name.as_ref() == label).ok_or_else(||
        CompileErrors(vec![CompileError::UnknownStepLabel { step: id.as_usize(), label: label.into() }]))?;
    let target = u16::try_from(index).map(StepIdx::new).map_err(|_| CompileErrors(vec![
        CompileError::StepIndexOutOfRange { value: index }]))?;
    if target <= id {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: id.as_usize(), field: "control target", expected: "a later step in the same lexical body",
        }]));
    }
    Ok(target)
}
