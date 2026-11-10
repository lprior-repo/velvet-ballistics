//! Scoped lexical bodies. Bindings never escape their branch or loop body.

use super::*;
use std::collections::{HashMap, HashSet};
use vb_core::{ActionId, SlotIdx};
use vb_expr::expr_typecheck::ExprType;

#[path = "canonical_nested.rs"]
mod nested;

pub(super) fn emit_scoped_branch_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<usize, CompileErrors> {
    let _last_output = emit_scoped_flat_body(body, start, common_next, scope, builder, "choose")?;
    body_width(body, 0).map_err(|e| CompileErrors(vec![e]))
}

/// Emit one node per flat Set/Do step and return the final body's value slot.
pub(in crate::mod_compile_lowering) fn emit_scoped_flat_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
    primitive: &'static str,
) -> Result<Option<SlotIdx>, CompileErrors> {
    emit_scoped_typed_body(body, start, common_next, scope, builder, primitive)
        .map(|result| result.map(|(slot, _)| slot))
}

/// Emit one node per flat Set/Do step and return (final slot, inferred type).
pub(in crate::mod_compile_lowering) fn emit_scoped_typed_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
    primitive: &'static str,
) -> Result<Option<(SlotIdx, ExprType)>, CompileErrors> {
    let mut local = BranchBindings::new(scope, primitive == "for_each");
    let layout = canonical_layout(body, start.as_usize()).map_err(|e| CompileErrors(vec![e]))?;
    let mut names = vec![Box::from(""); start.as_usize()];
    names.extend(canonical_step_names(body, &layout).map_err(|e| CompileErrors(vec![e]))?);
    let mut result = None;
    let mut offset = 0usize;
    for (index, step) in body.iter().enumerate() {
        reject_body_controls(step, index)?;
        let id = body_id(start, offset, primitive)?;
        offset = offset.checked_add(canonical_step_width_with_condition(step)
            .map_err(|e| CompileErrors(vec![e]))?).ok_or_else(|| index_error(offset))?;
        let next_index = index.checked_add(1).ok_or_else(|| index_error(index))?;
        let next = if next_index == body.len() {
            common_next
        } else {
            body_id(start, offset, primitive)?
        };
        let control = super::part_02::canonical_controls::prepare(step, id, Some(next), &names, local.view(), builder)?;
        let success = control.next.ok_or_else(|| index_error(id.as_usize()))?;
        result = local.lower(&control.step, control.body, success, &names, builder)?
            .map(|slot| (slot, local.type_of(slot).unwrap_or(ExprType::Unknown)));
        super::part_02::canonical_controls::complete(&control, builder);
    }
    Ok(result)
}



fn body_id(
    start: StepIdx,
    index: usize,
    primitive: &'static str,
) -> Result<StepIdx, CompileErrors> {
    let offset = u16::try_from(index).map_err(|_| index_error(index))?;
    checked_step_offset(start, offset, primitive, "body").map_err(|e| CompileErrors(vec![e]))
}

fn index_error(value: usize) -> CompileErrors {
    CompileErrors(vec![CompileError::StepIndexOutOfRange { value }])
}

#[derive(Clone, Copy)]
struct BodyTarget {
    id: StepIdx,
    next: StepIdx,
    slot: SlotIdx,
}

struct BranchBindings {
    outputs: HashMap<String, SlotIdx>,
    steps: HashMap<String, SlotIdx>,
    types: HashMap<SlotIdx, ExprType>,
    local_outputs: HashSet<String>,
    local_steps: HashSet<String>,
    strict_unknown: bool,
}

impl BranchBindings {
    fn new(scope: CanonicalScope<'_>, strict_unknown: bool) -> Self {
        Self {
            outputs: scope.outputs.clone(),
            steps: scope.step_outputs.clone(),
            types: scope.slot_types.clone(),
            local_outputs: HashSet::new(),
            local_steps: HashSet::new(),
            strict_unknown,
        }
    }

    fn view(&self) -> CanonicalScope<'_> {
        CanonicalScope {
            outputs: &self.outputs,
            step_outputs: &self.steps,
            slot_types: &self.types,
        }
    }

    fn type_of(&self, slot: SlotIdx) -> Option<ExprType> {
        self.types.get(&slot).copied()
    }

    fn lower(
        &mut self,
        step: &crate::StepAst,
        id: StepIdx,
        next: StepIdx,
        names: &[Box<str>],
        builder: &mut SlotCompiler,
    ) -> Result<Option<SlotIdx>, CompileErrors> {
        if !self.local_steps.insert(step.id.clone()) {
            return Err(CompileErrors(vec![CompileError::DuplicateStepId {
                id: step.id.clone().into(),
            }]));
        }
        if !matches!(step.primitive, crate::StepPrimitive::Set { .. } | crate::StepPrimitive::Do { .. }) {
            return nested::lower_nested(step, id, next, &mut self.outputs, &mut self.steps,
                &mut self.types, names, builder);
        }
        let slot = builder
            .allocate_slot()
            .map_err(|e| CompileErrors(vec![e]))?;
        let target = BodyTarget { id, next, slot };
        match &step.primitive {
            crate::StepPrimitive::Set { output, value } => {
                self.lower_set(output, value, target, builder)?
            }
            crate::StepPrimitive::Do { action, input } => {
                self.lower_do(action, input, target, builder)?
            }
            other => {
                return Err(CompileErrors(vec![
                    CompileError::UnsupportedStepPrimitive {
                        step: id.as_usize(),
                        primitive: canonical_primitive_name(other),
                    },
                ]));
            }
        }
        self.steps.insert(step.id.clone(), slot);
        Ok(Some(slot))
    }

    fn lower_set(
        &mut self,
        output: &str,
        value: &str,
        target: BodyTarget,
        builder: &mut SlotCompiler,
    ) -> Result<(), CompileErrors> {
        validate_output_alias(output).map_err(|e| CompileErrors(vec![e]))?;
        if !self.local_outputs.insert(output.to_owned()) {
            return Err(CompileErrors(vec![CompileError::DuplicateOutputName {
                name: output.into(),
            }]));
        }
        if self.strict_unknown {
            validate_for_each_expression(value, self.view())?;
        }
        let inferred = lower_canonical_set_expression(
            target.id,
            target.slot,
            value,
            Some(target.next),
            self.view(),
            builder,
        )?;
        self.outputs.insert(output.to_owned(), target.slot);
        self.types.insert(target.slot, inferred);
        Ok(())
    }

    fn lower_do(
        &self,
        action: &str,
        input: &str,
        target: BodyTarget,
        builder: &mut SlotCompiler,
    ) -> Result<(), CompileErrors> {
        let action = action.parse::<u16>().map_err(|_| {
            CompileErrors(vec![CompileError::StepFieldShape {
                step: target.id.as_usize(),
                field: "action",
                expected: "numeric action ID (u16)",
            }])
        })?;
        let input = resolve_slot_reference(input, &self.outputs, &self.steps)
            .map_err(|e| CompileErrors(vec![e]))?;
        let node = lower_do(
            target.id,
            ActionId::new(action),
            input,
            Some(target.slot),
            Some(target.next),
            builder,
        );
        builder.push_node(node);
        Ok(())
    }
}

fn reject_body_controls(step: &crate::StepAst, index: usize) -> Result<(), CompileErrors> {
    let fields = [
        (step.with.is_some(), "with"),
        (step.retry.is_some(), "try_again"),
    ];
    if let Some((_, field)) = fields.iter().find(|(present, _)| *present) {
        return Err(CompileErrors(vec![
            CompileError::UnsupportedStepControlField {
                step: index,
                field: Box::from(*field),
            },
        ]));
    }
    Ok(())
}
