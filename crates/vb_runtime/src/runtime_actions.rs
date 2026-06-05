#![forbid(unsafe_code)]

use vb_core::action::{ActionFailure, ActionOutputReady, ActionTicket};
use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::{CompiledNodeKind, CompiledWorkflow};

use crate::runtime::Runtime;
use crate::shard::timer_wheel::TimerEntry;
use crate::shard::{
    AskAnswer, AskTicket, InspectResponse, PendingTimer, PendingTimerKind, ShardCommand,
};
use crate::trace::TraceEvent;
use crate::{RuntimeError, RuntimeResult};

impl Runtime {
    /// Cancels a run.
    pub fn cancel_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Cancel { run, reason: None })
    }

    /// Kills a run unconditionally.
    pub fn kill_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Kill { run, reason: None })
    }

    /// Resumes a suspended run from its current program counter.
    pub fn resume_run(&self, run: RunId) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Resume { run })
    }

    /// Inspects run state.
    pub fn inspect_run(&self, run: RunId, correlation: u64) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::Inspect { run, correlation })
    }

    /// Returns a direct, non-queued run snapshot from the owning shard.
    pub fn snapshot_run(&self, run: RunId, correlation: u64) -> RuntimeResult<InspectResponse> {
        let shard = self.shard_for(run)?;
        Ok(shard.snapshot_run(run, correlation))
    }

    /// Completes an action for a run.
    pub fn complete_action(&self, run: RunId, step: StepIdx) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        shard.enqueue(ShardCommand::ActionCompletedLegacy { run, step })
    }

    /// Completes an action for a run with its typed output payload.
    pub fn complete_action_with_output(
        &mut self,
        ticket: ActionTicket,
        output: ActionOutputReady,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for_mut(ticket.run)?;
        if let Some(state) = shard.run_state_get(ticket.run) {
            crate::shard::lifecycle::preflight_action_completion(state, ticket, output.clone())?;
        }
        shard.enqueue(ShardCommand::ActionCompleted { ticket, output })
    }

    /// Fails an action with a typed failure payload.
    pub fn fail_action(&self, ticket: ActionTicket, failure: ActionFailure) -> RuntimeResult<()> {
        let shard = self.shard_for(ticket.run)?;
        shard.enqueue(ShardCommand::RuntimeActionFailed { ticket, failure })
    }

    /// Lists trace events for a run without draining the shard trace ring.
    pub fn list_events(&self, run: RunId) -> RuntimeResult<Vec<TraceEvent>> {
        let shard_index = self.shard_index(run);
        let Some(shard) = self.shards.get(shard_index) else {
            return Err(RuntimeError::RunNotFound);
        };
        let limit = shard.trace_ring().capacity();
        Ok(shard.trace_ring().snapshot_for_run(run, limit))
    }

    /// Answers an ask with an explicit typed payload and resume ticket.
    pub fn answer_ask(&self, answer: AskAnswer) -> RuntimeResult<()> {
        let shard = self.shard_for(answer.ticket.run)?;
        if !shard.run_state_contains(answer.ticket.run)
            && shard.terminal_runs_contains(answer.ticket.run)
        {
            return Err(RuntimeError::RunNotFound);
        }
        shard.enqueue(ShardCommand::AskAnswered { answer })
    }

    /// Answers the currently pending ask for a run by answer destination slot.
    pub fn answer_pending_ask_slot(
        &self,
        run: RunId,
        answer_slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
        encoded_len: u32,
    ) -> RuntimeResult<()> {
        let shard = self.shard_for(run)?;
        let ticket = pending_ask_ticket(shard, run, answer_slot)?;
        shard.enqueue(ShardCommand::AskAnswered {
            answer: AskAnswer {
                ticket,
                answer_slot,
                value,
                taint,
                encoded_len,
            },
        })
    }

    /// Legacy run-only timer delivery is fail-closed because it carries no authority.
    pub fn timer_fired(&self, run: RunId) -> RuntimeResult<()> {
        let _shard = self.shard_for(run)?;
        Err(RuntimeError::InvalidTimerFire)
    }

    /// Captures the current timer authority for tests and typed scheduler handoff.
    pub fn capture_timer_entry(&self, run: RunId) -> RuntimeResult<TimerEntry> {
        let shard = self.shard_for(run)?;
        shard.timer_entry(run).ok_or(RuntimeError::InvalidTimerFire)
    }

    /// Advances a run from a timer-wheel-captured authority entry.
    pub fn timer_entry_fired(&self, entry: TimerEntry) -> RuntimeResult<()> {
        let shard = self.shard_for(entry.run)?;
        shard.enqueue(ShardCommand::TimerFired {
            run: entry.run,
            generation: entry.generation,
            deadline: entry.deadline,
            kind: entry.kind,
        })
    }

    /// Takes the latest inspect response from the run's shard.
    pub fn take_inspect_response(&mut self, run: RunId) -> RuntimeResult<Option<InspectResponse>> {
        let shard_index = self.shard_index(run);
        let shard = self
            .shards
            .get_mut(shard_index)
            .ok_or(RuntimeError::RunNotFound)?;
        Ok(shard.take_inspect_response())
    }

    /// Drains all trace events from all shards.
    pub fn drain_trace(&mut self) -> Vec<TraceEvent> {
        let mut events = Vec::new();
        self.shards.iter_mut().for_each(|shard| {
            let capacity = shard.trace_ring_mut().capacity();
            shard.trace_ring_mut().drain_into(capacity, &mut events);
        });
        events
    }
}

fn pending_ask_ticket(
    shard: &crate::shard::Shard,
    run: RunId,
    answer_slot: SlotIdx,
) -> RuntimeResult<AskTicket> {
    let state = shard.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
    let pending_timer = shard
        .pending_timer_get(run)
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    pending_ask_ticket_from_parts(run, &state.workflow, pending_timer, answer_slot)
}

pub(crate) fn pending_ask_ticket_from_parts(
    run: RunId,
    workflow: &CompiledWorkflow,
    pending_timer: PendingTimer,
    answer_slot: SlotIdx,
) -> RuntimeResult<AskTicket> {
    let ask_next = workflow.node(pending_timer.step).and_then(|node| node.next);
    let resume_answer = ask_next.and_then(|resume_step| {
        workflow.node(resume_step).and_then(|node| match node.kind {
            CompiledNodeKind::AskResume { answer } => Some(answer),
            _ => None,
        })
    });
    match derive_ask_ticket_from_parts(
        run,
        pending_timer.kind,
        pending_timer.step,
        ask_next,
        resume_answer,
        answer_slot,
    ) {
        AskTicketDerivation::Ticket(ticket) => Ok(ticket),
        AskTicketDerivation::InvalidActionCompletion => Err(RuntimeError::InvalidActionCompletion),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(kani), allow(unreachable_pub))]
pub enum AskTicketDerivation {
    Ticket(AskTicket),
    InvalidActionCompletion,
}

pub(crate) fn derive_ask_ticket_from_parts(
    run: RunId,
    pending_kind: PendingTimerKind,
    ask_step: StepIdx,
    ask_next: Option<StepIdx>,
    resume_answer: Option<SlotIdx>,
    answer_slot: SlotIdx,
) -> AskTicketDerivation {
    if pending_kind != PendingTimerKind::Ask {
        return AskTicketDerivation::InvalidActionCompletion;
    }
    let Some(resume_step) = ask_next else {
        return AskTicketDerivation::InvalidActionCompletion;
    };
    match resume_answer {
        Some(answer) if answer == answer_slot => AskTicketDerivation::Ticket(AskTicket {
            run,
            ask_step,
            resume_step,
        }),
        _ => AskTicketDerivation::InvalidActionCompletion,
    }
}

#[cfg(kani)]
pub fn kani_derive_ask_ticket_from_parts(
    run: RunId,
    pending_kind: PendingTimerKind,
    ask_step: StepIdx,
    ask_next: Option<StepIdx>,
    resume_answer: Option<SlotIdx>,
    answer_slot: SlotIdx,
) -> AskTicketDerivation {
    derive_ask_ticket_from_parts(
        run,
        pending_kind,
        ask_step,
        ask_next,
        resume_answer,
        answer_slot,
    )
}
