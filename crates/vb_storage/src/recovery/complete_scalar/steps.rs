use super::{driver::Driver, invalid};
use crate::recovery::RecoveryResult;
use vb_core::replay::{ReplayAction, SuspensionKind};
use vb_core::{CompiledNodeKind as K, SlotIdx, StepIdx, StepState};

impl Driver<'_> {
    pub(super) fn start(&mut self, step: StepIdx) -> RecoveryResult<()> {
        if self.active.is_some() || self.frame.pc() != step || self.finished {
            return Err(invalid(
                step,
                "StepStarted differs from replay control edge",
            ));
        }
        self.frame
            .mark_running(step)
            .map_err(|e| invalid(step, e.to_string()))?;
        let node = self
            .plan
            .node(step)
            .ok_or_else(|| invalid(step, "step node missing"))?;
        let action = match ask_failure_action(self.plan, &mut self.frame, node)? {
            Some(action) => action,
            None => vb_core::replay::step::replay_step_with_collect(
                node,
                &mut self.frame,
                &mut self.store,
                self.plan,
                &mut self.collect,
            )
            .map_err(|e| invalid(step, format!("artifact replay failed: {e:?}")))?,
        };
        let prefix = self
            .plan
            .source_metadata()
            .map_or(0, |metadata| metadata.symbols_count());
        if self.store.total_arena_count() != u64::from(prefix) {
            return Err(invalid(
                step,
                "runtime arena allocation outside scalar replay scope",
            ));
        }
        if matches!(
            action,
            ReplayAction::Suspended {
                kind: SuspensionKind::AskPending,
                ..
            }
        ) {
            self.frame
                .increment_executed()
                .map_err(|e| invalid(step, e.to_string()))?;
        }
        self.active = Some((step, action));
        self.captures.clear();
        self.answered = false;
        Ok(())
    }

    pub(super) fn required(&self, step: StepIdx) -> RecoveryResult<Vec<SlotIdx>> {
        let node = self
            .plan
            .node(step)
            .ok_or_else(|| invalid(step, "writer node missing"))?;
        let mut slots: Vec<_> = node.output.slot().into_iter().collect();
        if let K::RepeatCheck { attempt_slot, .. } = node.kind
            && !slots.contains(&attempt_slot)
        {
            slots.push(attempt_slot);
        }
        Ok(slots)
    }

    pub(super) fn succeeded(&mut self, step: StepIdx, output: SlotIdx) -> RecoveryResult<()> {
        let (active, action) = self
            .active
            .as_ref()
            .ok_or_else(|| invalid(step, "success without start"))?;
        if *active != step || self.frame.step_state(step).ok() == Some(StepState::Failed) {
            return Err(invalid(step, "success does not match execution"));
        }
        let expected = match self.ask {
            Some((_, _, answer)) => answer,
            None => self
                .plan
                .node(step)
                .ok_or_else(|| invalid(step, "writer node missing"))?
                .output
                .slot()
                .unwrap_or(SlotIdx::new(0)),
        };
        if output != expected {
            return Err(invalid(
                step,
                "success output receipt differs from compiled writer",
            ));
        }
        if let Some((ask, _, slot)) = self.ask {
            if ask != step || !self.answered || !self.captures.contains(&slot) {
                return Err(invalid(step, "incomplete Ask completion receipt"));
            }
            self.ask = None;
        } else {
            if !matches!(action, ReplayAction::Continue(_) | ReplayAction::Finished)
                || self
                    .required(step)?
                    .iter()
                    .any(|slot| !self.captures.contains(slot))
            {
                return Err(invalid(step, "incomplete successful step receipt"));
            }
            self.frame
                .mark_succeeded(step)
                .map_err(|e| invalid(step, e.to_string()))?;
        }
        self.finished = matches!(action, ReplayAction::Finished);
        self.active = None;
        if let Some(region) = self.plan.loop_region(step)
            && self.frame.pc() == region.body()
        {
            self.rearm(step, region.body())?;
        }
        Ok(())
    }

    pub(super) fn failed(&mut self, step: StepIdx) -> RecoveryResult<()> {
        if self.active.as_ref().map(|value| value.0) != Some(step)
            || self.frame.step_state(step).ok() != Some(StepState::Failed)
        {
            return Err(invalid(step, "failure does not match artifact replay"));
        }
        self.active = None;
        Ok(())
    }

    fn rearm(&mut self, control: StepIdx, body: StepIdx) -> RecoveryResult<()> {
        let region = self
            .plan
            .loop_region(control)
            .ok_or_else(|| invalid(control, "owned loop cache missing"))?;
        if region.body() != body
            || self.frame.step_state(region.start()).ok() != Some(StepState::Succeeded)
        {
            return Err(invalid(control, "owned start authority missing"));
        }
        for raw in body.get()..=control.get() {
            let step = StepIdx::new(raw);
            match self
                .frame
                .step_state(step)
                .map_err(|e| invalid(step, e.to_string()))?
            {
                StepState::Pending | StepState::Succeeded => {}
                StepState::Failed => self
                    .frame
                    .validate_handled_failure_rearm(self.plan, control, step)
                    .map_err(|e| invalid(step, e.to_string()))?,
                _ => return Err(invalid(step, "active state at replay backedge")),
            }
        }
        for raw in body.get()..control.get() {
            let step = StepIdx::new(raw);
            if self.frame.step_state(step).ok() == Some(StepState::Failed) {
                self.frame
                    .rearm_handled_failure(self.plan, control, step)
                    .map_err(|e| invalid(step, e.to_string()))?;
            }
        }
        for raw in body.get()..=control.get() {
            let step = StepIdx::new(raw);
            if self.frame.step_state(step).ok() == Some(StepState::Succeeded) {
                self.frame
                    .mark_pending(step)
                    .map_err(|e| invalid(step, e.to_string()))?;
            }
        }
        Ok(())
    }
}

fn ask_failure_action(
    plan: &vb_core::CompiledWorkflow,
    frame: &mut vb_core::RunFrame,
    node: &vb_core::CompiledNode,
) -> RecoveryResult<Option<ReplayAction>> {
    let K::Ask {
        prompt,
        timeout_slot,
    } = node.kind
    else {
        return Ok(None);
    };
    let Err(error) = vb_core::engine::suspension::validate_ask_inputs(frame, prompt, timeout_slot)
    else {
        return Ok(None);
    };
    frame
        .mark_failed(node.id)
        .map_err(|e| invalid(node.id, e.to_string()))?;
    match vb_core::engine::route_error_handler(plan, frame, node.id, &error)
        .map_err(|e| invalid(node.id, e.to_string()))?
    {
        vb_core::engine::ErrorHandlerOutcome::Routed => {
            Ok(Some(ReplayAction::Continue(frame.pc())))
        }
        _ => Err(invalid(node.id, error.to_string())),
    }
}
