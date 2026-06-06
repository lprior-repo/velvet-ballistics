#![forbid(unsafe_code)]
//! Ask-answer runtime façade helpers.

use vb_core::ids::{RunId, SlotIdx, StepIdx};
use vb_core::value::{SlotValue, Taint};
use vb_core::workflow::CompiledNodeKind;

use crate::shard::{AskAnswer, AskTicket, PendingTimer, PendingTimerKind, Shard};
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
        let shard = shard_for_pending_ask(&mut self.shards, shard_index, run)?;
        let pending_timer = ask_timer_for_run(shard, run)?;
        let resume_step = ask_resume_step(shard, run, pending_timer, answer_slot)?;
        let answer = ask_answer(
            run,
            pending_timer,
            resume_step,
            answer_slot,
            value,
            taint,
            encoded_len,
        );
        shard.handle_ask_answer(answer)
    }
}

fn shard_for_pending_ask(
    shards: &mut [Shard],
    shard_index: usize,
    run: RunId,
) -> RuntimeResult<&mut Shard> {
    let shard = shards
        .get_mut(shard_index)
        .ok_or(RuntimeError::RunNotFound)?;
    if shard.run_state_contains(run) {
        Ok(shard)
    } else {
        Err(RuntimeError::RunNotFound)
    }
}

fn ask_timer_for_run(shard: &Shard, run: RunId) -> RuntimeResult<PendingTimer> {
    let pending_timer = shard
        .pending_timer_get(run)
        .ok_or(RuntimeError::InvalidActionCompletion)?;
    if pending_timer.kind == PendingTimerKind::Ask {
        Ok(pending_timer)
    } else {
        Err(RuntimeError::InvalidActionCompletion)
    }
}

fn ask_resume_step(
    shard: &Shard,
    run: RunId,
    pending_timer: PendingTimer,
    answer_slot: SlotIdx,
) -> RuntimeResult<StepIdx> {
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
        Some(CompiledNodeKind::AskResume { answer }) if answer == &answer_slot => Ok(resume_step),
        _ => Err(RuntimeError::InvalidActionCompletion),
    }
}

fn ask_answer(
    run: RunId,
    pending_timer: PendingTimer,
    resume_step: StepIdx,
    answer_slot: SlotIdx,
    value: SlotValue,
    taint: Taint,
    encoded_len: u32,
) -> AskAnswer {
    let ticket = AskTicket {
        run,
        ask_step: pending_timer.step,
        resume_step,
    };
    AskAnswer::with_encoded_len(ticket, answer_slot, value, taint, encoded_len)
}
