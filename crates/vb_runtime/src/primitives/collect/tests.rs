use super::*;
use crate::primitives::collect::CollectStates;
use crate::test_harness::list_in_slot;
use std::path::PathBuf;
use vb_core::EventSeq as CoreEventSeq;
use vb_core::errors::{CollectExtraHydrationFailureKind, EngineError};
use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_storage::recovery::{ActionReplayTracker, recover_full_journal};
use vb_storage::{EventSeq, JournalEvent};

fn fresh_frame() -> RunFrame {
    crate::test_harness::fresh_frame(8, 8)
}

fn fresh_states() -> CollectStates {
    CollectStates::new()
}

fn disk_backed_temp_dir(prefix: &str) -> Result<tempfile::TempDir, String> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&base).map_err(|e| format!("target workdir: {e}"))?;
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(base)
        .map_err(|e| format!("tempdir: {e}"))
}

fn assert_invalid_workflow_reason(result: Result<(), EngineError>, expected: &'static str) {
    match result {
        Err(EngineError::InvalidCompiledWorkflow { reason }) => assert_eq!(reason, expected),
        Err(EngineError::CollectExtraHydrationFailed { kind, .. })
            if expected == "collect pagination state decode failed" =>
        {
            assert_eq!(kind, CollectExtraHydrationFailureKind::DecodeFailed);
        }
        Err(EngineError::CollectExtraHydrationFailed { kind, .. })
            if expected == "collect pagination state identity mismatch" =>
        {
            assert!(matches!(
                kind,
                CollectExtraHydrationFailureKind::RunMismatch { .. }
                    | CollectExtraHydrationFailureKind::SlotMismatch { .. }
            ));
        }
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
    // Use 2 items with page_size=1 so state is preserved after first page
    list_in_slot(
        &mut *run,
        &mut store,
        source,
        vec![SlotValue::I64(10), SlotValue::I64(20)],
    );
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
    match *run.read_slot(slot).expect("read must succeed") {
        SlotValue::List(id) => {
            let items = store.list(id).expect("list read must succeed");
            assert_eq!(items, expected);
        }
        other => {
            assert!(
                matches!(other, SlotValue::List(_)),
                "expected List, got {other:?}"
            );
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
        let result = collect_next(
            &mut self.run,
            &mut self.store,
            &mut self.states,
            self.collector,
            self.body,
            self.done,
        );
        assert!(matches!(
            result,
            Err(EngineError::InvalidCompiledWorkflow {
                reason: "collect pagination state missing"
            }) | Err(EngineError::CollectPageOrderViolation { .. })
        ));
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

include!("tests/lifecycle_start.rs");

include!("tests/boundaries.rs");

include!("tests/state_helpers.rs");

include!("tests/lifecycle_recovery.rs");

include!("tests/validation_ordering.rs");

include!("tests/hydration.rs");
