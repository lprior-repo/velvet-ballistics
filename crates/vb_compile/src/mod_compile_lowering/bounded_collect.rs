use super::super::part_14::emit_scoped_flat_body;
use super::super::{SlotCompiler, checked_step_offset, slot_from_text};
use super::{CanonicalScope, resolve_slot_reference, validate_output_alias};
use crate::mod_compile_validation::validate_public_name;
use crate::{CompileError, CompileErrors};
use vb_core::{CompiledNode, CompiledNodeKind, SlotIdx, StepIdx};
use vb_expr::expr_typecheck::ExprType;

pub(in crate::mod_compile_lowering) struct CollectLowering<'a> {
    pub(in crate::mod_compile_lowering) source: &'a str,
    pub(in crate::mod_compile_lowering) variable: &'a str,
    pub(in crate::mod_compile_lowering) pages: Option<u32>,
    pub(in crate::mod_compile_lowering) items: Option<u32>,
    pub(in crate::mod_compile_lowering) body: &'a [crate::StepAst],
    pub(in crate::mod_compile_lowering) next: Option<StepIdx>,
    pub(in crate::mod_compile_lowering) scope: CanonicalScope<'a>,
}

struct CollectPlan {
    source: SlotIdx,
    limit: u32,
    page_size: u32,
    body_step: StepIdx,
    advance: StepIdx,
    done: StepIdx,
    collector_slot: SlotIdx,
    finish_slot: SlotIdx,
}

fn bounds_error(index: usize) -> CompileErrors {
    CompileErrors(vec![CompileError::StepFieldShape {
        step: index,
        field: "collect",
        expected: "explicit positive pages or items limit",
    }])
}

fn resolve_source(index: usize, config: &CollectLowering<'_>) -> Result<SlotIdx, CompileErrors> {
    if config.source.starts_with('$') {
        resolve_slot_reference(
            config.source,
            config.scope.outputs,
            config.scope.step_outputs,
        )
        .map_err(|e| CompileErrors(vec![e]))
    } else {
        slot_from_text(config.source, index, "collect.source")
    }
}

fn validate_variable_name(variable: &str) -> Result<(), CompileErrors> {
    validate_public_name("collect.variable", variable).map_err(|e| CompileErrors(vec![e]))?;
    validate_output_alias(variable).map_err(|e| CompileErrors(vec![e]))?;
    if variable == "step" {
        return Err(CompileErrors(vec![CompileError::InvalidName {
            field: "collect.variable",
            value: variable.into(),
        }]));
    }
    Ok(())
}

fn validate_variable_capture(
    variable: &str,
    config: &CollectLowering<'_>,
) -> Result<(), CompileErrors> {
    if config.scope.outputs.contains_key(variable)
        || config.scope.step_outputs.contains_key(variable)
    {
        return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
            name: variable.into(),
        }]));
    }
    Ok(())
}

fn validate_body(index: usize, config: &CollectLowering<'_>) -> Result<(), CompileErrors> {
    if config.body.is_empty() {
        return Err(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "collect.body",
            expected: "non-empty body",
        }]));
    }
    for step in config.body {
        if step.id == config.variable {
            return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                name: config.variable.into(),
            }]));
        }
        if matches!(
            &step.primitive,
            crate::StepPrimitive::Set { output, .. } if output == config.variable
        ) {
            return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                name: config.variable.into(),
            }]));
        }
    }
    Ok(())
}

fn checked_body_offset(id: StepIdx, n: u16, label: &'static str) -> Result<StepIdx, CompileErrors> {
    checked_step_offset(id, n, "collect", label).map_err(|e| CompileErrors(vec![e]))
}

fn checked_body_length_offset(body_len: usize, extra: usize) -> Result<u16, CompileErrors> {
    let total = body_len.checked_add(extra).ok_or(CompileErrors(vec![
        CompileError::StepIndexOutOfRange { value: body_len },
    ]))?;
    u16::try_from(total)
        .map_err(|_| CompileErrors(vec![CompileError::StepIndexOutOfRange { value: total }]))
}

fn plan_compute_edges(
    id: StepIdx,
    body_len: usize,
) -> Result<(StepIdx, StepIdx, StepIdx), CompileErrors> {
    let body = checked_body_offset(id, 1, "body")?;
    let advance_n = checked_body_length_offset(body_len, 1)?;
    let advance = checked_body_offset(id, advance_n, "page")?;
    let done_n = checked_body_length_offset(body_len, 2)?;
    let done = checked_body_offset(id, done_n, "done")?;
    Ok((body, advance, done))
}

fn plan_validate_bounds(index: usize, config: &CollectLowering<'_>) -> Result<u32, CompileErrors> {
    if config.pages.is_none() && config.items.is_none() {
        return Err(bounds_error(index));
    }
    if config.pages == Some(0) || config.items == Some(0) {
        return Err(bounds_error(index));
    }
    let pages = config.pages.unwrap_or(1);
    let items = config.items.unwrap_or(1);
    pages
        .checked_mul(items)
        .ok_or(CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field: "collect",
            expected: "pages * items must fit u32",
        }]))
}

fn allocate_collect_slots(builder: &mut SlotCompiler) -> Result<(SlotIdx, SlotIdx), CompileErrors> {
    let collector_slot = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    let finish_slot = builder
        .allocate_slot()
        .map_err(|e| CompileErrors(vec![e]))?;
    Ok((collector_slot, finish_slot))
}

fn plan_build(
    index: usize,
    id: StepIdx,
    config: &CollectLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<CollectPlan, CompileErrors> {
    let source = resolve_source(index, config)?;
    builder.record_slot(source);
    let (body, advance, done) = plan_compute_edges(id, config.body.len())?;
    let limit = plan_validate_bounds(index, config)?;
    let (collector_slot, finish_slot) = allocate_collect_slots(builder)?;
    Ok(CollectPlan {
        source,
        limit,
        page_size: config.items.unwrap_or(1),
        body_step: body,
        advance,
        done,
        collector_slot,
        finish_slot,
    })
}

fn emit_start(id: StepIdx, plan: &CollectPlan, builder: &mut SlotCompiler) {
    builder.push_node(CompiledNode {
        id,
        output: Some(plan.collector_slot),
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::CollectStart {
            source: plan.source,
            limit: plan.limit,
            page_size: plan.page_size,
            body: plan.body_step,
            done: plan.done,
        },
    });
}

fn emit_body(
    config: &CollectLowering<'_>,
    plan: &CollectPlan,
    builder: &mut SlotCompiler,
) -> Result<(), CompileErrors> {
    let mut local_outputs = config.scope.outputs.clone();
    let mut local_types = config.scope.slot_types.clone();
    local_outputs.insert(config.variable.to_owned(), plan.collector_slot);
    local_types.insert(plan.collector_slot, ExprType::List);
    let local_scope = CanonicalScope {
        outputs: &local_outputs,
        step_outputs: config.scope.step_outputs,
        slot_types: &local_types,
    };
    let _body_output = emit_scoped_flat_body(
        config.body,
        plan.body_step,
        plan.advance,
        local_scope,
        builder,
        "collect",
    )?;
    Ok(())
}

fn emit_next(plan: &CollectPlan, builder: &mut SlotCompiler) {
    builder.push_node(CompiledNode {
        id: plan.advance,
        output: None,
        next: None,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::CollectNext {
            collector_slot: plan.collector_slot,
            body: plan.body_step,
            done: plan.done,
        },
    });
}

fn emit_finish(next: Option<StepIdx>, plan: &CollectPlan, builder: &mut SlotCompiler) {
    builder.push_node(CompiledNode {
        id: plan.done,
        output: Some(plan.finish_slot),
        next,
        error_slot: None,
        on_error: None,
        kind: CompiledNodeKind::CollectFinish {
            collector_slot: plan.collector_slot,
        },
    });
}

pub(in crate::mod_compile_lowering) fn lower_bounded_collect(
    index: usize,
    id: StepIdx,
    config: CollectLowering<'_>,
    builder: &mut SlotCompiler,
) -> Result<SlotIdx, CompileErrors> {
    validate_variable_name(config.variable)?;
    validate_variable_capture(config.variable, &config)?;
    validate_body(index, &config)?;

    let plan = plan_build(index, id, &config, builder)?;
    emit_start(id, &plan, builder);
    emit_body(&config, &plan, builder)?;
    emit_next(&plan, builder);
    emit_finish(config.next, &plan, builder);

    Ok(plan.finish_slot)
}

#[cfg(test)]
#[path = "bounded_collect_offset_tests.rs"]
mod offset_tests;
