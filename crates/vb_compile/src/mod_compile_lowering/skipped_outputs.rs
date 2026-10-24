//! Possibly-skipped output audit.
//!
//! Conservative any-read rejection: slots produced by conditional `do` steps
//! are flagged if any emitted read references them, including reads that
//! appear before the producer in source order. This does not infer a
//! guaranteed execution path, nor does it permit reads under the same guard.

#![forbid(unsafe_code)]

use std::collections::HashSet;

use crate::{CompileError, StepPrimitive, WorkflowSource};
use vb_core::{CompiledNode, CompiledNodeKind, ExprOp, SlotIdx};

use super::SlotCompiler;
use super::part_11::slot_idx_for_step;

pub(super) struct PossiblySkippedOutputs {
    slots: HashSet<SlotIdx>,
}

impl PossiblySkippedOutputs {
    pub(super) fn from_source(source: &WorkflowSource) -> Result<Self, CompileError> {
        let mut slots = HashSet::new();
        for (i, step) in source.steps().iter().enumerate() {
            if step.condition.is_some() && matches!(step.primitive, StepPrimitive::Do { .. }) {
                let idx = slot_idx_for_step(i)?;
                slots.insert(idx);
            }
        }
        if !slots.is_empty() {
            match source.steps().last() {
                Some(last) if matches!(last.primitive, StepPrimitive::Finish { .. }) => {}
                _ => return Err(CompileError::LastStepMustFinish),
            }
        }
        Ok(Self { slots })
    }

    pub(super) fn validate(&self, builder: &SlotCompiler) -> Result<(), CompileError> {
        if self.slots.is_empty() {
            return Ok(());
        }
        for node in builder.nodes.iter() {
            self.check_node(node)?;
        }
        for expr in builder.expressions.iter() {
            for op in expr.ops.iter() {
                if let ExprOp::LoadSlot(slot) = op {
                    self.check(*slot, "expression")?;
                }
            }
        }
        for accessor in builder.accessors.iter() {
            self.check(accessor.root, "accessor.root")?;
        }
        Ok(())
    }

    fn check(&self, slot: SlotIdx, consumer: &'static str) -> Result<(), CompileError> {
        if self.slots.contains(&slot) {
            return Err(CompileError::PossiblySkippedReference { slot, consumer });
        }
        Ok(())
    }

    fn check_node(&self, node: &CompiledNode) -> Result<(), CompileError> {
        match &node.kind {
            CompiledNodeKind::Nop
            | CompiledNodeKind::SetConst { .. }
            | CompiledNodeKind::EvalExpr { .. }
            | CompiledNodeKind::Choose { .. }
            | CompiledNodeKind::TogetherStart { .. }
            | CompiledNodeKind::RepeatStart { .. }
            | CompiledNodeKind::ErrorHandler { .. }
            | CompiledNodeKind::Jump { .. } => Ok(()),
            CompiledNodeKind::Copy { .. }
            | CompiledNodeKind::BuildObject { .. }
            | CompiledNodeKind::BuildList { .. }
            | CompiledNodeKind::Do { .. }
            | CompiledNodeKind::ChooseSlot { .. }
            | CompiledNodeKind::Finish { .. } => self.check_value_kind(node),
            CompiledNodeKind::ForEachStart { .. }
            | CompiledNodeKind::ForEachNext { .. }
            | CompiledNodeKind::ForEachJoin { .. }
            | CompiledNodeKind::TogetherBranch { .. }
            | CompiledNodeKind::TogetherJoin { .. }
            | CompiledNodeKind::CollectStart { .. }
            | CompiledNodeKind::CollectPage { .. }
            | CompiledNodeKind::CollectNext { .. }
            | CompiledNodeKind::CollectFinish { .. } => self.check_collection_kind(node),
            CompiledNodeKind::ReduceStart { .. }
            | CompiledNodeKind::ReduceNext { .. }
            | CompiledNodeKind::ReduceFinish { .. }
            | CompiledNodeKind::RepeatAttempt { .. }
            | CompiledNodeKind::RepeatCheck { .. }
            | CompiledNodeKind::RepeatFinish { .. } => self.check_loop_kind(node),
            CompiledNodeKind::WaitUntil { .. }
            | CompiledNodeKind::WaitEvent { .. }
            | CompiledNodeKind::Ask { .. }
            | CompiledNodeKind::AskResume { .. }
            | CompiledNodeKind::RetryCheck { .. } => self.check_suspend_kind(node),
            _ => self.audit_unsupported(node),
        }
    }

    fn check_value_kind(&self, node: &CompiledNode) -> Result<(), CompileError> {
        match &node.kind {
            CompiledNodeKind::Copy { source } => self.check(*source, "copy.source"),
            CompiledNodeKind::BuildObject { fields } => self.check_build_object(fields),
            CompiledNodeKind::BuildList { items } => self.check_build_list(items),
            CompiledNodeKind::Do { input, .. } => self.check(*input, "do.input"),
            CompiledNodeKind::ChooseSlot { branches, .. } => self.check_choose_slot(branches),
            CompiledNodeKind::Finish { result } => self.check(*result, "finish.result"),
            _ => self.audit_unsupported(node),
        }
    }

    fn check_collection_kind(&self, node: &CompiledNode) -> Result<(), CompileError> {
        match &node.kind {
            CompiledNodeKind::ForEachStart { input, .. } => {
                self.check(*input, "foreachstart.input")
            }
            CompiledNodeKind::ForEachNext { iterator_slot, .. } => {
                self.check(*iterator_slot, "foreachnext.iterator")
            }
            CompiledNodeKind::ForEachJoin { output } => self.check(*output, "foreachjoin.output"),
            CompiledNodeKind::TogetherBranch { accumulator, .. } => {
                self.check(*accumulator, "togetherbranch.accumulator")
            }
            CompiledNodeKind::TogetherJoin { accumulator, .. } => {
                self.check(*accumulator, "togetherjoin.accumulator")
            }
            CompiledNodeKind::CollectStart { source, .. } => {
                self.check(*source, "collectstart.source")
            }
            CompiledNodeKind::CollectPage { collector_slot, .. } => {
                self.check(*collector_slot, "collectpage.collector")
            }
            CompiledNodeKind::CollectNext { collector_slot, .. } => {
                self.check(*collector_slot, "collectnext.collector")
            }
            CompiledNodeKind::CollectFinish { collector_slot } => {
                self.check(*collector_slot, "collectfinish.collector")
            }
            _ => self.audit_unsupported(node),
        }
    }

    fn check_loop_kind(&self, node: &CompiledNode) -> Result<(), CompileError> {
        match &node.kind {
            CompiledNodeKind::ReduceStart { input, .. } => self.check(*input, "reducestart.input"),
            CompiledNodeKind::ReduceNext {
                iterator_slot,
                accumulator,
                ..
            } => self.check_reduce_next(iterator_slot, accumulator),
            CompiledNodeKind::ReduceFinish { accumulator } => {
                self.check(*accumulator, "reducefinish.accumulator")
            }
            CompiledNodeKind::RepeatAttempt { attempt_slot, .. } => {
                self.check(*attempt_slot, "repeatattempt.attempt")
            }
            CompiledNodeKind::RepeatCheck { attempt_slot, .. } => {
                self.check(*attempt_slot, "repeatcheck.attempt")
            }
            CompiledNodeKind::RepeatFinish { result } => self.check(*result, "repeatfinish.result"),
            _ => self.audit_unsupported(node),
        }
    }

    fn check_suspend_kind(&self, node: &CompiledNode) -> Result<(), CompileError> {
        match &node.kind {
            CompiledNodeKind::WaitUntil { deadline_slot } => {
                self.check(*deadline_slot, "waituntil.deadline")
            }
            CompiledNodeKind::WaitEvent {
                event,
                timeout_slot,
            } => self.check_wait_event(event, timeout_slot),
            CompiledNodeKind::Ask {
                prompt,
                timeout_slot,
            } => self.check_ask(prompt, timeout_slot),
            CompiledNodeKind::AskResume { answer } => self.check(*answer, "askresume.answer"),
            CompiledNodeKind::RetryCheck { policy_slot, .. } => {
                self.check(*policy_slot, "retrycheck.policy")
            }
            _ => self.audit_unsupported(node),
        }
    }

    fn audit_unsupported(&self, _node: &CompiledNode) -> Result<(), CompileError> {
        Err(CompileError::ExpressionLoweringUnsupported {
            feature: Box::from("skipped-output audit node kind"),
        })
    }

    fn check_build_object(
        &self,
        fields: &[(vb_core::SymbolId, SlotIdx)],
    ) -> Result<(), CompileError> {
        for (_name, slot) in fields.iter() {
            self.check(*slot, "buildobject.field")?;
        }
        Ok(())
    }

    fn check_build_list(&self, items: &[SlotIdx]) -> Result<(), CompileError> {
        for slot in items.iter() {
            self.check(*slot, "buildlist.item")?;
        }
        Ok(())
    }

    fn check_choose_slot(&self, branches: &[vb_core::SlotBranch]) -> Result<(), CompileError> {
        for branch in branches.iter() {
            self.check(branch.condition, "chooseslot.condition")?;
        }
        Ok(())
    }

    fn check_reduce_next(
        &self,
        iterator_slot: &SlotIdx,
        accumulator: &SlotIdx,
    ) -> Result<(), CompileError> {
        self.check(*iterator_slot, "reducenext.iterator")?;
        self.check(*accumulator, "reducenext.accumulator")
    }

    fn check_wait_event(
        &self,
        event: &SlotIdx,
        timeout_slot: &Option<SlotIdx>,
    ) -> Result<(), CompileError> {
        self.check(*event, "waitevent.event")?;
        if let Some(ts) = timeout_slot {
            self.check(*ts, "waitevent.timeout")?;
        }
        Ok(())
    }

    fn check_ask(
        &self,
        prompt: &SlotIdx,
        timeout_slot: &Option<SlotIdx>,
    ) -> Result<(), CompileError> {
        self.check(*prompt, "ask.prompt")?;
        if let Some(ts) = timeout_slot {
            self.check(*ts, "ask.timeout")?;
        }
        Ok(())
    }
}
