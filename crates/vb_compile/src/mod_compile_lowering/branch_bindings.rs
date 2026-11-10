//! Per-body lexical bindings; only actual producers publish result slots.

use super::*;
use std::collections::{HashMap, HashSet};

#[path = "body_calls.rs"]
mod body_calls;
#[path = "canonical_nested.rs"]
mod nested;

#[derive(Clone, Copy)]
struct BodyTarget {
    id: StepIdx,
    next: StepIdx,
    slot: SlotIdx,
}

pub(super) struct BranchBindings {
    outputs: HashMap<String, SlotIdx>,
    steps: HashMap<String, SlotIdx>,
    types: HashMap<SlotIdx, ExprType>,
    local_outputs: HashSet<String>,
    local_steps: HashSet<String>,
    strict_unknown: bool,
}

impl BranchBindings {
    pub(super) fn new(scope: CanonicalScope<'_>, strict_unknown: bool) -> Self {
        Self {
            outputs: scope.outputs.clone(),
            steps: scope.step_outputs.clone(),
            types: scope.slot_types.clone(),
            local_outputs: HashSet::new(),
            local_steps: HashSet::new(),
            strict_unknown,
        }
    }

    pub(super) fn view(&self) -> CanonicalScope<'_> {
        CanonicalScope {
            outputs: &self.outputs,
            step_outputs: &self.steps,
            slot_types: &self.types,
        }
    }

    pub(super) fn type_of(&self, slot: SlotIdx) -> Option<ExprType> {
        self.types.get(&slot).copied()
    }

    pub(super) fn lower(
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
        if !matches!(
            step.primitive,
            crate::StepPrimitive::Set { .. } | crate::StepPrimitive::Do { .. }
        ) {
            return nested::lower_nested(step, id, next, self, names, builder);
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
}
