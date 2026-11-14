impl Shard {
    fn prepare_started_evidence(
        context: &mut EvidenceContext<'_>,
        step: vb_core::ids::StepIdx,
    ) -> RuntimeResult<()> {
        Self::prepare_step_started_event(
            context.run,
            context.attempt,
            step,
            context.journal_events,
            context.trace_events,
        )
    }

    fn prepare_failed_evidence(
        context: &mut EvidenceContext<'_>,
        step: vb_core::ids::StepIdx,
    ) -> RuntimeResult<()> {
        Self::prepare_step_failed_event(context.run, context.attempt, step, context.journal_events)
    }

    fn prepare_succeeded_evidence(
        context: &mut EvidenceContext<'_>,
        step: vb_core::ids::StepIdx,
        output: Option<SlotIdx>,
    ) -> RuntimeResult<()> {
        Self::prepare_step_succeeded_event(
            context.run,
            context.attempt,
            step,
            output,
            context.journal_events,
        )
    }
}
