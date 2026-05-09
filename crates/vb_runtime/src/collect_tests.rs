use super::*;
use crate::test_harness::list_in_slot;
use vb_core::value_store::ValueStore;
use vb_storage::recovery::{ActionReplayTracker, recover_full_journal};
use vb_storage::{EventSeq, JournalEvent};

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(8, 8)
}

fn fresh_states() -> CollectStates {
    CollectStates::new()
}

fn assert_invalid_workflow_reason(result: Result<(), EngineError>, expected: &'static str) {
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => assert_eq!(reason, expected),
        other => assert_eq!(
            other,
            Err(EngineError::InvalidCompiledWorkflow { reason: expected })
        ),
    }
}

fn captured_collect_extra(run: &mut RunFrame, collector: SlotIdx) -> Result<Vec<u8>, String> {
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(&mut *run, &mut store, source, vec![SlotValue::I64(10)]);
    collect_start(
        run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;
    states
        .capture_extra(run.run_id(), collector)
        .map_err(|e| format!("capture: {e:?}"))?
        .ok_or("expected pagination extra".to_owned())
}

fn slot_written_extra(run: RunId, slot: SlotIdx, extra: Vec<u8>) -> JournalEvent {
    JournalEvent::SlotWrittenEvent {
        run,
        seq: EventSeq::new(0),
        attempt: 1,
        slot,
        value: None,
        extra: Some(extra),
    }
}

fn assert_slot_list_items(
    run: &RunFrame,
    store: &ValueStore,
    slot: SlotIdx,
    expected: &[SlotValue],
) {
    match *run
        .read_slot(slot)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            assert_eq!(items, expected);
        }
        other => {
            assert_eq!(other, SlotValue::Null);
        }
    }
}

fn slot_list_id(run: &RunFrame, slot: SlotIdx) -> Result<ListId, String> {
    match *run.read_slot(slot).map_err(|e| format!("{e:?}"))? {
        SlotValue::List(id) => Ok(id),
        other => Err(format!("expected List, got {other:?}")),
    }
}

fn collect_state_for_current_page(
    states: &CollectStates,
    run: &RunFrame,
    collector: SlotIdx,
) -> Result<CollectPaginationState, String> {
    let current_page = slot_list_id(run, collector)?;
    states
        .find(run.run_id(), collector, current_page)
        .ok_or("collect state missing for current page".to_owned())
}

struct CollectScenario {
    run: RunFrame,
    store: ValueStore,
    states: CollectStates,
    collector: SlotIdx,
    body: StepIdx,
    done: StepIdx,
}

impl CollectScenario {
    fn start(items: Vec<SlotValue>, limit: u32, page_size: u32) -> Result<Self, String> {
        let mut run = fresh_frame();
        let mut store = ValueStore::new();
        let mut states = fresh_states();
        let source = SlotIdx::new(0);
        let collector = SlotIdx::new(1);
        let body = StepIdx::new(1);
        let done = StepIdx::new(2);

        list_in_slot(&mut run, &mut store, source, items);
        collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            limit,
            page_size,
            body,
            done,
            Some(collector),
            None,
        )
        .map_err(|e| format!("collect_start: {e:?}"))?;

        Ok(Self {
            run,
            store,
            states,
            collector,
            body,
            done,
        })
    }

    fn next(&mut self) -> Result<vb_core::EngineSignal, String> {
        collect_next(
            &mut self.run,
            &mut self.store,
            &mut self.states,
            self.collector,
            self.body,
            self.done,
        )
        .map_err(|e| format!("collect_next: {e:?}"))
    }

    fn current_page(&self) -> Result<ListId, String> {
        slot_list_id(&self.run, self.collector)
    }

    fn assert_current_cursor(&self, expected: usize) -> Result<(), String> {
        assert_eq!(
            collect_state_for_current_page(&self.states, &self.run, self.collector)?.cursor,
            expected
        );
        Ok(())
    }

    fn assert_live_cursor(&self, page: ListId, expected: usize) -> Result<(), String> {
        assert_eq!(
            self.states
                .find(self.run.run_id(), self.collector, page)
                .ok_or("live state missing after stale rejection".to_owned())?
                .cursor,
            expected
        );
        Ok(())
    }

    fn assert_collector_items(&self, expected: &[SlotValue]) -> Result<(), String> {
        let id = slot_list_id(&self.run, self.collector)?;
        let items = self
            .store
            .list(id)
            .map_err(|e| format!("collector list read: {e:?}"))?;
        assert_eq!(items, expected);
        Ok(())
    }

    fn write_collector_page(&mut self, page: ListId) -> Result<(), String> {
        self.run
            .write_slot(self.collector, SlotValue::List(page))
            .map_err(|e| format!("write collector page: {e:?}"))
    }

    fn reject_next(&mut self) {
        assert_eq!(
            collect_next(
                &mut self.run,
                &mut self.store,
                &mut self.states,
                self.collector,
                self.body,
                self.done,
            ),
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "collect pagination state missing"
            })
        );
    }
}

fn non_monotonic_collect_scenario() -> Result<CollectScenario, String> {
    CollectScenario::start(
        vec![
            SlotValue::I64(30),
            SlotValue::I64(10),
            SlotValue::I64(20),
            SlotValue::I64(40),
            SlotValue::I64(15),
        ],
        5,
        2,
    )
}

fn duplicate_page_rejection_scenario() -> Result<(CollectScenario, ListId), String> {
    let mut scenario = CollectScenario::start(
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
        ],
        4,
        2,
    )?;
    let duplicate_page = scenario.current_page()?;
    scenario.next()?;
    let advanced_page = scenario.current_page()?;
    scenario.assert_current_cursor(4)?;
    scenario.write_collector_page(duplicate_page)?;
    scenario.reject_next();
    Ok((scenario, advanced_page))
}

fn stale_page_rejection_scenario() -> Result<(CollectScenario, ListId), String> {
    let mut scenario = CollectScenario::start(
        vec![SlotValue::I64(7), SlotValue::I64(8), SlotValue::I64(9)],
        3,
        1,
    )?;
    let stale_page = scenario.current_page()?;
    scenario.next()?;
    let live_page = scenario.current_page()?;
    scenario.write_collector_page(stale_page)?;
    scenario.reject_next();
    Ok((scenario, live_page))
}

#[test]
fn collect_start_initializes_collector() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    let slot_val = *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"));
    assert!(matches!(slot_val, SlotValue::List(_)));
}

#[test]
fn collect_page_increments_page_count() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(10)]);
    let result = collect_page(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

#[test]
fn collect_next_advances_to_next_page_while_page_has_items() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(5), SlotValue::I64(6), SlotValue::I64(7)],
    );
    let start = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector),
        None,
    );
    assert_eq!(start, Ok(vb_core::EngineSignal::Continue));
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[SlotValue::I64(5), SlotValue::I64(6)],
    );
    let result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(7)]);
}

#[test]
fn collect_finish_materializes_output() {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next_step = StepIdx::new(3);
    run.write_slot(collector, SlotValue::I64(99))
        .ok()
        .unwrap_or_else(|| panic!("slot write must succeed"));
    let result = collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(next_step),
        StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), next_step);
    assert_eq!(
        *run.read_slot(output)
            .ok()
            .unwrap_or_else(|| panic!("read must succeed")),
        SlotValue::I64(99)
    );
}

#[test]
fn collect_start_returns_error_when_source_is_not_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    run.write_slot(source, SlotValue::Bool(true))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_returns_error_when_limit_exceeded() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        3,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    match result {
        Err(EngineError::CollectItemLimitExceeded) => {}
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_returns_error_when_output_missing() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        None,
        None,
    );
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_returns_error_when_page_size_zero() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        0,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => {
            assert_eq!(reason, "collect page_size must be nonzero");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_jumps_to_done_when_source_empty() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, source, vec![]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        done,
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn collect_next_returns_done_when_remaining_empty() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, collector, vec![]);
    let result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        done,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn collect_finish_returns_error_when_output_missing() {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    run.write_slot(collector, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let result = collect_finish(
        &mut run,
        &mut states,
        collector,
        None,
        Some(StepIdx::new(1)),
        StepIdx::ZERO,
    );
    match result {
        Err(EngineError::MissingOutputSlot { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_finish_returns_error_when_next_missing() {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    run.write_slot(collector, SlotValue::I64(1))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let result = collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        None,
        StepIdx::ZERO,
    );
    match result {
        Err(EngineError::MissingNextStep { step }) => {
            assert_eq!(step, StepIdx::ZERO);
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_page_returns_error_when_collector_not_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    run.write_slot(collector, SlotValue::I64(42))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "number");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_writes_first_page_to_collector() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("read must succeed"))
    {
        SlotValue::List(id) => {
            let items = store
                .list(id)
                .ok()
                .unwrap_or_else(|| panic!("list read must succeed"));
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(0), Some(&SlotValue::I64(1)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(2)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn collect_start_increments_executed_counter() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
    let before = run.executed();
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn collect_next_increments_executed_with_pagination_state() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    let start = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(collector),
        None,
    );
    assert_eq!(start, Ok(vb_core::EngineSignal::Continue));
    let before = run.executed();
    let result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn collect_page_increments_executed_counter() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, collector, vec![SlotValue::I64(1)]);
    let before = run.executed();
    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn collect_finish_increments_executed_counter() {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    run.write_slot(collector, SlotValue::I64(99))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let before = run.executed();
    let result = collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(StepIdx::new(1)),
        StepIdx::ZERO,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.executed(), before + 1);
}

#[test]
fn collect_next_rejects_nonempty_current_page_without_state() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    list_in_slot(
        &mut run,
        &mut store,
        collector,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    let result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    assert_eq!(
        result,
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect pagination state missing",
        })
    );
    assert_eq!(run.pc(), StepIdx::ZERO);
}

#[test]
fn collect_next_returns_error_when_not_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    run.write_slot(collector, SlotValue::Bool(true))
        .ok()
        .unwrap_or_else(|| panic!("write must succeed"));
    let result = collect_next(
        &mut run,
        &mut store,
        &mut states,
        collector,
        StepIdx::new(1),
        StepIdx::new(2),
    );
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "boolean");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_zero_items_with_nonzero_limit_goes_to_done() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let done = StepIdx::new(3);
    list_in_slot(&mut run, &mut store, source, vec![]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        done,
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
}

#[test]
fn collect_start_page_size_zero_returns_error_even_for_empty_list() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(&mut run, &mut store, source, vec![]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        0,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(
        result,
        Err(EngineError::InvalidCompiledWorkflow {
            reason: "collect page_size must be nonzero",
        })
    );
}

#[test]
fn collect_start_items_at_exact_limit_boundary() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let body = StepIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        3,
        2,
        body,
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
}

#[test]
fn collect_start_items_exceeding_limit_by_one() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
        ],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        3,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    match result {
        Err(EngineError::CollectItemLimitExceeded) => {}
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_first_page_smaller_than_total() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 2);
            assert_eq!(items.get(0), Some(&SlotValue::I64(1)));
            assert_eq!(items.get(1), Some(&SlotValue::I64(2)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn collect_start_page_size_larger_than_items_clamps_to_item_count() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(42), SlotValue::I64(99)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        10,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 2);
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn collect_next_progresses_pages_then_jumps_done() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    let start = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector),
        None,
    );
    assert_eq!(start, Ok(vb_core::EngineSignal::Continue));
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[SlotValue::I64(1), SlotValue::I64(2)],
    );
    let next = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(next, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), body);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(3)]);
    let finished = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    assert_eq!(finished, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), done);
    assert_slot_list_items(&run, &store, collector, &[]);
}

#[test]
fn collect_start_null_source_returns_type_mismatch() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    run.write_slot(source, SlotValue::Null)
        .ok()
        .unwrap_or_else(|| panic!("write"));
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    match result {
        Err(EngineError::TypeMismatch { expected, found }) => {
            assert_eq!(expected, "list");
            assert_eq!(found, "null");
        }
        other => {
            assert_eq!(other, Ok(vb_core::EngineSignal::Continue));
        }
    }
}

#[test]
fn collect_start_page_size_one_single_item_per_page() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    match *run
        .read_slot(output)
        .ok()
        .unwrap_or_else(|| panic!("must read"))
    {
        SlotValue::List(id) => {
            let items = store.list(id).ok().unwrap_or_else(|| panic!("must read"));
            assert_eq!(items.len(), 1);
            assert_eq!(items.get(0), Some(&SlotValue::I64(10)));
        }
        other => {
            assert_eq!(other, SlotValue::I64(0));
        }
    }
}

#[test]
fn collect_start_rejects_page_size_above_limit() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        1,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
    assert_eq!(run.pc(), StepIdx::ZERO);
}

#[test]
fn collect_start_page_size_u32_max_returns_error() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(1)]);
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        u32::MAX,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
}

#[test]
fn collect_start_page_size_at_limit_boundary() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        2,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    assert_eq!(run.pc(), StepIdx::new(1));
}

#[test]
fn collect_start_page_size_exactly_one_over_limit() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        1,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(SlotIdx::new(1)),
        None,
    );
    assert_eq!(result, Err(EngineError::CollectPageLimitExceeded));
}

#[test]
fn collect_start_with_time_limit_stores_limit_in_state() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector),
        Some(60000),
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    let state = states.entries.values().next();
    assert!(state.is_some());
    assert_eq!(state.unwrap().time_limit_ms, Some(60000));
}

#[test]
fn collect_start_without_time_limit_stores_none() {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector),
        None,
    );
    assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
    let state = states.entries.values().next();
    assert!(state.is_some());
    assert_eq!(state.unwrap().time_limit_ms, None);
}

#[test]
fn check_time_limit_returns_error_when_exceeded() {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: Some(1),
        start_millis: 0,
    };
    assert_eq!(
        check_time_limit(&state),
        Err(EngineError::CollectTimeLimitExceeded)
    );
}

#[test]
fn check_time_limit_ok_when_not_exceeded() -> Result<(), EngineError> {
    let now = millis_since_epoch()?;
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: Some(60000),
        start_millis: now,
    };
    assert_eq!(check_time_limit(&state), Ok(()));
    Ok(())
}

#[test]
fn check_time_limit_ok_when_no_limit_set() {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    assert_eq!(check_time_limit(&state), Ok(()));
}

// ── Result-returning tests with ensure pattern ─────────────────────

fn ensure(condition: bool, message: impl Into<String>) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

#[test]
fn collect_states_new_is_empty() -> Result<(), String> {
    let states = CollectStates::new();
    ensure(
        states
            .find(RunId::new(1), SlotIdx::new(0), ListId::new(0))
            .is_none(),
        "new states should be empty",
    )
}

#[test]
fn collect_states_upsert_and_find_roundtrip() -> Result<(), String> {
    let mut states = CollectStates::new();
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 5,
        page_size: 10,
        item_count: 30,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states
        .upsert(state)
        .map_err(|e| format!("upsert failed: {e:?}"))?;
    let found = states
        .find(RunId::new(1), SlotIdx::new(0), ListId::new(20))
        .ok_or("state not found after upsert")?;
    ensure(
        found.cursor == 5,
        format!("expected cursor 5, got {}", found.cursor),
    )?;
    ensure(
        found.page_size == 10,
        format!("expected page_size 10, got {}", found.page_size),
    )?;
    ensure(
        found.item_count == 30,
        format!("expected item_count 30, got {}", found.item_count),
    )
}

#[test]
fn collect_states_find_returns_none_for_wrong_page() -> Result<(), String> {
    let mut states = CollectStates::new();
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states
        .upsert(state)
        .map_err(|e| format!("upsert failed: {e:?}"))?;
    let found = states.find(RunId::new(1), SlotIdx::new(0), ListId::new(99));
    ensure(found.is_none(), "find should return None for wrong page id")
}

#[test]
fn collect_states_find_returns_none_for_wrong_run_id() -> Result<(), String> {
    let mut states = CollectStates::new();
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states
        .upsert(state)
        .map_err(|e| format!("upsert failed: {e:?}"))?;
    let found = states.find(RunId::new(999), SlotIdx::new(0), ListId::new(20));
    ensure(found.is_none(), "find should return None for wrong run_id")
}

#[test]
fn collect_states_remove_clears_entry() -> Result<(), String> {
    let mut states = CollectStates::new();
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states
        .upsert(state)
        .map_err(|e| format!("upsert failed: {e:?}"))?;
    states.remove(RunId::new(1), SlotIdx::new(0));
    ensure(
        states
            .find(RunId::new(1), SlotIdx::new(0), ListId::new(20))
            .is_none(),
        "state should be gone after remove",
    )
}

#[test]
fn collect_states_upsert_replaces_existing() -> Result<(), String> {
    let mut states = CollectStates::new();
    let state_v1 = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states
        .upsert(state_v1)
        .map_err(|e| format!("upsert v1 failed: {e:?}"))?;
    let state_v2 = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(30),
        cursor: 10,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states
        .upsert(state_v2)
        .map_err(|e| format!("upsert v2 failed: {e:?}"))?;
    let found = states
        .find(RunId::new(1), SlotIdx::new(0), ListId::new(30))
        .ok_or("state v2 not found")?;
    ensure(
        found.cursor == 10,
        format!("expected cursor 10, got {}", found.cursor),
    )
}

#[test]
fn collect_states_default_is_empty() -> Result<(), String> {
    let states = CollectStates::default();
    ensure(
        states
            .find(RunId::new(0), SlotIdx::new(0), ListId::new(0))
            .is_none(),
        "default states should be empty",
    )
}

#[test]
fn collect_pagination_state_copy_equality() -> Result<(), String> {
    let a = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(2),
        source: ListId::new(3),
        current_page: ListId::new(4),
        cursor: 5,
        page_size: 10,
        item_count: 20,
        limit: 30,
        time_limit_ms: Some(1000),
        start_millis: 500,
    };
    let b = a;
    ensure(a == b, "identical states should be equal")
}

#[test]
fn collect_pagination_state_inequality() -> Result<(), String> {
    let a = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    let b = CollectPaginationState {
        run_id: RunId::new(2),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    ensure(a != b, "states with different run_id should not be equal")
}

#[test]
fn page_size_from_rejects_zero() -> Result<(), String> {
    let result = page_size_from(0);
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => ensure(
            reason == "collect page_size must be nonzero",
            format!("unexpected reason: {reason}"),
        ),
        other => Err(format!("expected InvalidCompiledWorkflow, got {other:?}")),
    }
}

#[test]
fn page_size_from_accepts_one() -> Result<(), String> {
    let result = page_size_from(1).map_err(|e| format!("{e:?}"))?;
    ensure(result == 1, format!("expected 1, got {result}"))
}

#[test]
fn validate_page_bound_rejects_page_above_limit() -> Result<(), String> {
    let result = validate_page_bound(10, 5);
    match result {
        Err(EngineError::CollectPageLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectPageLimitExceeded, got {other:?}")),
    }
}

#[test]
fn validate_page_bound_accepts_page_at_limit() -> Result<(), String> {
    validate_page_bound(5, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn validate_page_bound_accepts_page_below_limit() -> Result<(), String> {
    validate_page_bound(3, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn validate_item_limit_rejects_count_above_limit() -> Result<(), String> {
    let result = validate_item_limit(10, 5);
    match result {
        Err(EngineError::CollectItemLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectItemLimitExceeded, got {other:?}")),
    }
}

#[test]
fn validate_item_limit_accepts_count_at_limit() -> Result<(), String> {
    validate_item_limit(5, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn validate_item_limit_accepts_count_below_limit() -> Result<(), String> {
    validate_item_limit(3, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn copy_prefix_returns_first_page_size_items() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(3),
        SlotValue::I64(4),
        SlotValue::I64(5),
    ]
    .into_boxed_slice();
    let prefix = copy_prefix(&items, 3).map_err(|e| format!("{e:?}"))?;
    ensure(
        prefix.len() == 3,
        format!("expected len 3, got {}", prefix.len()),
    )?;
    ensure(prefix.get(0) == Some(&SlotValue::I64(1)), "item 0 mismatch")?;
    ensure(prefix.get(1) == Some(&SlotValue::I64(2)), "item 1 mismatch")?;
    ensure(prefix.get(2) == Some(&SlotValue::I64(3)), "item 2 mismatch")
}

#[test]
fn copy_prefix_clamps_to_item_count() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice();
    let prefix = copy_prefix(&items, 100).map_err(|e| format!("{e:?}"))?;
    ensure(
        prefix.len() == 2,
        format!("expected len 2, got {}", prefix.len()),
    )
}

#[test]
fn copy_page_range_returns_correct_slice() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![
        SlotValue::I64(0),
        SlotValue::I64(1),
        SlotValue::I64(2),
        SlotValue::I64(3),
        SlotValue::I64(4),
    ]
    .into_boxed_slice();
    let page = copy_page_range(&items, 2, 2).map_err(|e| format!("{e:?}"))?;
    ensure(
        page.len() == 2,
        format!("expected len 2, got {}", page.len()),
    )?;
    ensure(page.get(0) == Some(&SlotValue::I64(2)), "item 0 mismatch")?;
    ensure(page.get(1) == Some(&SlotValue::I64(3)), "item 1 mismatch")
}

#[test]
fn copy_page_range_clamps_at_end() -> Result<(), String> {
    let items: Box<[SlotValue]> =
        vec![SlotValue::I64(0), SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice();
    let page = copy_page_range(&items, 2, 10).map_err(|e| format!("{e:?}"))?;
    ensure(
        page.len() == 1,
        format!("expected len 1, got {}", page.len()),
    )?;
    ensure(page.get(0) == Some(&SlotValue::I64(2)), "item 0 mismatch")
}

#[test]
fn validate_collect_state_rejects_page_size_above_limit() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 50,
        item_count: 10,
        limit: 30,
        time_limit_ms: None,
        start_millis: 0,
    };
    let result = validate_collect_state(&state, 10);
    match result {
        Err(EngineError::CollectPageLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectPageLimitExceeded, got {other:?}")),
    }
}

#[test]
fn validate_collect_state_rejects_item_count_above_limit() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 5,
        item_count: 50,
        limit: 30,
        time_limit_ms: None,
        start_millis: 0,
    };
    let result = validate_collect_state(&state, 50);
    match result {
        Err(EngineError::CollectItemLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectItemLimitExceeded, got {other:?}")),
    }
}

#[test]
fn validate_collect_state_rejects_source_length_change() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 5,
        item_count: 10,
        limit: 30,
        time_limit_ms: None,
        start_millis: 0,
    };
    let result = validate_collect_state(&state, 20);
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => ensure(
            reason == "collect source length changed",
            format!("unexpected reason: {reason}"),
        ),
        other => Err(format!("expected InvalidCompiledWorkflow, got {other:?}")),
    }
}

#[test]
fn validate_collect_state_accepts_valid_state() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 5,
        item_count: 10,
        limit: 30,
        time_limit_ms: None,
        start_millis: 0,
    };
    validate_collect_state(&state, 10).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

#[test]
fn checked_add_usize_succeeds_for_valid_addition() -> Result<(), String> {
    let result = checked_add_usize(5, 3, "test").map_err(|e| format!("{e:?}"))?;
    ensure(result == 8, format!("expected 8, got {result}"))
}

#[test]
fn checked_add_usize_fails_on_overflow() -> Result<(), String> {
    let result = checked_add_usize(usize::MAX, 1, "overflow test");
    match result {
        Err(EngineError::InternalInvariantViolation { reason }) => ensure(
            reason == "overflow test",
            format!("unexpected reason: {reason}"),
        ),
        other => Err(format!(
            "expected InternalInvariantViolation, got {other:?}"
        )),
    }
}

#[test]
fn collect_start_uses_source_as_collector_when_output_is_none_for_non_empty() -> Result<(), String>
{
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    // When output is None, source slot is used as the collector.
    // This means the source slot is overwritten with the first page.
    let result = collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        None,
        None,
    );
    let signal = result.map_err(|e| format!("collect_start failed: {e:?}"))?;
    ensure(
        signal == vb_core::EngineSignal::Continue,
        "expected Continue",
    )?;
    ensure(
        run.pc() == body,
        format!("expected pc={body:?}, got {:?}", run.pc()),
    )?;
    // The source slot should now hold the first page (a list of 2 items)
    match *run.read_slot(source).map_err(|e| format!("{e:?}"))? {
        SlotValue::List(id) => {
            let items = store.list(id).map_err(|e| format!("{e:?}"))?;
            ensure(
                items.len() == 2,
                format!("expected 2 items in page, got {}", items.len()),
            )
        }
        other => return Err(format!("expected List, got {other:?}")),
    }
}

#[test]
fn collect_next_validates_state_consistency() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    // Set up a source list with 5 items
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
    );
    // Start collect with page_size=2
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start failed: {e:?}"))?;
    // Now modify the source list to have a different length (simulating mutation)
    let new_items: Vec<SlotValue> = (0..10).map(SlotValue::I64).collect();
    let new_id = store
        .insert_list(new_items.into_boxed_slice())
        .map_err(|e| format!("{e:?}"))?;
    // Find the state and get the source ListId
    let current_page = match *run.read_slot(collector).map_err(|e| format!("{e:?}"))? {
        SlotValue::List(id) => id,
        other => return Err(format!("expected List, got {other:?}")),
    };
    let state = states
        .find(run.run_id(), collector, current_page)
        .ok_or("state not found")?;
    // Replace source with a different-length list
    let mut modified_state = state;
    // Write a new source with wrong length into the store using the same source id
    // Since we can't mutate store in-place, let's trigger the validation via
    // collect_next which reads from store.list(state.source)
    // We need the source list to have a different length than state.item_count
    // Insert a new list and update the state's source to point to it
    // Actually we can write over the source slot in the run, but the state
    // tracks the original source ListId. Let's insert a new list into the store
    // and update the pagination state to point to it.
    modified_state.source = new_id;
    states
        .upsert(modified_state)
        .map_err(|e| format!("{e:?}"))?;
    // Now collect_next should fail because source length changed
    let result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => ensure(
            reason == "collect source length changed",
            format!("unexpected: {reason}"),
        ),
        other => Err(format!("expected InvalidCompiledWorkflow, got {other:?}")),
    }
}

#[test]
fn collect_finish_removes_state_after_writing_output() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(3);
    run.write_slot(collector, SlotValue::I64(77))
        .map_err(|e| format!("{e:?}"))?;
    // Insert a state to verify it's removed
    let state = CollectPaginationState {
        run_id: run.run_id(),
        collector_slot: collector,
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states.upsert(state).map_err(|e| format!("{e:?}"))?;
    collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(next),
        StepIdx::ZERO,
    )
    .map_err(|e| format!("collect_finish failed: {e:?}"))?;
    ensure(
        states
            .find(run.run_id(), collector, ListId::new(0))
            .is_none(),
        "state should be removed after collect_finish",
    )
}

#[test]
fn millis_since_epoch_returns_reasonable_value() -> Result<(), String> {
    let ms = millis_since_epoch().map_err(|e| format!("{e:?}"))?;
    ensure(
        ms > 946_684_800_000,
        format!("millis should be post-2000, got {ms}"),
    )?;
    ensure(
        ms < 32_503_680_000_000,
        format!("millis should be pre-3000, got {ms}"),
    )
}

// =========================================================================
// Additional coverage: state machine transitions, pagination boundaries,
// capacity enforcement, and collect lifecycle edge cases.
// =========================================================================

/// Full start -> page -> next -> next -> done lifecycle with page_size=1.
#[test]
fn collect_full_lifecycle_single_item_pages() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![
            SlotValue::I64(10),
            SlotValue::I64(20),
            SlotValue::I64(30),
            SlotValue::I64(40),
        ],
    );

    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;
    ensure(
        run.pc() == body,
        format!("expected pc={body:?}, got {:?}", run.pc()),
    )?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(10)]);

    collect_next(&mut run, &mut store, &mut states, collector, body, done)
        .map_err(|e| format!("collect_next 1: {e:?}"))?;
    ensure(run.pc() == body, "expected pc at body after next 1")?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(20)]);

    collect_next(&mut run, &mut store, &mut states, collector, body, done)
        .map_err(|e| format!("collect_next 2: {e:?}"))?;
    ensure(run.pc() == body, "expected pc at body after next 2")?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(30)]);

    collect_next(&mut run, &mut store, &mut states, collector, body, done)
        .map_err(|e| format!("collect_next 3: {e:?}"))?;
    ensure(run.pc() == body, "expected pc at body after next 3")?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(40)]);

    collect_next(&mut run, &mut store, &mut states, collector, body, done)
        .map_err(|e| format!("collect_next 4: {e:?}"))?;
    ensure(
        run.pc() == done,
        format!("expected pc={done:?} after exhaustion, got {:?}", run.pc()),
    )?;
    assert_slot_list_items(&run, &store, collector, &[]);
    Ok(())
}

/// Captured pagination extra hydrates a fresh state table and preserves CollectNext progress.
#[test]
fn collect_pagination_extra_round_trips_for_recovery() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;

    let extra = states
        .capture_extra(run.run_id(), collector)
        .map_err(|e| format!("capture: {e:?}"))?
        .ok_or("expected pagination extra")?;
    let mut recovered = fresh_states();
    recovered
        .hydrate_extra(run.run_id(), collector, &extra)
        .map_err(|e| format!("hydrate: {e:?}"))?;

    collect_next(&mut run, &mut store, &mut recovered, collector, body, done)
        .map_err(|e| format!("collect_next: {e:?}"))?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(20)]);
    Ok(())
}

/// Corrupt durable pagination extra is rejected instead of silently losing cursor state.
#[test]
fn collect_pagination_extra_rejects_corrupt_bytes() -> Result<(), String> {
    let mut states = fresh_states();
    let result = states.hydrate_extra(RunId::new(1), SlotIdx::new(1), &[255, 0, 7]);
    assert_invalid_workflow_reason(result, "collect pagination state decode failed");
    Ok(())
}

#[test]
fn collect_journal_extra_rejects_corrupt_bytes() -> Result<(), String> {
    let event = slot_written_extra(RunId::new(1), SlotIdx::new(1), vec![255, 0, 7]);
    let mut states = fresh_states();
    let result = states.hydrate_journal_events(&[event]);
    assert_invalid_workflow_reason(result, "collect pagination state decode failed");
    Ok(())
}

#[test]
fn collect_pagination_extra_recovered_journal_rejects_corrupt_bytes() -> Result<(), String> {
    let dir = tempfile::TempDir::new().map_err(|e| format!("tempdir: {e}"))?;
    let journal =
        vb_storage::FjallJournal::open(dir.path(), Some(vb_storage::FjallConfig::default()))
            .map_err(|e| format!("journal open: {e:?}"))?;
    let run = RunId::new(9);
    journal
        .append_strict(&slot_written_extra(run, SlotIdx::new(1), vec![255, 0, 7]))
        .map_err(|e| format!("append: {e:?}"))?;

    let mut tracker = ActionReplayTracker::new();
    let recovered =
        recover_full_journal(&journal, run, &mut tracker).map_err(|e| format!("recover: {e:?}"))?;
    let result = hydrate_collect_states_from_recovered_journal(&recovered);
    assert_invalid_workflow_reason(result.map(|_| ()), "collect pagination state decode failed");
    Ok(())
}

#[test]
fn collect_pagination_extra_recovered_journal_round_trips_and_resumes_next_page()
-> Result<(), String> {
    let dir = tempfile::TempDir::new().map_err(|e| format!("tempdir: {e}"))?;
    let journal =
        vb_storage::FjallJournal::open(dir.path(), Some(vb_storage::FjallConfig::default()))
            .map_err(|e| format!("journal open: {e:?}"))?;
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20), SlotValue::I64(30)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;
    let extra = states
        .capture_extra(run.run_id(), collector)
        .map_err(|e| format!("capture: {e:?}"))?
        .ok_or("expected pagination extra")?;
    journal
        .append_strict(&slot_written_extra(run.run_id(), collector, extra))
        .map_err(|e| format!("append: {e:?}"))?;

    let mut tracker = ActionReplayTracker::new();
    let recovered = recover_full_journal(&journal, run.run_id(), &mut tracker)
        .map_err(|e| format!("recover: {e:?}"))?;
    let mut hydrated = hydrate_collect_states_from_recovered_journal(&recovered)
        .map_err(|e| format!("hydrate: {e:?}"))?;
    let hydrated_state = hydrated
        .capture_state(run.run_id(), collector)
        .ok_or("expected hydrated pagination state")?;
    assert_eq!(hydrated_state.cursor, 1);
    assert_eq!(hydrated_state.page_size, 1);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(10)]);

    collect_next(&mut run, &mut store, &mut hydrated, collector, body, done)
        .map_err(|e| format!("collect_next: {e:?}"))?;

    assert_eq!(run.pc(), body);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(20)]);
    let resumed_state = hydrated
        .capture_state(run.run_id(), collector)
        .ok_or("expected resumed pagination state")?;
    assert_eq!(resumed_state.cursor, 2);
    assert_eq!(resumed_state.page_size, 1);
    Ok(())
}

/// Hydration must not accept extras from another run or collector slot.
#[test]
fn collect_pagination_extra_rejects_identity_mismatch() -> Result<(), String> {
    let mut run = fresh_frame();
    let collector = SlotIdx::new(1);
    let extra = captured_collect_extra(&mut run, collector)?;
    let mut recovered = fresh_states();
    let result = recovered.hydrate_extra(RunId::new(2), collector, &extra);
    assert_invalid_workflow_reason(result, "collect pagination state identity mismatch");
    Ok(())
}

#[test]
fn collect_journal_extra_rejects_identity_mismatch() -> Result<(), String> {
    let mut run = fresh_frame();
    let collector = SlotIdx::new(1);
    let extra = captured_collect_extra(&mut run, collector)?;
    let event = slot_written_extra(RunId::new(2), collector, extra);
    let mut recovered = fresh_states();
    let result = recovered.hydrate_journal_events(&[event]);
    assert_invalid_workflow_reason(result, "collect pagination state identity mismatch");
    Ok(())
}

#[test]
fn collect_pagination_extra_recovered_journal_rejects_identity_mismatch() -> Result<(), String> {
    let dir = tempfile::TempDir::new().map_err(|e| format!("tempdir: {e}"))?;
    let journal =
        vb_storage::FjallJournal::open(dir.path(), Some(vb_storage::FjallConfig::default()))
            .map_err(|e| format!("journal open: {e:?}"))?;
    let mut run = fresh_frame();
    let collector = SlotIdx::new(1);
    let extra = captured_collect_extra(&mut run, collector)?;
    let durable_run = RunId::new(2);
    journal
        .append_strict(&slot_written_extra(durable_run, collector, extra))
        .map_err(|e| format!("append: {e:?}"))?;

    let mut tracker = ActionReplayTracker::new();
    let recovered = recover_full_journal(&journal, durable_run, &mut tracker)
        .map_err(|e| format!("recover: {e:?}"))?;
    let result = hydrate_collect_states_from_recovered_journal(&recovered);
    assert_invalid_workflow_reason(
        result.map(|_| ()),
        "collect pagination state identity mismatch",
    );
    Ok(())
}

/// CollectFinish propagates list values to the output slot.
#[test]
fn collect_finish_propagates_list_to_output() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(3);

    let items: Box<[SlotValue]> = vec![SlotValue::I64(7), SlotValue::I64(8)].into_boxed_slice();
    let list_id = store.insert_list(items).map_err(|e| format!("{e:?}"))?;
    run.write_slot(collector, SlotValue::List(list_id))
        .map_err(|e| format!("{e:?}"))?;

    collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(next),
        StepIdx::ZERO,
    )
    .map_err(|e| format!("collect_finish: {e:?}"))?;

    match *run.read_slot(output).map_err(|e| format!("{e:?}"))? {
        SlotValue::List(id) => {
            ensure(id == list_id, "output list id should match collector")?;
        }
        other => return Err(format!("expected List, got {other:?}")),
    }
    ensure(
        run.pc() == next,
        format!("expected pc={next:?}, got {:?}", run.pc()),
    )?;
    ensure(
        states.find(run.run_id(), collector, list_id).is_none(),
        "state should be removed after finish",
    )
}

/// CollectPage on a non-empty list at an arbitrary step advances to body.
#[test]
fn collect_page_with_nonempty_collector_advances_to_body() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let body = StepIdx::new(5);

    list_in_slot(
        &mut run,
        &mut store,
        collector,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );

    let result = collect_page(
        &mut run,
        &mut store,
        &mut states,
        collector,
        body,
        StepIdx::new(6),
    );
    let signal = result.map_err(|e| format!("{e:?}"))?;
    ensure(
        signal == vb_core::EngineSignal::Continue,
        "expected Continue",
    )?;
    ensure(
        run.pc() == body,
        format!("expected pc={body:?}, got {:?}", run.pc()),
    )
}

/// Limit boundary: exact match succeeds, one over fails.
#[test]
fn collect_start_limit_boundary_exact_vs_one_over() -> Result<(), String> {
    let mut run1 = fresh_frame();
    let mut store1 = ValueStore::new();
    let mut states1 = fresh_states();
    let source = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    list_in_slot(
        &mut run1,
        &mut store1,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
    );
    let r1 = collect_start(
        &mut run1,
        &mut store1,
        &mut states1,
        source,
        5,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    ensure(r1.is_ok(), format!("exact limit should succeed: {r1:?}"))?;

    let mut run2 = fresh_frame();
    let mut store2 = ValueStore::new();
    let mut states2 = fresh_states();
    list_in_slot(
        &mut run2,
        &mut store2,
        source,
        vec![
            SlotValue::I64(1),
            SlotValue::I64(2),
            SlotValue::I64(3),
            SlotValue::I64(4),
            SlotValue::I64(5),
        ],
    );
    let r2 = collect_start(
        &mut run2,
        &mut store2,
        &mut states2,
        source,
        4,
        2,
        StepIdx::new(1),
        StepIdx::new(2),
        Some(output),
        None,
    );
    match r2 {
        Err(EngineError::CollectItemLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectItemLimitExceeded, got {other:?}")),
    }
}

/// CollectNext with expired time_limit returns CollectTimeLimitExceeded.
#[test]
fn collect_next_time_limit_exceeded_returns_error() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        body,
        done,
        Some(collector),
        Some(1),
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;

    // Corrupt start_millis to 0 to guarantee expiry
    let current_page = match *run.read_slot(collector).map_err(|e| format!("{e:?}"))? {
        SlotValue::List(id) => id,
        other => return Err(format!("expected List, got {other:?}")),
    };
    let state = states
        .find(run.run_id(), collector, current_page)
        .ok_or("state not found")?;
    states
        .upsert(CollectPaginationState {
            start_millis: 0,
            ..state
        })
        .map_err(|e| format!("{e:?}"))?;

    let result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    match result {
        Err(EngineError::CollectTimeLimitExceeded) => Ok(()),
        other => Err(format!("expected CollectTimeLimitExceeded, got {other:?}")),
    }
}

/// Two entries with different run IDs do not collide.
#[test]
fn collect_states_independent_entries_per_run() -> Result<(), String> {
    let mut states = CollectStates::new();
    let s1 = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 3,
        page_size: 5,
        item_count: 10,
        limit: 50,
        time_limit_ms: None,
        start_millis: 0,
    };
    let s2 = CollectPaginationState {
        run_id: RunId::new(2),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(11),
        current_page: ListId::new(21),
        cursor: 7,
        page_size: 5,
        item_count: 15,
        limit: 50,
        time_limit_ms: None,
        start_millis: 0,
    };
    states.upsert(s1).map_err(|e| format!("{e:?}"))?;
    states.upsert(s2).map_err(|e| format!("{e:?}"))?;
    let f1 = states
        .find(RunId::new(1), SlotIdx::new(0), ListId::new(20))
        .ok_or("run 1 state missing")?;
    let f2 = states
        .find(RunId::new(2), SlotIdx::new(0), ListId::new(21))
        .ok_or("run 2 state missing")?;
    ensure(
        f1.cursor == 3,
        format!("cursor 1 should be 3, got {}", f1.cursor),
    )?;
    ensure(
        f2.cursor == 7,
        format!("cursor 2 should be 7, got {}", f2.cursor),
    )
}

/// Remove on a non-existent key is a silent no-op.
#[test]
fn collect_states_remove_nonexistent_is_noop() -> Result<(), String> {
    let mut states = CollectStates::new();
    states.remove(RunId::new(999), SlotIdx::new(99));
    ensure(
        states.entries.is_empty(),
        "removing nonexistent key should not add entries",
    )
}

/// copy_page_range with start at end of items returns empty page.
#[test]
fn copy_page_range_at_end_returns_empty() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![SlotValue::I64(1), SlotValue::I64(2)].into_boxed_slice();
    let page = copy_page_range(&items, 2, 5).map_err(|e| format!("{e:?}"))?;
    ensure(
        page.is_empty(),
        format!("expected empty page, got {} items", page.len()),
    )
}

/// copy_page_range with start beyond length returns error.
#[test]
fn copy_page_range_start_beyond_length_returns_error() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![SlotValue::I64(1)].into_boxed_slice();
    let result = copy_page_range(&items, 5, 1);
    match result {
        Err(EngineError::InternalInvariantViolation { reason }) => ensure(
            reason == "collect cursor beyond item count",
            format!("unexpected: {reason}"),
        ),
        other => Err(format!(
            "expected InternalInvariantViolation, got {other:?}"
        )),
    }
}

/// copy_prefix with zero-length items returns empty page.
#[test]
fn copy_prefix_empty_items_returns_empty() -> Result<(), String> {
    let items: Box<[SlotValue]> = vec![].into_boxed_slice();
    let page = copy_prefix(&items, 10).map_err(|e| format!("{e:?}"))?;
    ensure(page.is_empty(), "empty items should produce empty prefix")
}

/// CollectFinish propagates taint from collector to output.
#[test]
fn collect_finish_propagates_taint() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut states = fresh_states();
    let collector = SlotIdx::new(0);
    let output = SlotIdx::new(1);
    let next = StepIdx::new(3);

    run.write_slot_with_taint(collector, SlotValue::I64(42), Taint::Secret)
        .map_err(|e| format!("{e:?}"))?;
    collect_finish(
        &mut run,
        &mut states,
        collector,
        Some(output),
        Some(next),
        StepIdx::ZERO,
    )
    .map_err(|e| format!("{e:?}"))?;

    let taint = run.read_taint(output).map_err(|e| format!("{e:?}"))?;
    ensure(
        taint == Taint::Secret,
        format!("expected Secret, got {taint:?}"),
    )
}

/// CollectNext where cursor exactly equals item_count goes to done.
#[test]
fn collect_next_cursor_at_item_count_goes_to_done() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        2,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("{e:?}"))?;

    let result = collect_next(&mut run, &mut store, &mut states, collector, body, done);
    let signal = result.map_err(|e| format!("{e:?}"))?;
    ensure(
        signal == vb_core::EngineSignal::Continue,
        "expected Continue",
    )?;
    ensure(
        run.pc() == done,
        format!("expected pc={done:?}, got {:?}", run.pc()),
    )
}

/// validate_collect_state accepts when page_size == limit.
#[test]
fn validate_collect_state_accepts_page_size_at_limit() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 10,
        item_count: 5,
        limit: 10,
        time_limit_ms: None,
        start_millis: 0,
    };
    validate_collect_state(&state, 5).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

/// validate_collect_state accepts when item_count == limit.
#[test]
fn validate_collect_state_accepts_item_count_at_limit() -> Result<(), String> {
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(0),
        current_page: ListId::new(0),
        cursor: 0,
        page_size: 5,
        item_count: 10,
        limit: 10,
        time_limit_ms: None,
        start_millis: 0,
    };
    validate_collect_state(&state, 10).map_err(|e| format!("{e:?}"))?;
    Ok(())
}

/// checked_add_usize with zero operands succeeds.
#[test]
fn checked_add_usize_zero_plus_zero() -> Result<(), String> {
    let result = checked_add_usize(0, 0, "zero test").map_err(|e| format!("{e:?}"))?;
    ensure(result == 0, format!("expected 0, got {result}"))
}

/// CollectStates find with matching key but wrong slot returns None.
#[test]
fn collect_states_find_returns_none_for_wrong_slot() -> Result<(), String> {
    let mut states = CollectStates::new();
    let state = CollectPaginationState {
        run_id: RunId::new(1),
        collector_slot: SlotIdx::new(0),
        source: ListId::new(10),
        current_page: ListId::new(20),
        cursor: 0,
        page_size: 10,
        item_count: 10,
        limit: 100,
        time_limit_ms: None,
        start_millis: 0,
    };
    states.upsert(state).map_err(|e| format!("{e:?}"))?;
    let found = states.find(RunId::new(1), SlotIdx::new(5), ListId::new(20));
    ensure(
        found.is_none(),
        "find should return None for wrong collector_slot",
    )
}

/// Multiple rounds of start -> next produce independent state.
#[test]
fn collect_repeated_start_next_cycles() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        3,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("start 1: {e:?}"))?;

    collect_next(&mut run, &mut store, &mut states, collector, body, done)
        .map_err(|e| format!("next 1: {e:?}"))?;
    ensure(run.pc() == done, "first cycle should reach done")?;

    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        100,
        1,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("start 2: {e:?}"))?;
    ensure(run.pc() == body, "second cycle should start at body")?;
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(10)]);
    Ok(())
}

#[test]
fn collect_first_page_preserves_non_monotonic_source_order() -> Result<(), String> {
    let scenario = non_monotonic_collect_scenario()?;

    assert_eq!(scenario.run.pc(), scenario.body);
    scenario.assert_collector_items(&[SlotValue::I64(30), SlotValue::I64(10)])?;
    scenario.assert_current_cursor(2)?;
    Ok(())
}

#[test]
fn collect_second_page_preserves_non_monotonic_source_order() -> Result<(), String> {
    let mut scenario = non_monotonic_collect_scenario()?;

    assert_eq!(scenario.next()?, vb_core::EngineSignal::Continue);
    assert_eq!(scenario.run.pc(), scenario.body);
    scenario.assert_collector_items(&[SlotValue::I64(20), SlotValue::I64(40)])?;
    scenario.assert_current_cursor(4)?;
    Ok(())
}

#[test]
fn collect_third_page_preserves_non_monotonic_source_order() -> Result<(), String> {
    let mut scenario = non_monotonic_collect_scenario()?;

    scenario.next()?;
    assert_eq!(scenario.next()?, vb_core::EngineSignal::Continue);
    assert_eq!(scenario.run.pc(), scenario.body);
    scenario.assert_collector_items(&[SlotValue::I64(15)])?;
    scenario.assert_current_cursor(5)?;
    Ok(())
}

#[test]
fn collect_next_rejects_duplicate_first_page_response_after_cursor_advanced() -> Result<(), String>
{
    duplicate_page_rejection_scenario()?;
    Ok(())
}

#[test]
fn duplicate_first_page_rejection_preserves_advanced_state() -> Result<(), String> {
    let (scenario, advanced_page) = duplicate_page_rejection_scenario()?;

    scenario.assert_live_cursor(advanced_page, 4)?;
    Ok(())
}

#[test]
fn collect_next_rejects_stale_completion_page() -> Result<(), String> {
    stale_page_rejection_scenario()?;
    Ok(())
}

#[test]
fn stale_completion_page_rejection_preserves_live_state() -> Result<(), String> {
    let (scenario, live_page) = stale_page_rejection_scenario()?;

    scenario.assert_live_cursor(live_page, 2)?;
    Ok(())
}

#[test]
fn collect_start_enforces_page_size_bounds_before_allocating_page() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2)],
    );

    assert_eq!(
        collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            2,
            3,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(collector),
            None,
        ),
        Err(EngineError::CollectPageLimitExceeded)
    );
    assert_eq!(
        run.read_slot(collector),
        Err(EngineError::SlotUninitialized { slot: collector })
    );
    assert_eq!(states.capture_state(run.run_id(), collector), None);
    Ok(())
}

#[test]
fn collect_start_enforces_fanout_item_limit_at_exact_boundary() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );

    assert_eq!(
        collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            3,
            2,
            body,
            done,
            Some(collector),
            None,
        ),
        Ok(vb_core::EngineSignal::Continue)
    );
    assert_eq!(run.pc(), body);
    assert_slot_list_items(
        &run,
        &store,
        collector,
        &[SlotValue::I64(1), SlotValue::I64(2)],
    );
    assert_eq!(
        collect_state_for_current_page(&states, &run, collector)?.item_count,
        3
    );
    assert_eq!(
        collect_state_for_current_page(&states, &run, collector)?.limit,
        3
    );
    Ok(())
}

#[test]
fn collect_start_rejects_fanout_one_over_limit_without_collector_state() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );

    assert_eq!(
        collect_start(
            &mut run,
            &mut store,
            &mut states,
            source,
            2,
            2,
            StepIdx::new(1),
            StepIdx::new(2),
            Some(collector),
            None,
        ),
        Err(EngineError::CollectItemLimitExceeded)
    );
    assert_eq!(
        run.read_slot(collector),
        Err(EngineError::SlotUninitialized { slot: collector })
    );
    assert_eq!(states.capture_state(run.run_id(), collector), None);
    Ok(())
}

#[test]
fn collect_next_honors_value_store_arena_cap_without_advancing_cursor() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::with_max_slots(2);
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(
        &mut run,
        &mut store,
        source,
        vec![SlotValue::I64(1), SlotValue::I64(2), SlotValue::I64(3)],
    );
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        3,
        1,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;
    let first_page = slot_list_id(&run, collector)?;

    assert_eq!(
        collect_next(&mut run, &mut store, &mut states, collector, body, done),
        Err(EngineError::BudgetExceeded {
            budget: "max_slots",
            limit: 2,
        })
    );
    assert_eq!(slot_list_id(&run, collector)?, first_page);
    assert_slot_list_items(&run, &store, collector, &[SlotValue::I64(1)]);
    assert_eq!(
        states
            .find(run.run_id(), collector, first_page)
            .ok_or("first-page state missing after arena-cap rejection".to_owned())?
            .cursor,
        1
    );
    Ok(())
}

#[test]
fn collect_next_writes_empty_page_and_removes_state_after_last_item() -> Result<(), String> {
    let mut run = fresh_frame();
    let mut store = ValueStore::new();
    let mut states = fresh_states();
    let source = SlotIdx::new(0);
    let collector = SlotIdx::new(1);
    let body = StepIdx::new(1);
    let done = StepIdx::new(2);
    list_in_slot(&mut run, &mut store, source, vec![SlotValue::I64(99)]);
    collect_start(
        &mut run,
        &mut store,
        &mut states,
        source,
        1,
        1,
        body,
        done,
        Some(collector),
        None,
    )
    .map_err(|e| format!("collect_start: {e:?}"))?;
    let first_page = slot_list_id(&run, collector)?;

    assert_eq!(
        collect_next(&mut run, &mut store, &mut states, collector, body, done),
        Ok(vb_core::EngineSignal::Continue)
    );
    assert_eq!(run.pc(), done);
    assert_slot_list_items(&run, &store, collector, &[]);
    assert_eq!(states.find(run.run_id(), collector, first_page), None);
    assert_eq!(states.capture_state(run.run_id(), collector), None);
    Ok(())
}
