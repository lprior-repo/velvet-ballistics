//! Per-body lexical bindings with recursive container dispatch.
//!
//! `BranchBindings` owns one body level's overlay maps only — the parent
//! `CanonicalScope` is held by reference and never cloned wholesale. Inserts
//! go into small per-level HashMaps (typically 0–4 entries) and lookups walk
//! overlay → parent chain via borrowed scopes. This kills the three
//! `HashMap::clone()`s per body level that were previously waivered for
//! performance.

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
    /// Overlay for outputs bound inside this body. Parent outputs are *not*
    /// copied; lookups fall through to `parent_scope`.
    outputs: HashMap<String, SlotIdx>,
    /// Overlay for step-id → slot bindings inside this body.
    steps: HashMap<String, SlotIdx>,
    /// Overlay for slot → type bindings inside this body.
    types: HashMap<SlotIdx, ExprType>,
    /// Borrowed parent lexical environment (never cloned).
    parent_scope: CanonicalScope<'a>,
    local_outputs: HashSet<String>,
    local_steps: HashSet<String>,
    strict_unknown: bool,
    ctx: BodyCtx<'a>,
}

impl<'a> BranchBindings<'a> {
    pub(super) fn new(scope: CanonicalScope<'a>, strict_unknown: bool, ctx: BodyCtx<'a>) -> Self {
        Self {
            outputs: HashMap::new(),
            steps: HashMap::new(),
            types: HashMap::new(),
            parent_scope: scope,
            local_outputs: HashSet::new(),
            local_steps: HashSet::new(),
            strict_unknown,
            ctx,
        }
    }

    fn view(&self) -> CanonicalScope<'_> {
        // Borrowed chained view: local overlay points to our tiny maps,
        // parent points to the borrowed outer scope. No whole-map clone.
        // We leak a reference chain via `parent: Some(&self.parent_scope)`
        // where `parent_scope` is stored by value; the returned scope's
        // lifetime is tied to `&self`.
        CanonicalScope {
            outputs: &self.outputs,
            step_outputs: &self.steps,
            slot_types: &self.types,
            parent: Some(&self.parent_scope),
        }
    }

    #[inline]
    #[allow(dead_code)]
    pub(super) fn get_output(&self, name: &str) -> Option<SlotIdx> {
        self.outputs
            .get(name)
            .copied()
            .or_else(|| self.parent_scope.get_output(name))
    }

    #[inline]
    #[allow(dead_code)]
    pub(super) fn get_step(&self, name: &str) -> Option<SlotIdx> {
        self.steps
            .get(name)
            .copied()
            .or_else(|| self.parent_scope.get_step(name))
    }

    pub(super) fn lower(
        &mut self,
        step: &crate::StepAst,
        id: StepIdx,
        next: StepIdx,
        builder: &mut SlotCompiler,
        target: Option<SlotIdx>,
    ) -> Result<(SlotIdx, ExprType), CompileErrors> {
        if !self.local_steps.insert(step.id.clone()) {
            return Err(CompileErrors(vec![CompileError::DuplicateStepId {
                id: step.id.clone().into(),
            }]));
        }
        // Only leaves allocate a value slot up front; containers thread
        // their inner body's result slot instead, so no dead slots remain.
        // Leaves report their inferred type; Do and container results stay
        // Unknown, matching top-level action outputs.
        // `target` points the final leaf step at a caller-owned slot (the
        // reduce accumulator): its single write carries a self-edge, which
        // the slot dependency gate accepts for loop-carried state.
        let (value, value_type) = match &step.primitive {
            crate::StepPrimitive::Set { output, value } => {
                let slot = match target {
                    Some(slot) => slot,
                    None => alloc_body_slot(builder)?,
                };
                let inferred =
                    self.lower_set(output, value, BodyTarget { id, next, slot }, builder)?;
                (slot, inferred)
            }
            crate::StepPrimitive::Do { action, input } => {
                let slot = match target {
                    Some(slot) => slot,
                    None => alloc_body_slot(builder)?,
                };
                self.lower_do(action, input, BodyTarget { id, next, slot }, builder)?;
                (slot, ExprType::Unknown)
            }
            crate::StepPrimitive::ForEach { .. }
            | crate::StepPrimitive::Collect { .. }
            | crate::StepPrimitive::Repeat { .. }
            | crate::StepPrimitive::Choose { .. } => {
                let slot = self.lower_nested_container(&step.primitive, id, next, builder)?;
                (slot, ExprType::Unknown)
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
        Ok((value, value_type))
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
            } => self.lower_nested_for_each(input, variable, *at_once, body, id, next, builder),
            crate::StepPrimitive::Collect {
                source,
                variable,
                pages,
                items,
                body,
                ..
            } => {
                self.lower_nested_collect(source, variable, *pages, *items, body, id, next, builder)
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

    #[allow(clippy::too_many_arguments)]
    fn lower_nested_for_each(
        &mut self,
        input: &str,
        variable: &str,
        at_once: Option<u32>,
        body: &[crate::StepAst],
        id: StepIdx,
        next: StepIdx,
        builder: &mut SlotCompiler,
    ) -> Result<SlotIdx, CompileErrors> {
        let output = lower_bounded_for_each(
            self.ctx.index,
            id,
            ForEachLowering {
                input,
                variable,
                at_once,
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

    #[allow(clippy::too_many_arguments)]
    fn lower_nested_collect(
        &mut self,
        source: &str,
        variable: &str,
        pages: Option<u32>,
        items: Option<u32>,
        body: &[crate::StepAst],
        id: StepIdx,
        next: StepIdx,
        builder: &mut SlotCompiler,
    ) -> Result<SlotIdx, CompileErrors> {
        let output: CollectOutput = lower_bounded_collect(
            self.ctx.index,
            id,
            CollectLowering {
                source,
                variable,
                pages,
                items,
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

    fn empty_body_error(index: usize, field: &'static str) -> CompileErrors {
        CompileErrors(vec![CompileError::StepFieldShape {
            step: index,
            field,
            expected: "non-empty body",
        }])
    }
}
