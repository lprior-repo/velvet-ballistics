use super::*;

pub(super) type Outcome = Result<vb_core::EngineSignal, EngineError>;

pub(super) fn has_list_variant(value: SlotValue) -> bool {
    matches!(value, SlotValue::List(_))
}

pub(super) struct Fixture {
    pub(super) store: ValueStore,
    pub(super) run: RunFrame,
}

impl Fixture {
    pub(super) fn fresh() -> Self {
        Self {
            run: fresh_frame(),
            store: ValueStore::new(),
        }
    }

    pub(super) fn with_counts(step_count: u16, slot_count: u16) -> Self {
        Self {
            run: fresh_frame_with(step_count, slot_count),
            store: ValueStore::new(),
        }
    }

    pub(super) fn seed(&mut self, accumulator: SlotIdx, items: Vec<SlotValue>) {
        list_in_slot(&mut self.run, &mut self.store, accumulator, items);
    }

    pub(super) fn branch(
        &mut self,
        branch: impl Into<BranchIdx>,
        route: (StepIdx, StepIdx),
        slots: (SlotIdx, Option<SlotIdx>),
    ) -> Outcome {
        let (entry, join) = route;
        let (accumulator, output) = slots;
        together_branch(
            &mut self.run,
            &mut self.store,
            branch,
            entry,
            join,
            accumulator,
            output,
        )
    }

    pub(super) fn join(
        &mut self,
        branches: u16,
        slots: (SlotIdx, Option<SlotIdx>),
        continuation: (Option<StepIdx>, StepIdx),
    ) -> Outcome {
        let (accumulator, output) = slots;
        let (next, step) = continuation;
        together_join(
            &mut self.run,
            &mut self.store,
            branches,
            accumulator,
            output,
            next,
            step,
        )
    }
}
