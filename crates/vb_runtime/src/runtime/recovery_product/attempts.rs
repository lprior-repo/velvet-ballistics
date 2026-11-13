#![forbid(unsafe_code)]

use crate::recovery::RecoveredRetainedInputs;
use crate::recovery::RecoveredRunBoundary;
use crate::{RuntimeError, RuntimeResult};
use vb_core::{CompiledNodeKind, CompiledWorkflow, RunFrame, SlotValue, StepIdx};
use vb_storage::JournalEvent;

pub(super) fn validate_owned_run_attempts(
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
    retained: &RecoveredRetainedInputs,
) -> RuntimeResult<()> {
    if !owned_replay_candidate(workflow, retained) {
        return Ok(());
    }
    RunAttemptAuthority::validate(events)
}

pub(super) fn validate_pending_action_capacity(
    boundary: RecoveredRunBoundary,
    workflow: &CompiledWorkflow,
    frame: Option<&RunFrame>,
) -> RuntimeResult<()> {
    let Some(pending) = boundary.pending_action_ticket() else {
        return Ok(());
    };
    let ticket = pending.ticket();
    let expected = expected_action_capacity(ticket.step, workflow, frame)?;
    if ticket.capacity != expected {
        return Err(action_capacity_mismatch());
    }
    Ok(())
}

fn expected_action_capacity(
    step: StepIdx,
    workflow: &CompiledWorkflow,
    frame: Option<&RunFrame>,
) -> RuntimeResult<u16> {
    let node = workflow.node(step).ok_or_else(action_capacity_mismatch)?;
    if let Some(policy) = node.retry {
        return Ok(policy.max_attempts);
    }
    retry_check_capacity(node, workflow, frame)
}

fn retry_check_capacity(
    node: &vb_core::workflow::CompiledNode,
    workflow: &CompiledWorkflow,
    frame: Option<&RunFrame>,
) -> RuntimeResult<u16> {
    let Some(next) = node
        .routing
        .next
        .step()
        .and_then(|step| workflow.node(step))
    else {
        return Ok(1);
    };
    let CompiledNodeKind::RetryCheck { policy_slot, .. } = &next.kind else {
        return Ok(1);
    };
    let value = frame
        .ok_or_else(action_capacity_mismatch)?
        .read_slot(*policy_slot)
        .map_err(|_| action_capacity_mismatch())?;
    let SlotValue::I64(value) = *value else {
        return Err(action_capacity_mismatch());
    };
    u16::try_from(value)
        .ok()
        .filter(|capacity| *capacity > 0)
        .ok_or_else(action_capacity_mismatch)
}

fn owned_replay_candidate(workflow: &CompiledWorkflow, retained: &RecoveredRetainedInputs) -> bool {
    let checkpoint = retained.store_checkpoint();
    vb_storage::recovery::is_owned_for_each_contract(workflow)
        || vb_storage::recovery::is_owned_input_replay_contract(
            workflow,
            retained.initial_inputs(),
            checkpoint.as_ref(),
        )
}

struct RunAttemptAuthority {
    current: u16,
    authorized_next: Option<u16>,
    preceding_failure: bool,
}

impl RunAttemptAuthority {
    fn validate(events: &[JournalEvent]) -> RuntimeResult<()> {
        let mut authority = Self::new();
        for event in events {
            authority.apply(event)?;
        }
        authority.finish()
    }

    fn new() -> Self {
        Self {
            current: 1,
            authorized_next: None,
            preceding_failure: false,
        }
    }

    fn apply(&mut self, event: &JournalEvent) -> RuntimeResult<()> {
        self.apply_attempt_authority(event)?;
        self.preceding_failure = is_current_run_failure(event, self.current);
        Ok(())
    }

    fn apply_attempt_authority(&mut self, event: &JournalEvent) -> RuntimeResult<()> {
        match event {
            JournalEvent::RetryCheckpoint { attempt, .. } => self.authorize_retry(*attempt),
            JournalEvent::RunContinuation { carrier } => {
                self.advance(carrier.header().actual_attempt())
            }
            JournalEvent::StepStarted { attempt, .. } => self.advance(*attempt),
            _ => self.validate_current_attempt(event),
        }
    }

    fn authorize_retry(&mut self, attempt: u16) -> RuntimeResult<()> {
        let next = self.current.checked_add(1).ok_or_else(attempt_mismatch)?;
        if !self.preceding_failure || attempt != next || self.authorized_next.is_some() {
            return Err(attempt_mismatch());
        }
        self.authorized_next = Some(attempt);
        Ok(())
    }

    fn advance(&mut self, attempt: u16) -> RuntimeResult<()> {
        if attempt == self.current {
            return Ok(());
        }
        if self.authorized_next == Some(attempt) {
            self.current = attempt;
            self.authorized_next = None;
            return Ok(());
        }
        Err(attempt_mismatch())
    }

    fn validate_current_attempt(&self, event: &JournalEvent) -> RuntimeResult<()> {
        if event
            .attempt()
            .is_some_and(|attempt| attempt != self.current)
        {
            return Err(attempt_mismatch());
        }
        Ok(())
    }

    fn finish(self) -> RuntimeResult<()> {
        if self.authorized_next.is_some() {
            return Err(attempt_mismatch());
        }
        Ok(())
    }
}

fn is_current_run_failure(event: &JournalEvent, current: u16) -> bool {
    matches!(event, JournalEvent::RunFailedEvent { attempt, .. } if *attempt == current)
}

fn attempt_mismatch() -> RuntimeError {
    RuntimeError::RecoveryCannotResume {
        reason: String::from("run_attempt_authority_mismatch"),
    }
}

fn action_capacity_mismatch() -> RuntimeError {
    RuntimeError::RecoveryCannotResume {
        reason: String::from("pending_action_capacity_mismatch"),
    }
}
