use vb_core::ids::{RunId, StepIdx};
use vb_core::{CompiledNodeKind, frame::StepState};

use super::types::{AskAnswer, AskTicket, PendingAskAuthority, PendingTimerKind, RunState, Shard};
use crate::{RuntimeError, RuntimeResult};

impl Shard {
    /// Looks up the ticket for the currently pending ask boundary.
    #[must_use]
    pub fn lookup_pending_ask_ticket(&self, run: RunId, ask_step: StepIdx) -> Option<AskTicket> {
        self.pending_ask_ticket(run, ask_step)
    }

    pub(crate) fn pending_ask_ticket(&self, run: RunId, ask_step: StepIdx) -> Option<AskTicket> {
        let state = self.run_state_get(run)?;
        let authority = *self.pending_ask_authorities.get(&run)?;
        if !Self::is_asking_boundary_current(state, authority, ask_step) {
            return None;
        }
        let (resume_step, answer_slot, timeout_slot) =
            Self::compiled_ask_resume_authority(state, ask_step)?;
        if !self.ask_timer_matches(run, ask_step, timeout_slot) {
            return None;
        }
        Some(AskTicket::issued(
            run,
            ask_step,
            resume_step,
            answer_slot,
            authority.boundary_seq,
        ))
    }

    fn is_asking_boundary_current(
        state: &RunState,
        authority: PendingAskAuthority,
        ask_step: StepIdx,
    ) -> bool {
        authority.step == ask_step
            && state.frame.pc() == ask_step
            && state.frame.step_state(ask_step) == Ok(StepState::Asking)
    }

    fn compiled_ask_resume_authority(
        state: &RunState,
        ask_step: StepIdx,
    ) -> Option<(
        StepIdx,
        vb_core::ids::SlotIdx,
        Option<vb_core::ids::SlotIdx>,
    )> {
        let ask = state.workflow.node(ask_step)?;
        let CompiledNodeKind::Ask { timeout_slot, .. } = &ask.kind else {
            return None;
        };
        let resume_step = ask.next?;
        match &state.workflow.node(resume_step)?.kind {
            CompiledNodeKind::AskResume { answer } => Some((resume_step, *answer, *timeout_slot)),
            _ => None,
        }
    }

    fn ask_timer_matches(
        &self,
        run: RunId,
        ask_step: StepIdx,
        timeout: Option<vb_core::ids::SlotIdx>,
    ) -> bool {
        match (timeout, self.pending_timer_get(run)) {
            (Some(_), Some(timer)) => timer.step == ask_step && timer.kind == PendingTimerKind::Ask,
            (None, None) => true,
            _ => false,
        }
    }

    pub(crate) fn validate_pending_ask_answer(&self, answer: AskAnswer) -> RuntimeResult<()> {
        let ticket = self
            .pending_ask_ticket(answer.ticket.run(), answer.ticket.ask_step())
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        if ticket != answer.ticket || answer.answer_slot != ticket.answer_slot() {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        Ok(())
    }

    pub(crate) fn record_pending_ask_authority(
        &mut self,
        run: RunId,
        step: StepIdx,
        boundary_seq: vb_storage::EventSeq,
    ) {
        let _previous = self
            .pending_ask_authorities
            .insert(run, PendingAskAuthority { step, boundary_seq });
    }

    pub(crate) fn clear_pending_ask_authority(&mut self, run: RunId) {
        let _removed = self.pending_ask_authorities.swap_remove(&run);
    }
}
