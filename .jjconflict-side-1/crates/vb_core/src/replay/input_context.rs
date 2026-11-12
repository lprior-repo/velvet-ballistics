use super::*;

impl<'a> ReplayEngine<'a> {
    /// Seeds retained input values and taints before deterministic replay.
    pub fn with_initial_inputs(mut self, inputs: &'a [(SlotIdx, SlotValue, Taint)]) -> Self {
        self.initial_inputs = inputs;
        self
    }

    pub(super) fn new_replay_frame(&self) -> Result<RunFrame, ReplayError> {
        let mut frame = RunFrame::new(
            RunId::new(0),
            self.plan.entry(),
            self.plan.node_count(),
            self.plan.slot_count(),
        )
        .map_err(engine_to_replay_err)?;

        self.initial_inputs
            .iter()
            .try_for_each(|(slot, value, taint)| {
                frame
                    .write_slot_with_taint(*slot, *value, *taint)
                    .map_err(engine_to_replay_err)
            })?;

        Ok(frame)
    }
}
