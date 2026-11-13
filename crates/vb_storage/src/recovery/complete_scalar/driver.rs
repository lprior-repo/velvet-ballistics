use super::action_wait::ScheduledAction;
use super::{TraceReplayInput, TraceReplayMode, invalid};
use crate::recovery::RecoveryResult;
use crate::{
    JournalEvent as E,
    slot_extra::{DecodedSlotWrittenExtra, decode_slot_written_extra},
};
use std::collections::HashSet;
use vb_core::replay::ReplayAction;
use vb_core::replay::step::ReplayCollectStates;
use vb_core::{
    CompiledNodeKind as K, CompiledWorkflow, RunFrame, SlotIdx, SlotValue, StepIdx, Taint,
    ValueStore,
};

pub(super) struct Driver<'a> {
    pub(super) plan: &'a CompiledWorkflow,
    pub(super) frame: RunFrame,
    pub(super) store: ValueStore,
    pub(super) collect: ReplayCollectStates,
    pub(super) active: Option<(StepIdx, ReplayAction)>,
    pub(super) captures: HashSet<SlotIdx>,
    pub(super) ask: Option<(StepIdx, StepIdx, SlotIdx)>,
    pub(super) answer: Option<(SlotValue, Taint)>,
    pub(super) answered: bool,
    pub(super) finished: bool,
    terminal: bool,
    pub(super) action: Option<ScheduledAction>,
    pub(super) waiting: Option<(StepIdx, u16)>,
    pub(super) mode: TraceReplayMode,
}

pub(super) fn replay(events: &[E], plan: &CompiledWorkflow) -> RecoveryResult<RunFrame> {
    replay_with_inputs(
        events,
        plan,
        &TraceReplayInput {
            initial_inputs: &[],
            store_checkpoint: None,
            mode: TraceReplayMode::Scalar,
        },
    )
}

pub(crate) fn replay_with_inputs(
    events: &[E],
    plan: &CompiledWorkflow,
    input: &TraceReplayInput<'_>,
) -> RecoveryResult<RunFrame> {
    replay_with_inputs_and_store(events, plan, input).map(|(frame, _store)| frame)
}

pub(crate) fn replay_with_inputs_and_store(
    events: &[E],
    plan: &CompiledWorkflow,
    input: &TraceReplayInput<'_>,
) -> RecoveryResult<(RunFrame, ValueStore)> {
    let first = events
        .first()
        .ok_or_else(|| invalid(plan.entry(), "empty bound trace"))?;
    let store = match input.store_checkpoint {
        Some(checkpoint) => {
            ValueStore::from_checkpoint_v1(checkpoint.clone()).map_err(|error| {
                invalid(plan.entry(), format!("invalid owned value store: {error}"))
            })?
        }
        None => {
            let mut store = ValueStore::with_max_slots(plan.resource_contract().max_slots);
            if let Some(metadata) = plan.source_metadata() {
                vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)
                    .map_err(|e| invalid(plan.entry(), e.to_string()))?;
            }
            store
        }
    };
    let mut frame = vb_core::new_run_frame(first.run_id(), plan)
        .map_err(|e| invalid(plan.entry(), e.to_string()))?;
    for (slot, value, taint) in input.initial_inputs {
        frame
            .write_slot_with_taint(*slot, *value, *taint)
            .map_err(|e| invalid(plan.entry(), e.to_string()))?;
    }
    let mut driver = Driver {
        plan,
        frame,
        store,
        collect: ReplayCollectStates::new(),
        active: None,
        captures: HashSet::new(),
        ask: None,
        answer: None,
        answered: false,
        finished: false,
        terminal: false,
        action: None,
        waiting: None,
        mode: input.mode,
    };
    let mut previous = None;
    let mut accepted = false;
    let mut admitted = false;
    let mut started = false;
    let parts = plan.to_parts();
    let budget =
        vb_core::WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)
            .map_err(|e| invalid(plan.entry(), e.to_string()))?;
    if input.mode == TraceReplayMode::Scalar && budget.max_parallel_in_flight != 0 {
        return Err(invalid(
            plan.entry(),
            "parallel trace is outside scalar replay",
        ));
    }
    driver
        .frame
        .set_max_parallel_in_flight(budget.max_parallel_in_flight);
    for event in events {
        if event.run_id() != first.run_id()
            || previous.is_some_and(|seq: u64| seq.checked_add(1) != Some(event.seq().get()))
        {
            return Err(invalid(
                driver.frame.pc(),
                "incomplete bound event sequence",
            ));
        }
        previous = Some(event.seq().get());
        match event {
            E::RunAccepted { workflow, .. } if !accepted && *workflow == plan.digest() => {
                accepted = true
            }
            E::RunAdmission { .. } | E::RunAdmissionBudget { .. }
                if accepted && !admitted && !started =>
            {
                admitted = true;
            }
            E::RunContinuation { .. } if accepted && admitted && !started => {}
            E::CompatibilityDecisionEvent { .. }
            | E::TenantAdmissionEvent { .. }
            | E::PolicyDecisionEvent { .. }
            | E::GateOpenedEvent { .. }
            | E::GateDecisionEvent { .. }
            | E::RetryCheckpoint { .. }
                if accepted => {}
            E::StepStarted { step, .. } if accepted && admitted => {
                started = true;
                driver.start(*step)?;
            }
            E::SlotWrittenEvent {
                slot, value, extra, ..
            } => driver.capture(*slot, value.as_deref(), extra.as_deref())?,
            E::StepSucceeded { step, output, .. } => driver.succeeded(*step, *output)?,
            E::StepFailed { step, .. } => driver.failed(*step)?,
            E::AskScheduledEvent { step, .. } => driver.scheduled_ask(*step)?,
            E::AskAnsweredEvent { step, .. } => driver.answered_ask(*step)?,
            E::AskTimedOutEvent { step, .. } => driver.timed_out_ask(*step)?,
            E::ActionFailedEvent { step, .. } => driver.action_failed(*step)?,
            E::RunFailedEvent { .. } if driver.active.is_none() => driver.terminal = true,
            E::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest,
                ..
            } => driver.scheduled_action(*ticket, *input, *output, *action_abi_digest)?,
            E::ActionCompletedEnvelope {
                ticket,
                outcome,
                output,
                value,
                taint,
                value_digest,
                encoded_len,
                action_abi_digest,
                ..
            } => driver.completed_action(super::action_complete::ActionCompletion {
                ticket: *ticket,
                outcome: *outcome,
                output: *output,
                value,
                taint: *taint,
                value_digest,
                encoded_len: *encoded_len,
                abi_digest: *action_abi_digest,
            })?,
            E::WaitScheduledEvent { step, attempt, .. } => {
                driver.wait_scheduled(*step, *attempt)?;
            }
            E::WaitResolvedEvent { step, attempt, .. } => {
                driver.wait_resolved(*step, *attempt)?;
            }
            E::RunFinished { result, .. }
                if driver.finished && !driver.terminal && driver.active.is_none() =>
            {
                let Some(K::Finish { result: expected }) =
                    plan.node(driver.frame.pc()).map(|node| &node.kind)
                else {
                    return Err(invalid(driver.frame.pc(), "terminal without Finish node"));
                };
                if expected != result {
                    return Err(invalid(
                        driver.frame.pc(),
                        "terminal result differs from Finish node",
                    ));
                }
                driver
                    .frame
                    .read_slot(*result)
                    .map_err(|e| invalid(driver.frame.pc(), e.to_string()))?;
                driver.terminal = true;
            }
            _ => {
                return Err(invalid(
                    driver.frame.pc(),
                    format!(
                        "unsupported or out-of-order bound event {:?}",
                        event.record_kind()
                    ),
                ));
            }
        }
        if driver.frame.executed() > budget.max_total_steps {
            return Err(invalid(
                driver.frame.pc(),
                "bound trace exceeds workflow step budget",
            ));
        }
    }
    if driver.active.is_some() && !driver.pending_boundary() {
        return Err(invalid(driver.frame.pc(), "incomplete step receipt"));
    }
    if input.mode == TraceReplayMode::Scalar && driver.frame.parallel_in_flight() != 0 {
        return Err(invalid(
            driver.frame.pc(),
            "unknown scalar parallel reservations",
        ));
    }
    if driver.finished && !driver.terminal {
        return Err(invalid(driver.frame.pc(), "terminal receipt missing"));
    }
    Ok((driver.frame, driver.store))
}

impl Driver<'_> {
    pub(super) fn capture(
        &mut self,
        slot: SlotIdx,
        bytes: Option<&[u8]>,
        extra: Option<&[u8]>,
    ) -> RecoveryResult<()> {
        let (step, action) = self
            .active
            .as_ref()
            .ok_or_else(|| invalid(self.frame.pc(), "capture without active step"))?;
        let step = *step;
        if self.frame.step_state(step).ok() == Some(vb_core::StepState::Failed) {
            return Err(invalid(step, "failed step cannot emit output capture"));
        }
        let value =
            postcard::from_bytes(bytes.ok_or_else(|| invalid(step, "missing captured value"))?)
                .map_err(|_| invalid(step, "invalid captured value"))?;
        let DecodedSlotWrittenExtra::Envelope(envelope) = decode_slot_written_extra(
            extra.ok_or_else(|| invalid(step, "missing captured taint"))?,
        )
        .map_err(|_| invalid(step, "invalid captured taint"))?
        else {
            return Err(invalid(step, "unbound legacy taint capture"));
        };
        if envelope.frame_extra.is_some() {
            return Err(invalid(step, "unproven scalar frame component"));
        }
        if let Some((_, _, answer)) = self.ask {
            if slot != answer || self.answer.is_some() || self.answered {
                return Err(invalid(step, "unauthorized Ask capture"));
            }
            if let SlotValue::Symbol(id) = value
                && self.store.symbol(id).is_err()
            {
                return Err(invalid(step, "Ask symbol outside replay source prefix"));
            }
            self.answer = Some((value, envelope.taint));
        } else {
            if !self.required(step, action)?.contains(&slot)
                || self.frame.read_slot(slot).ok() != Some(&value)
                || self.frame.read_taint(slot).ok() != Some(envelope.taint)
            {
                return Err(invalid(step, "capture differs from artifact replay"));
            }
        }
        if !self.captures.insert(slot) {
            return Err(invalid(step, "duplicate step capture"));
        }
        Ok(())
    }
}
