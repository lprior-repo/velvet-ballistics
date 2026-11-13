#![forbid(unsafe_code)]

use super::{Driver, K, invalid};
use crate::JournalEvent as E;
use crate::recovery::RecoveryResult;
use crate::recovery::types::RecoveryError;

#[path = "events/admission.rs"]
mod admission;

impl Driver<'_> {
    pub(super) fn apply_bound_event(&mut self, event: &E) -> RecoveryResult<()> {
        self.validate_sequence(event)?;
        self.close_admission_prefix(event);
        self.dispatch_event(event)?;
        self.validate_step_budget()
    }

    fn close_admission_prefix(&mut self, event: &E) {
        if !matches!(
            event,
            E::RunAccepted { .. }
                | E::RunAdmission { .. }
                | E::RunAdmissionBudget { .. }
                | E::CompatibilityDecisionEvent { .. }
                | E::TenantAdmissionEvent { .. }
                | E::RunBudgetOriginEvent { .. }
        ) {
            self.admission_prefix_closed = true;
        }
    }

    fn validate_sequence(&mut self, event: &E) -> RecoveryResult<()> {
        if event.run_id() != self.run_id
            || self
                .previous_seq
                .is_some_and(|seq| seq.checked_add(1) != Some(event.seq().get()))
        {
            return Err(invalid(self.frame.pc(), "incomplete bound event sequence"));
        }
        self.previous_seq = Some(event.seq().get());
        Ok(())
    }

    fn dispatch_event(&mut self, event: &E) -> RecoveryResult<()> {
        match event {
            E::RunAccepted { .. }
            | E::RunAdmission { .. }
            | E::RunAdmissionBudget { .. }
            | E::RunContinuation { .. }
            | E::CompatibilityDecisionEvent { .. }
            | E::TenantAdmissionEvent { .. }
            | E::RunBudgetOriginEvent { .. }
            | E::PolicyDecisionEvent { .. }
            | E::GateOpenedEvent { .. }
            | E::GateDecisionEvent { .. }
            | E::RetryCheckpoint { .. } => self.apply_admission_event(event),
            E::StepStarted { .. }
            | E::SlotWrittenEvent { .. }
            | E::StepSucceeded { .. }
            | E::StepFailed { .. }
            | E::AskScheduledEvent { .. }
            | E::AskAnsweredEvent { .. }
            | E::AskTimedOutEvent { .. }
            | E::ActionFailedEvent { .. }
            | E::RunFailedEvent { .. } => self.apply_execution_event(event),
            E::ActionScheduledTicket { .. } | E::ActionCompletedEnvelope { .. } => {
                self.apply_action_event(event)
            }
            E::WaitScheduledEvent { .. } | E::WaitResolvedEvent { .. } => {
                self.apply_wait_event(event)
            }
            E::RunFinished { .. } => self.apply_finished_event(event),
            _ => Err(self.unsupported_event(event)),
        }
    }

    fn apply_execution_event(&mut self, event: &E) -> RecoveryResult<()> {
        match event {
            E::StepStarted { step, .. } if self.accepted && self.admitted => {
                self.started = true;
                self.start(*step)
            }
            E::SlotWrittenEvent {
                run,
                seq,
                slot,
                value,
                extra,
                ..
            } => self.capture(*run, *seq, *slot, value.as_deref(), extra.as_deref()),
            E::StepSucceeded { step, output, .. } => self.succeeded(*step, *output),
            E::StepFailed { step, .. } => self.failed(*step),
            E::AskScheduledEvent { step, .. } => self.scheduled_ask(*step),
            E::AskAnsweredEvent { step, .. } => self.answered_ask(*step),
            E::AskTimedOutEvent { step, .. } => self.timed_out_ask(*step),
            E::ActionFailedEvent { step, .. } => self.action_failed(*step),
            E::RunFailedEvent { .. } if self.active.is_none() => {
                self.terminal = true;
                Ok(())
            }
            _ => Err(self.unsupported_event(event)),
        }
    }

    fn apply_action_event(&mut self, event: &E) -> RecoveryResult<()> {
        match event {
            E::ActionScheduledTicket {
                ticket,
                input,
                output,
                action_abi_digest,
                ..
            } => self.scheduled_action(*ticket, *input, *output, *action_abi_digest),
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
            } => self.completed_action(super::super::action_complete::ActionCompletion {
                ticket: *ticket,
                outcome: *outcome,
                output: *output,
                value,
                taint: *taint,
                value_digest,
                encoded_len: *encoded_len,
                abi_digest: *action_abi_digest,
            }),
            _ => Err(self.unsupported_event(event)),
        }
    }

    fn apply_wait_event(&mut self, event: &E) -> RecoveryResult<()> {
        match event {
            E::WaitScheduledEvent { step, attempt, .. } => self.wait_scheduled(*step, *attempt),
            E::WaitResolvedEvent { step, attempt, .. } => self.wait_resolved(*step, *attempt),
            _ => Err(self.unsupported_event(event)),
        }
    }

    fn apply_finished_event(&mut self, event: &E) -> RecoveryResult<()> {
        let E::RunFinished { result, .. } = event else {
            return Err(self.unsupported_event(event));
        };
        if !self.finished || self.terminal || self.active.is_some() {
            return Err(self.unsupported_event(event));
        }
        let Some(K::Finish { result: expected }) =
            self.plan.node(self.frame.pc()).map(|node| &node.kind)
        else {
            return Err(invalid(self.frame.pc(), "terminal without Finish node"));
        };
        if expected != result {
            return Err(invalid(
                self.frame.pc(),
                "terminal result differs from Finish node",
            ));
        }
        self.frame
            .read_slot(*result)
            .map_err(|error| invalid(self.frame.pc(), error.to_string()))?;
        self.terminal = true;
        Ok(())
    }

    fn validate_step_budget(&self) -> RecoveryResult<()> {
        if self.frame.executed() > self.max_total_steps {
            return Err(invalid(
                self.frame.pc(),
                "bound trace exceeds workflow step budget",
            ));
        }
        Ok(())
    }

    pub(super) fn unsupported_event(&self, event: &E) -> RecoveryError {
        invalid(
            self.frame.pc(),
            format!(
                "unsupported or out-of-order bound event {:?}",
                event.record_kind()
            ),
        )
    }
}
