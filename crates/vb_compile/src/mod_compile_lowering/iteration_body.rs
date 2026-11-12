use super::*;
use vb_core::{ConstValue, SlotIdx, StepIdx, workflow::node};

pub(in crate::mod_compile_lowering) fn produces_value(step: &crate::StepAst) -> bool {
    matches!(
        step.primitive,
        crate::StepPrimitive::Set { .. }
            | crate::StepPrimitive::Do { .. }
            | crate::StepPrimitive::Ask { .. }
            | crate::StepPrimitive::ForEach { .. }
            | crate::StepPrimitive::Collect { .. }
            | crate::StepPrimitive::Aggregate { .. }
            | crate::StepPrimitive::Repeat { .. }
            | crate::StepPrimitive::Together { .. }
    )
}

pub(in crate::mod_compile_lowering) fn needs_tracking(body: &[crate::StepAst]) -> bool {
    let Some(last) = body.iter().rposition(produces_value) else {
        return false;
    };
    let skips_last = |target: &str| {
        body.iter()
            .position(|step| step.id == target)
            .is_some_and(|target| target > last)
    };
    if body.get(last).is_some_and(|step| {
        step.condition.is_some()
            || step
                .on_error
                .as_ref()
                .is_some_and(|error| skips_last(&error.handler))
    }) {
        return true;
    }
    body.iter().take(last).any(|step| {
        step.then.as_deref().is_some_and(&skips_last)
            || step.on_error.as_ref().is_some_and(|error| skips_last(&error.handler))
            || matches!(&step.primitive, crate::StepPrimitive::Choose { otherwise: Some(target), .. } if skips_last(target))
    })
}

pub(in crate::mod_compile_lowering) fn iteration_body_width(
    body: &[crate::StepAst],
) -> Result<usize, CompileError> {
    let mut width = body_width(body, usize::from(needs_tracking(body)))?;
    if needs_tracking(body) {
        for _ in body.iter().filter(|step| produces_value(step)) {
            width = width
                .checked_add(1)
                .ok_or(CompileError::StepIndexOutOfRange { value: width })?;
        }
    }
    Ok(width)
}

pub(in crate::mod_compile_lowering) fn emit_iteration_body(
    body: &[crate::StepAst],
    start: StepIdx,
    next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<Option<SlotIdx>, CompileErrors> {
    if !needs_tracking(body) {
        return canonical_branch_body::emit_scoped_flat_body(
            body, start, next, scope, builder, "for_each",
        );
    }
    let result = builder.allocate_slot()?;
    let null = builder.push_constant(ConstValue::Null)?;
    let first = checked_step_offset(start, 1, "for_each", "body initialization")?;
    builder.push_node(node(
        start,
        CompiledNodeKind::SetConst { value: null },
        Some(result),
        Some(first),
    ));
    canonical_branch_body::emit_typed_body(
        body,
        first,
        next,
        scope,
        builder,
        "for_each",
        Some(result),
    )?;
    Ok(Some(result))
}
