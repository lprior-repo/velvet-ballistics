fn core_event_seq(seq: vb_storage::EventSeq) -> EventSeq {
    EventSeq::new(seq.get())
}

/// Builds collect pagination state from durable journal events recovered for a run.
pub fn hydrate_collect_states_from_recovered_journal(
    events: &[JournalEvent],
) -> Result<CollectStates, EngineError> {
    let mut states = CollectStates::new();
    states.hydrate_journal_events(events)?;
    Ok(states)
}

fn validate_hydrated_identity(
    state: &CollectPaginationState,
    run_id: RunId,
    collector_slot: SlotIdx,
    event_seq: Option<EventSeq>,
) -> Result<(), EngineError> {
    if state.run_id != run_id {
        return Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::RunMismatch {
                expected: run_id,
                actual: state.run_id,
            },
            run_id,
            collector_slot,
            event_seq,
        });
    }
    if state.collector_slot != collector_slot {
        return Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::SlotMismatch {
                expected: collector_slot,
                actual: state.collector_slot,
            },
            run_id,
            collector_slot,
            event_seq,
        });
    }
    Ok(())
}

fn validate_hydrated_page(
    state: &CollectPaginationState,
    run_id: RunId,
    collector_slot: SlotIdx,
    event_seq: Option<EventSeq>,
    expected: ListId,
) -> Result<(), EngineError> {
    if state.current_page != expected {
        return Err(EngineError::CollectExtraHydrationFailed {
            kind: CollectExtraHydrationFailureKind::CurrentPageMismatch {
                expected,
                actual: state.current_page,
            },
            run_id,
            collector_slot,
            event_seq,
        });
    }
    Ok(())
}

fn collect_page_from_event_value(
    run_id: RunId,
    collector_slot: SlotIdx,
    event_seq: Option<EventSeq>,
    value: Option<&[u8]>,
) -> Result<Option<ListId>, EngineError> {
    match value {
        Some(bytes) => match postcard::from_bytes::<SlotValue>(bytes) {
            Ok(SlotValue::List(page)) => Ok(Some(page)),
            Ok(_) => Ok(None),
            Err(_) => Err(EngineError::CollectExtraHydrationFailed {
                kind: CollectExtraHydrationFailureKind::DecodeFailed,
                run_id,
                collector_slot,
                event_seq,
            }),
        },
        None => Ok(None),
    }
}
