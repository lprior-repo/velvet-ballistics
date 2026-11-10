//! Per-body lexical bindings with recursive container dispatch.
//!
//! `BranchBindings` owns one body level's scope maps. Leaves bind here;
//! containers recurse through their top-level lowering entry points and
//! thread their inner body's result slot back as the step value.

use super::super::part_02::{
    CollectLowering, CollectOutput, ForEachLowering, lower_bounded_collect, lower_bounded_for_each,
};
use super::*;
use std::collections::{HashMap, HashSet};
use vb_core::SlotIdx;
use vb_expr::expr_typecheck::ExprType;

#[path = "body_calls.rs"]
mod body_calls;

#[derive(Clone, Copy)]
struct BodyTarget {
    id: StepIdx,
    next: StepIdx,
    slot: SlotIdx,
}

pub(super) struct BranchBindings<'a> {
    outputs: HashMap<String, SlotIdx>,
    steps: HashMap<String, SlotIdx>,
    types: HashMap<SlotIdx, ExprType>,
    local_outputs: HashSet<String>,
    local_steps: HashSet<String>,
    strict_unknown: bool,
    ctx: BodyCtx<'a>,
}

impl<'a> BranchBindings<'a> {
    pub(super) fn new(scope: CanonicalScope<'_>, strict_unknown: bool, ctx: BodyCtx<'a>) -> Self {
        Self {
            outputs: scope.outputs.clone(),
            steps: scope.step_outputs.clone(),
            types: scope.slot_types.clone(),
            local_outputs: HashSet::new(),
            local_steps: HashSet::new(),
            strict_unknown,
            ctx,
        }
    }

    fn view(&self) -> CanonicalScope<'_> {
        CanonicalScope {
            outputs: &self.outputs,
            step_outputs: &self.steps,
            slot_types: &self.types,
        }
    }

    pub(super) fn lower(
        &mut self,
        step: &crate::StepAst,
        id: StepIdx,
        next: StepIdx,
        builder: &mut SlotCompiler,
    ) -> Result<SlotIdx, CompileErrors> {
        if !self.local_steps.insert(step.id.clone()) {
            return Err(CompileErrors(vec![CompileError::DuplicateStepId {
                id: step.id.clone().into(),
            }]));
        }
        // Only leaves allocate a value slot up front; containers thread
        // their inner body's result slot instead, so no dead slots remain.
        let value = match &step.primitive {
            crate::StepPrimitive::Set { output, value } => {
                let slot = alloc_body_slot(builder)?;
                self.lower_set(output, value, BodyTarget { id, next, slot }, builder)?;
                slot
            }
            crate::StepPrimitive::Do { action, input } => {
                let slot = alloc_body_slot(builder)?;
                self.lower_do(action, input, BodyTarget { id, next, slot }, builder)?;
                slot
            }
            crate::StepPrimitive::ForEach { .. }
            | crate::StepPrimitive::Collect { .. }
            | crate::StepPrimitive::Repeat { .. }
            | crate::StepPrimitive::Choose { .. } => {
                self.lower_nested_container(&step.primitive, id, next, builder)?
            }
            other => {
                return Err(CompileErrors(vec![
                    CompileError::UnsupportedStepPrimitive {
                        step: id.as_usize(),
                        primitive: canonical_primitive_name(other),
                    },
                ]));
            }
        };
        self.steps.insert(step.id.clone(), value);
        Ok(value)
    }

    /// Lowers one nested container through the same entry point its
    /// top-level form uses, so layout widths and emitted nodes agree.
    fn lower_nested_container(
        &mut self,
        primitive: &crate::StepPrimitive,
        id: StepIdx,
        next: StepIdx,
        builder: &mut SlotCompiler,
    ) -> Result<SlotIdx, CompileErrors> {
        match primitive {
            crate::StepPrimitive::ForEach {
                input,
                variable,
                at_once,
                body,
            } => {
                let output = lower_bounded_for_each(
                    self.ctx.index,
                    id,
                    ForEachLowering {
                        input,
                        variable,
                        at_once: *at_once,
                        body,
                        next: Some(next),
                        scope: self.view(),
                        inputs: self.ctx.inputs,
                        nested: self.ctx,
                    },
                    builder,
                )?;
                output.ok_or_else(|| Self::empty_body_error(self.ctx.index, "for_each.body"))
            }
            crate::StepPrimitive::Collect {
                source,
                variable,
                pages,
                items,
                body,
                ..
            } => {
                let output: CollectOutput = lower_bounded_collect(
                    self.ctx.index,
                    id,
                    CollectLowering {
                        source,
                        variable,
                        pages: *pages,
                        items: *items,
                        body,
                        next: Some(next),
                        scope: self.view(),
                        nested: self.ctx,
                    },
                    builder,
                )?;
                output
                    .body
                    .ok_or_else(|| Self::empty_body_error(self.ctx.index, "collect.body"))
            }
            crate::StepPrimitive::Repeat { max_attempts, body } => {
                self.lower_nested_repeat(*max_attempts, body, id, next, builder)
            }
            crate::StepPrimitive::Choose {
                branches,
                otherwise,
            } => {
                self.lower_nested_choose(branches, otherwise.as_deref(), id, next, builder)?;
                // A choose binds no value; the marker slot stays unwritten
                // and reads of it fail closed downstream.
                alloc_body_slot(builder)
            }
            other => Err(CompileErrors(vec![
                CompileError::UnsupportedStepPrimitive {
                    step: id.as_usize(),
                    primitive: canonical_primitive_name(other),
                },
            ])),
        }
    }

    fn empty_body_error(index: usize, field: &'static str) -> CompileErrors {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field,
            expected: "non-empty body",
        }])
    }
}
