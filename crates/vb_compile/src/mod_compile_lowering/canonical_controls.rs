//! Shared control edges around a complete authored step.

use super::*;
use vb_core::{
    CompiledNodeKind, ExprBranch,
    workflow::{ErrorRouting, node},
};
use vb_expr::expr_typecheck::ExprType;

pub(in crate::mod_compile_lowering) struct PreparedControl {
    pub step: crate::StepAst,
    pub body: StepIdx,
    pub next: Option<StepIdx>,
    pub handler: Option<StepIdx>,
    pub first_node: usize,
    pub retry: Option<vb_core::workflow::CompiledRetryPolicy>,
}

pub(in crate::mod_compile_lowering) fn prepare(
    step: &crate::StepAst,
    id: StepIdx,
    next: Option<StepIdx>,
    names: &[Box<str>],
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<PreparedControl, CompileErrors> {
    let first_node = builder.nodes.len();
    let retry = retry_policy(step, id)?;
    let success = step
        .then
        .as_deref()
        .map(|label| target(label, id, names))
        .transpose()?
        .or(next);
    let handler = step
        .on_error
        .as_ref()
        .map(|error| target(&error.handler, id, names))
        .transpose()?;
    let body = if let Some(condition) = &step.condition {
        let (condition, inferred) = compile_canonical_expression(condition, scope, builder)?;
        if inferred != ExprType::Bool {
            return Err(CompileErrors(vec![CompileError::TypeMismatch {
                field: "if",
                expected: "boolean",
                found: "non-boolean",
            }]));
        }
        let body = checked_step_offset(id, 1, "if", "body").map_err(|e| CompileErrors(vec![e]))?;
        builder.push_node(node(
            id,
            CompiledNodeKind::Choose {
                branches: Box::from([ExprBranch {
                    condition,
                    target: body,
                }]),
                otherwise: next.into(),
            },
            None,
            None,
        ));
        body
    } else {
        id
    };
    let mut plain = step.clone();
    let body = if let Some(value) = &step.with {
        let (input, _) = super::canonical_values::emit(value, true, scope, builder)?;
        if let crate::StepPrimitive::Do {
            input: reference, ..
        } = &mut plain.primitive
        {
            *reference = input.get().to_string();
        } else {
            return Err(CompileErrors(vec![CompileError::StepFieldShape {
                step: id.as_usize(),
                field: "with",
                expected: "an action step",
            }]));
        }
        step_idx(builder.nodes.len()).map_err(|e| CompileErrors(vec![e]))?
    } else {
        body
    };
    plain.with = None;
    plain.condition = None;
    plain.then = None;
    plain.on_error = None;
    plain.retry = None;
    Ok(PreparedControl {
        step: plain,
        body,
        next: success,
        handler,
        first_node,
        retry,
    })
}

pub(in crate::mod_compile_lowering) fn complete(
    control: &PreparedControl,
    builder: &mut SlotCompiler,
) {
    for node in builder.nodes.iter_mut().skip(control.first_node) {
        if node.id == control.body && matches!(node.kind, CompiledNodeKind::Do { .. }) {
            node.retry = control.retry;
        }
    }
    for node in builder.nodes.iter().skip(control.first_node) {
        builder.display_names.entry(node.id).or_insert_with(|| {
            control
                .step
                .name
                .as_deref()
                .unwrap_or(&control.step.id)
                .into()
        });
    }
    if let Some(handler) = control.handler {
        for node in builder.nodes.iter_mut().skip(control.first_node) {
            if matches!(node.routing.error, ErrorRouting::None) {
                node.routing.error = ErrorRouting::Handler {
                    handler,
                    error_slot: None,
                };
            }
        }
    }
}

fn retry_policy(
    step: &crate::StepAst,
    id: StepIdx,
) -> Result<Option<vb_core::workflow::CompiledRetryPolicy>, CompileErrors> {
    let Some(retry) = &step.retry else {
        return Ok(None);
    };
    let invalid = || {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: id.as_usize(),
            field: "try_again",
            expected: "an action with 1..=3 total attempts and a bounded constant delay (ms, s, m, h, d)",
        }])
    };
    if !matches!(step.primitive, crate::StepPrimitive::Do { .. })
        || retry.max_attempts == 0
        || retry.max_attempts > vb_core::ResourceContract::DEFAULT.max_retry_attempts
    {
        return Err(invalid());
    }
    let delay_ms = retry
        .delay
        .as_deref()
        .map(delay_millis)
        .transpose()
        .map_err(|()| invalid())?
        .unwrap_or(0);
    Ok(Some(vb_core::workflow::CompiledRetryPolicy {
        max_attempts: retry.max_attempts,
        delay_ms,
    }))
}

fn delay_millis(text: &str) -> Result<u64, ()> {
    let text = text.trim();
    let end = text
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(text.len());
    let amount = text.get(..end).ok_or(())?.parse::<u64>().map_err(|_| ())?;
    let unit = match text.get(end..).ok_or(())?.trim() {
        "" | "ms" => 1,
        "s" => 1_000,
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => return Err(()),
    };
    amount
        .checked_mul(unit)
        .filter(|value| *value <= vb_core::workflow::CompiledRetryPolicy::MAX_DELAY_MS)
        .ok_or(())
}

fn target(label: &str, id: StepIdx, names: &[Box<str>]) -> Result<StepIdx, CompileErrors> {
    let index = names
        .iter()
        .position(|name| name.as_ref() == label)
        .ok_or_else(|| {
            CompileErrors(vec![CompileError::UnknownStepLabel {
                step: id.as_usize(),
                label: label.into(),
            }])
        })?;
    let target = u16::try_from(index)
        .map(StepIdx::new)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: index }]))?;
    if target <= id {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: id.as_usize(),
            field: "control target",
            expected: "a later step in the same lexical body",
        }]));
    }
    Ok(target)
}
