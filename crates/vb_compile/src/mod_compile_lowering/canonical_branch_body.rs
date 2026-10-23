//! Flat branch-local lexical bodies. Bindings never escape the selected branch.

use super::*;
use std::collections::{HashMap, HashSet};
use vb_core::{ActionId, SlotIdx};
use vb_expr::expr_typecheck::ExprType;

pub(super) fn emit_scoped_branch_body(
    body: &[crate::StepAst],
    start: StepIdx,
    common_next: StepIdx,
    scope: CanonicalScope<'_>,
    builder: &mut SlotCompiler,
) -> Result<usize, CompileErrors> {
    let mut local = BranchBindings::new(scope);
    for (index, step) in body.iter().enumerate() {
        reject_body_controls(step, index)?;
        let id = body_id(start, index)?;
        let next_index = index.checked_add(1).ok_or_else(|| index_error(index))?;
        let next = if next_index == body.len() {
            common_next
        } else {
            body_id(start, next_index)?
        };
        local.lower(step, id, next, builder)?;
    }
    Ok(body.len())
}

fn body_id(start: StepIdx, index: usize) -> Result<StepIdx, CompileErrors> {
    let offset = u16::try_from(index).map_err(|_| index_error(index))?;
    checked_step_offset(start, offset, "choose", "body").map_err(|e| CompileErrors(vec![e]))
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
}

impl BranchBindings {
    fn new(scope: CanonicalScope<'_>) -> Self {
        Self {
            outputs: scope.outputs.clone(),
            steps: scope.step_outputs.clone(),
            types: scope.slot_types.clone(),
            local_outputs: HashSet::new(),
            local_steps: HashSet::new(),
        }
    }

    fn view(&self) -> CanonicalScope<'_> {
        CanonicalScope {
            outputs: &self.outputs,
            step_outputs: &self.steps,
            slot_types: &self.types,
        }
    }

    fn lower(
        &mut self,
        step: &crate::StepAst,
        id: StepIdx,
        next: StepIdx,
        builder: &mut SlotCompiler,
    ) -> Result<(), CompileErrors> {
        if !self.local_steps.insert(step.id.clone()) {
            return Err(CompileErrors(vec![CompileError::DuplicateStepId {
                id: step.id.clone().into(),
            }]));
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
        Ok(())
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
        (step.name.is_some(), "name"),
        (step.condition.is_some(), "if"),
        (step.with.is_some(), "with"),
        (step.retry.is_some(), "try_again"),
        (step.on_error.is_some(), "on_error"),
        (step.then.is_some(), "then"),
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
