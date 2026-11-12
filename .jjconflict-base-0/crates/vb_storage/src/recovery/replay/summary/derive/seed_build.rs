use super::*;

pub(super) fn build_recovery_frame_seed(
    accumulator: FrameSeedAccumulator,
    workflow: Option<&CompiledWorkflow>,
    inputs: Option<ReplayInputContext<'_>>,
) -> RecoveryResult<(RecoveryFrameSeed, Option<(StepIdx, u64)>)> {
    let dimensions = recovery_dimensions(&accumulator, workflow)?;
    if let (Some(plan), Some(_)) = (workflow, accumulator.last_succeeded_step) {
        reject_unbound_retry_policy(plan)?;
    }
    let (slots, cursor) = super::super::hydrate::recover_slots(&accumulator, workflow, inputs)?;
    let cursor = recovered_cursor(&accumulator, cursor);
    assemble_recovery_seed(accumulator, dimensions, slots, cursor)
}

fn assemble_recovery_seed(
    accumulator: FrameSeedAccumulator,
    dimensions: (u16, u16),
    slots: RecoveredSlots,
    cursor: Option<(StepIdx, u64)>,
) -> RecoveryResult<(RecoveryFrameSeed, Option<(StepIdx, u64)>)> {
    let first_step = accumulator.first_step();
    let unsupported = seed_unsupported_state(&accumulator, &slots);
    let seed = RecoveryFrameSeed {
        summary: accumulator.summary,
        first_step,
        step_count: dimensions.0,
        slot_count: dimensions.1,
        pc: cursor.map_or(accumulator.pc, |(pc, _)| pc),
        steps: recovered_steps(accumulator.step_states),
        slots: slots.entries,
        pending_actions: recovered_pending_actions(accumulator.pending_actions),
        unsupported,
    };
    Ok((seed, cursor))
}
