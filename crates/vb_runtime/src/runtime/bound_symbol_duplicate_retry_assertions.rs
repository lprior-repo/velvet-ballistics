use super::*;
use vb_core::frame::{CheckpointStepState, RunFrameCheckpointV1};
use vb_core::value_store::ValueStoreCheckpointV1;

pub(super) fn initial_runtime_store() -> ValueStoreCheckpointV1 {
    ValueStoreCheckpointV1 {
        symbols: vec!["key".into(), "noise".into()],
        lists: vec![],
        list_taints: vec![],
        objects: vec![Box::from([ObjectField::clean(
            SymbolId::new(0),
            SlotValue::Symbol(SymbolId::new(1)),
        )])],
        blobs: vec![],
        max_arena_entries: u64::from(vb_core::workflow::ResourceContract::DEFAULT.max_slots),
    }
}

fn full_store() -> ValueStoreCheckpointV1 {
    ValueStoreCheckpointV1 {
        symbols: vec!["key".into(), "noise".into(), "key".into()],
        lists: vec![Box::from([
            SlotValue::Symbol(SymbolId::new(2)),
            SlotValue::I64(13),
        ])],
        list_taints: vec![Box::from([Taint::Clean, Taint::Secret])],
        objects: vec![
            Box::from([ObjectField::clean(
                SymbolId::new(0),
                SlotValue::Symbol(SymbolId::new(1)),
            )]),
            Box::from([ObjectField {
                key: SymbolId::new(2),
                value: SlotValue::Symbol(SymbolId::new(1)),
                taint: Taint::Clean,
            }]),
        ],
        blobs: vec![],
        max_arena_entries: u64::from(vb_core::workflow::ResourceContract::DEFAULT.max_slots),
    }
}

fn expected_frame(action: CheckpointStepState) -> RunFrameCheckpointV1 {
    RunFrameCheckpointV1 {
        run_id: RUN,
        pc: ACTION,
        executed: 1,
        max_parallel_in_flight: 0,
        parallel_in_flight: 0,
        states: vec![
            CheckpointStepState::Succeeded,
            action,
            CheckpointStepState::Pending,
            CheckpointStepState::Pending,
        ],
        slots: expected_slots(),
        taint: vec![
            Taint::Clean,
            Taint::Clean,
            Taint::Clean,
            Taint::Clean,
            Taint::Secret,
        ],
    }
}

fn expected_slots() -> Vec<Option<SlotValue>> {
    vec![
        Some(SlotValue::Object(ObjectId::new(1))),
        Some(SlotValue::I64(11)),
        None,
        None,
        Some(SlotValue::List(vb_core::ListId::new(0))),
    ]
}

pub(super) fn assert_literal_state(state: &RunState) -> TestResult {
    assert_eq!(state.store.checkpoint_v1(), full_store());
    assert_eq!(
        state.frame.checkpoint_v1(),
        expected_frame(CheckpointStepState::Pending)
    );
    assert_eq!(state.action_attempts.as_ref(), [0, 0, 0, 0]);
    assert_collect(&state.collect_states.checkpoint_v1())
}

fn assert_collect(actual: &crate::primitives::collect::CollectStatesCheckpointV1) -> TestResult {
    assert_eq!(actual.entries, vec![expected_collect_entry()]);
    assert_collect_lineage(actual)
}

fn expected_collect_entry() -> crate::primitives::collect::CollectPaginationState {
    crate::primitives::collect::CollectPaginationState {
        run_id: RUN,
        collector_slot: SlotIdx::new(4),
        source: vb_core::ListId::new(0),
        current_page: vb_core::ListId::new(0),
        cursor: 1,
        page_size: 2,
        item_count: 1,
        limit: 3,
        time_limit_ms: Some(50),
        start_millis: 101,
    }
}

fn assert_collect_lineage(
    actual: &crate::primitives::collect::CollectStatesCheckpointV1,
) -> TestResult {
    assert_eq!(
        (actual.total_items_consumed, actual.total_pages_delivered),
        (1, 1)
    );
    assert_eq!(actual.lineages.len(), 1);
    let lineage = actual.lineages.first().ok_or("missing lineage")?;
    assert_eq!(
        (lineage.run_id, lineage.collector_slot),
        (RUN, SlotIdx::new(4))
    );
    assert_eq!(lineage.previous_page, None);
    assert_eq!(lineage.stale_pages, Vec::<vb_core::ListId>::new());
    Ok(())
}

pub(super) fn assert_ticket_input(runtime: &Runtime, ticket: ActionTicket) -> TestResult {
    assert_eq!(
        (ticket.run, ticket.step, ticket.action, ticket.attempt),
        (RUN, ACTION, vb_core::ActionId::new(0), 1)
    );
    let state = runtime.inspect_run_state(RUN)?;
    assert_eq!(
        *state.frame.read_slot(SlotIdx::ZERO)?,
        SlotValue::Object(ObjectId::new(1))
    );
    assert_eq!(state.store.checkpoint_v1(), full_store());
    Ok(())
}

pub(super) fn assert_produced(checkpoint: &RunCheckpointV1, before: &RunState) -> TestResult {
    assert_eq!(checkpoint.value_store, full_store());
    assert_eq!(
        checkpoint.frame,
        expected_frame(CheckpointStepState::Pending)
    );
    assert_eq!(checkpoint.action_attempts.as_ref(), [0, 1, 0, 0]);
    assert_eq!(
        checkpoint.collect_states,
        before.collect_states.checkpoint_v1()
    );
    assert_collect(&checkpoint.collect_states)?;
    assert_eq!(checkpoint.admission, before.admission);
    assert_eq!(checkpoint.action_contracts, Box::from([contract()?]));
    assert_eq!(checkpoint.run_started_millis, before.run_started_millis);
    assert_eq!(
        (
            checkpoint.run_id,
            checkpoint.run_attempt.get(),
            checkpoint.retry_step
        ),
        (RUN, 2, ACTION)
    );
    assert!(checkpoint.journal_bytes_written > before.journal_bytes_written);
    Ok(())
}

pub(super) fn assert_recovered(runtime: &Runtime, evidence: &Evidence) -> TestResult {
    let state = runtime.inspect_run_state(RUN)?;
    let checkpoint = &evidence.checkpoint;
    assert_eq!(state.store.checkpoint_v1(), full_store());
    assert_eq!(
        state.frame.checkpoint_v1(),
        expected_frame(CheckpointStepState::Running)
    );
    assert_eq!(state.action_attempts.as_ref(), [0, 1, 0, 0]);
    assert_eq!(state.admission, evidence.before.admission);
    assert_eq!(
        state.collect_states.checkpoint_v1(),
        checkpoint.collect_states
    );
    assert_collect(&state.collect_states.checkpoint_v1())?;
    assert_eq!(state.action_contracts, checkpoint.action_contracts);
    assert_eq!(state.run_started_millis, checkpoint.run_started_millis);
    assert_eq!(state.run_attempt.get(), 2);
    assert_eq!(
        state.registered_execution,
        crate::shard::types::RegisteredExecutionMode::ReconcileOnly
    );
    assert!(state.journal_bytes_written > checkpoint.journal_bytes_written);
    Ok(())
}

pub(super) fn assert_terminal(journal: &FjallJournal) -> TestResult {
    let events = journal.events_for_run_full(RUN)?;
    assert_finished(&events);
    assert_has_write(&events)
}

fn assert_finished(events: &[JournalEvent]) {
    let finished: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::RunFinished {
                run,
                result,
                attempt,
                ..
            } => Some((*run, *result, *attempt)),
            _ => None,
        })
        .collect();
    assert_eq!(finished, [(RUN, SlotIdx::new(3), 2)]);
}

fn assert_has_write(events: &[JournalEvent]) -> TestResult {
    let writes: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            JournalEvent::SlotWrittenEvent { slot, value, .. } if *slot == SlotIdx::new(3) => {
                Some(value)
            }
            _ => None,
        })
        .collect();
    assert_eq!(writes.len(), 1);
    let value = writes
        .first()
        .ok_or("missing Has write")?
        .as_ref()
        .ok_or("missing Has payload")?;
    assert_eq!(
        postcard::from_bytes::<SlotValue>(value)?,
        SlotValue::Bool(false)
    );
    Ok(())
}
