#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::errors::CollectExtraHydrationFailureKind;
use vb_core::{EngineError, EventSeq as CoreEventSeq, ListId, RunId, SlotIdx, SlotValue, Taint};
use vb_runtime::primitives::collect::{
    CollectPaginationState, CollectStates, hydrate_collect_states_from_recovered_journal,
};
use vb_storage::{EventSeq, JournalEvent};

const MODE_ENVELOPE: u8 = 0;
const MODE_LEGACY: u8 = 1;
const MODE_ENVELOPE_WITHOUT_FRAME_EXTRA: u8 = 2;
const MODE_CORRUPT_ENVELOPE: u8 = 3;
const MODE_EMPTY_LEGACY_EXTRA: u8 = 4;
const MODE_RUN_MISMATCH: u8 = 5;
const MODE_SLOT_MISMATCH: u8 = 6;
const MODE_PAGE_MISMATCH: u8 = 7;

fuzz_target!(|data: &[u8]| {
    let case = CollectExtraCase::from_data(data);
    let Some(event) = case.event() else {
        return;
    };
    let result = hydrate_collect_states_from_recovered_journal(&[event]);
    case.assert_result(result);
});

#[derive(Clone, Copy)]
struct CollectExtraCase {
    mode: u8,
    run: RunId,
    slot: SlotIdx,
    seq: EventSeq,
    state: CollectPaginationState,
}

impl CollectExtraCase {
    fn from_data(data: &[u8]) -> Self {
        let mode = data.first().copied().unwrap_or(MODE_ENVELOPE) % 8;
        let run = RunId::new(u64::from(byte_at(data, 1)).saturating_add(1));
        let slot = SlotIdx::new(u16::from(byte_at(data, 2)));
        let current_page = ListId::new(u32::from(byte_at(data, 3)).saturating_add(1));
        let source = ListId::new(u32::from(byte_at(data, 4)).saturating_add(1));
        let cursor = usize::from(byte_at(data, 5) % 8);
        let page_size = usize::from(byte_at(data, 6) % 8).saturating_add(1);
        let item_count = cursor.saturating_add(page_size);
        let limit = item_count.saturating_add(usize::from(byte_at(data, 7) % 8));
        let state = CollectPaginationState {
            run_id: run,
            collector_slot: slot,
            source,
            current_page,
            cursor,
            page_size,
            item_count,
            limit,
            time_limit_ms: None,
            start_millis: u64::from(byte_at(data, 8)),
        };
        Self {
            mode,
            run,
            slot,
            seq: EventSeq::new(u64::from(byte_at(data, 9))),
            state,
        }
    }

    fn event(self) -> Option<JournalEvent> {
        let event_page = if self.mode == MODE_PAGE_MISMATCH {
            next_list(self.state.current_page)
        } else {
            self.state.current_page
        };
        let value = match postcard::to_allocvec(&SlotValue::List(event_page)) {
            Ok(encoded) => encoded,
            Err(_) => return None,
        };
        Some(JournalEvent::SlotWrittenEvent {
            run: self.run,
            seq: self.seq,
            slot: self.slot,
            value: Some(value),
            extra: Some(self.extra()?),
            attempt: 1,
        })
    }

    fn extra(self) -> Option<Vec<u8>> {
        let frame_state = self.frame_state_for_mode();
        let frame_extra = match postcard::to_allocvec(&frame_state) {
            Ok(encoded) => encoded,
            Err(_) => return None,
        };
        match self.mode {
            MODE_ENVELOPE | MODE_RUN_MISMATCH | MODE_SLOT_MISMATCH | MODE_PAGE_MISMATCH => {
                vb_storage::encode_slot_written_extra(Taint::Clean, Some(frame_extra)).ok()
            }
            MODE_LEGACY => Some(frame_extra),
            MODE_ENVELOPE_WITHOUT_FRAME_EXTRA => {
                vb_storage::encode_slot_written_extra(Taint::Clean, None).ok()
            }
            MODE_CORRUPT_ENVELOPE => Some(corrupt_envelope_extra()),
            MODE_EMPTY_LEGACY_EXTRA => Some(Vec::new()),
            _ => None,
        }
    }

    fn frame_state_for_mode(self) -> CollectPaginationState {
        match self.mode {
            MODE_RUN_MISMATCH => CollectPaginationState {
                run_id: next_run(self.run),
                ..self.state
            },
            MODE_SLOT_MISMATCH => CollectPaginationState {
                collector_slot: next_slot(self.slot),
                ..self.state
            },
            MODE_ENVELOPE
            | MODE_LEGACY
            | MODE_ENVELOPE_WITHOUT_FRAME_EXTRA
            | MODE_CORRUPT_ENVELOPE
            | MODE_EMPTY_LEGACY_EXTRA
            | MODE_PAGE_MISMATCH => self.state,
            _ => self.state,
        }
    }

    fn assert_result(self, result: Result<CollectStates, EngineError>) {
        match self.expected() {
            ExpectedCollectHydration::StateHydrated => assert_state_hydrated(result, self),
            ExpectedCollectHydration::NoState => assert_no_state(result, self),
            ExpectedCollectHydration::Failure(kind) => assert_collect_failure(result, self, kind),
        }
    }

    fn expected(self) -> ExpectedCollectHydration {
        match self.mode {
            MODE_ENVELOPE | MODE_LEGACY => ExpectedCollectHydration::StateHydrated,
            MODE_ENVELOPE_WITHOUT_FRAME_EXTRA => ExpectedCollectHydration::NoState,
            MODE_CORRUPT_ENVELOPE => {
                ExpectedCollectHydration::Failure(CollectExtraHydrationFailureKind::DecodeFailed)
            }
            MODE_EMPTY_LEGACY_EXTRA => {
                ExpectedCollectHydration::Failure(CollectExtraHydrationFailureKind::EmptyExtra)
            }
            MODE_RUN_MISMATCH => {
                ExpectedCollectHydration::Failure(CollectExtraHydrationFailureKind::RunMismatch {
                    expected: self.run,
                    actual: next_run(self.run),
                })
            }
            MODE_SLOT_MISMATCH => {
                ExpectedCollectHydration::Failure(CollectExtraHydrationFailureKind::SlotMismatch {
                    expected: self.slot,
                    actual: next_slot(self.slot),
                })
            }
            MODE_PAGE_MISMATCH => ExpectedCollectHydration::Failure(
                CollectExtraHydrationFailureKind::CurrentPageMismatch {
                    expected: next_list(self.state.current_page),
                    actual: self.state.current_page,
                },
            ),
            _ => ExpectedCollectHydration::StateHydrated,
        }
    }
}

enum ExpectedCollectHydration {
    StateHydrated,
    NoState,
    Failure(CollectExtraHydrationFailureKind),
}

fn byte_at(data: &[u8], index: usize) -> u8 {
    data.get(index).copied().unwrap_or(0)
}

fn next_run(run: RunId) -> RunId {
    match run.get().checked_add(1) {
        Some(value) => RunId::new(value),
        None => RunId::new(1),
    }
}

const fn next_slot(slot: SlotIdx) -> SlotIdx {
    SlotIdx::new(slot.get().saturating_add(1))
}

const fn next_list(list: ListId) -> ListId {
    ListId::new(list.get().saturating_add(1))
}

fn corrupt_envelope_extra() -> Vec<u8> {
    let mut extra = Vec::new();
    extra.extend_from_slice(vb_storage::SLOT_WRITTEN_EXTRA_PREFIX);
    extra.push(0xFF);
    extra
}

fn assert_state_hydrated(result: Result<CollectStates, EngineError>, case: CollectExtraCase) {
    assert!(result.is_ok(), "valid collect extra must hydrate");
    let Ok(states) = result else {
        return;
    };
    assert_eq!(
        states.capture_state(case.run, case.slot),
        Some(case.state),
        "hydrated collect state must match encoded frame extra"
    );
}

fn assert_no_state(result: Result<CollectStates, EngineError>, case: CollectExtraCase) {
    assert!(
        result.is_ok(),
        "envelope without frame_extra must be accepted as no collect state"
    );
    let Ok(states) = result else {
        return;
    };
    assert_eq!(
        states.capture_state(case.run, case.slot),
        None,
        "envelope without frame_extra must not hydrate collect state"
    );
}

fn assert_collect_failure(
    result: Result<CollectStates, EngineError>,
    case: CollectExtraCase,
    expected_kind: CollectExtraHydrationFailureKind,
) {
    assert!(
        matches!(
            result,
            Err(EngineError::CollectExtraHydrationFailed {
                kind,
                run_id,
                collector_slot,
                event_seq,
            }) if kind == expected_kind
                && run_id == case.run
                && collector_slot == case.slot
                && event_seq == Some(CoreEventSeq::new(case.seq.get()))
        ),
        "collect hydration failure must preserve exact typed failure kind and context"
    );
}
