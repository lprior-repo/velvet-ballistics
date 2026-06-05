#![forbid(unsafe_code)]
//! Ask-answer runtime façade helpers.

use vb_core::ids::{RunId, SlotIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledNodeKind;

use crate::shard::AskAnswer;
use crate::{Runtime, RuntimeError, RuntimeResult};

impl Runtime {
    /// Answers the currently pending ask for a run by deriving the active ask ticket.
    pub fn answer_pending_ask_slot(
        &mut self,
        run: RunId,
        answer_slot: SlotIdx,
        value: SlotValue,
        taint: Taint,
        encoded_len: u32,
    ) -> RuntimeResult<()> {
        let shard_index = self.shard_index(run);
        let shard = self
            .shards
            .get_mut(shard_index)
            .ok_or(RuntimeError::RunNotFound)?;
        if !shard.run_state_contains(run) {
            return Err(RuntimeError::RunNotFound);
        }
        let pending_timer = shard
            .pending_timer_get(run)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        if pending_timer.kind != crate::shard::PendingTimerKind::Ask {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        let state = shard.run_state_get(run).ok_or(RuntimeError::RunNotFound)?;
        let ask_node = state
            .workflow
            .node(pending_timer.step)
            .ok_or(RuntimeError::InvalidActionCompletion)?;
        if !matches!(ask_node.kind, CompiledNodeKind::Ask { .. }) {
            return Err(RuntimeError::InvalidActionCompletion);
        }
        let resume_step = ask_node.next.ok_or(RuntimeError::InvalidActionCompletion)?;
        match state.workflow.node(resume_step).map(|node| &node.kind) {
            Some(CompiledNodeKind::AskResume { answer }) if answer == &answer_slot => {}
            _ => return Err(RuntimeError::InvalidActionCompletion),
        }
        let answer = AskAnswer::with_encoded_len(
            crate::shard::AskTicket {
                run,
                ask_step: pending_timer.step,
                resume_step,
            },
            answer_slot,
            value,
            taint,
            encoded_len,
        );
        shard.handle_ask_answer(answer)
    }
}
