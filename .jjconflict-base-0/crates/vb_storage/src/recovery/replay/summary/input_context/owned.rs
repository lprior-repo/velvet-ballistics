use super::*;

#[derive(Debug)]
pub struct RecoveredReplayOutcome {
    seed: RecoveryFrameSeedProduct,
    cursor: Option<(StepIdx, u64)>,
    store: Option<ValueStore>,
}

impl RecoveredReplayOutcome {
    pub fn into_parts(
        self,
    ) -> (
        RecoveryFrameSeedProduct,
        Option<(StepIdx, u64)>,
        Option<ValueStore>,
    ) {
        (self.seed, self.cursor, self.store)
    }
}

pub fn recover_runtime_frame_seed_from_events_with_replay_outcome(
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
    inputs: &[(SlotIdx, SlotValue, Taint)],
    store: Option<&ValueStore>,
) -> RecoveryResult<RecoveredReplayOutcome> {
    reject_workflow_digest_mismatch(events, workflow.digest())?;
    if inputs.is_empty() {
        if let Some(outcome) = complete_scalar_outcome(events, workflow)? {
            return Ok(outcome);
        }
    }
    context_outcome(events, workflow, inputs, store)
}

fn complete_scalar_outcome(
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
) -> RecoveryResult<Option<RecoveredReplayOutcome>> {
    Ok(
        crate::recovery::complete_scalar::replay_complete_scalar(events, workflow)?.map(
            |replayed| RecoveredReplayOutcome {
                seed: replayed.seed().clone(),
                cursor: None,
                store: None,
            },
        ),
    )
}

fn context_outcome(
    events: &[JournalEvent],
    workflow: &CompiledWorkflow,
    inputs: &[(SlotIdx, SlotValue, Taint)],
    store: Option<&ValueStore>,
) -> RecoveryResult<RecoveredReplayOutcome> {
    let sink = Cell::new(None);
    let ctx = ReplayInputContext {
        inputs,
        store,
        mode: ReplayStoreMode::OwnReplay(&sink),
    };
    let result = recover_runtime_frame_seed_with_context(events, Some(workflow), Some(ctx))?;
    let (seed, cursor) = result;
    let product = RecoveryFrameSeedProduct::from_seed(seed);
    let store = sink.take();
    Ok(RecoveredReplayOutcome {
        seed: product,
        cursor,
        store,
    })
}

pub(super) fn fresh_store(plan: &CompiledWorkflow) -> RecoveryResult<ValueStore> {
    let mut store = ValueStore::with_max_slots(plan.resource_contract().max_slots);
    if let Some(prefix) = plan.symbol_payloads() {
        return vb_core::value_store::compose_compiler_symbols(prefix, &store.checkpoint_v1(), &[])
            .map(|(store, _)| store)
            .map_err(|error| RecoveryError::ReplayDivergence {
                step: StepIdx::ZERO,
                detail: format!("invalid replay input context: {error}"),
            });
    }
    if let Some(metadata) = plan.source_metadata() {
        vb_core::workflow::source_metadata::seed_source_symbols(metadata, &mut store)
            .map_err(context_error)?;
    }
    Ok(store)
}

fn entries_resolve(store: Option<&ValueStore>, entries: &[RecoveredSlotEntry]) -> bool {
    entries.iter().all(|entry| match store {
        Some(store) => validate_reference(store, entry.value).is_ok(),
        None => !matches!(
            entry.value,
            SlotValue::List(_) | SlotValue::Object(_) | SlotValue::Blob(_)
        ),
    })
}

impl ReplayInputContext<'_> {
    pub(in crate::recovery::replay::summary) fn validate_final_slots(
        self,
        slots: &mut RecoveredSlots,
    ) -> RecoveryResult<()> {
        let ReplayStoreMode::OwnReplay(sink) = self.mode else {
            return Ok(());
        };
        let actual = sink.take();
        let valid = entries_resolve(actual.as_ref().or(self.store), &slots.entries);
        sink.set(actual);
        if !valid {
            slots.fully_supported = false;
        }
        Ok(())
    }
}
