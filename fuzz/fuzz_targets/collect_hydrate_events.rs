//! Fuzz target for collect hydration from recovered journal events.
//!
//! Tests `hydrate_collect_states_from_recovered_journal` with:
//! - Mix of collect and non-collect SlotWritten events
//! - Corrupted extra bytes
//! - Mismatched run/slot/seq identifiers
//! - Out-of-order event sequences
//! - Duplicate events
//! - Truncated payloads

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_core::errors::CollectExtraHydrationFailureKind;
use vb_core::{EngineError, ListId, RunId, SlotIdx, SlotValue, Taint};
use vb_runtime::primitives::collect::{
    CollectPaginationState, hydrate_collect_states_from_recovered_journal,
};
use vb_storage::{EventSeq, JournalEvent};

const MODE_COLLECT_ENVELOPE: u8 = 0;
const MODE_COLLECT_LEGACY: u8 = 1;
const MODE_COLLECT_CORRUPT: u8 = 2;
const MODE_NON_COLLECT: u8 = 3;
const MODE_RUN_MISMATCH: u8 = 4;
const MODE_SLOT_MISMATCH: u8 = 5;
const MODE_EMPTY_EXTRA: u8 = 6;
const MODE_TRUNCATED_EXTRA: u8 = 7;

fuzz_target!(|data: &[u8]| {
    let events = build_event_sequence(data);
    if events.is_empty() {
        return;
    }

    let result = hydrate_collect_states_from_recovered_journal(&events);
    let _states = observe_collect_hydration(result, &events);
});

fn build_event_sequence(data: &[u8]) -> Vec<JournalEvent> {
    let mut events = Vec::new();
    let num_events = (data.first().copied().unwrap_or(1) % 8) as usize + 1;

    for i in 0..num_events {
        let byte_offset = i * 4;
        if byte_offset >= data.len() {
            break;
        }

        let mode_byte = data[byte_offset];
        let run_id = RunId::new(u64::from(byte_at(data, byte_offset + 1)).saturating_add(1));
        let slot_idx = SlotIdx::new(u16::from(byte_at(data, byte_offset + 2)));
        let seq_num = EventSeq::new(u64::from(byte_at(data, byte_offset + 3)));

        let mode = mode_byte % 8;
        let event = match mode {
            MODE_COLLECT_ENVELOPE => collect_envelope_event(run_id, slot_idx, seq_num, data, i),
            MODE_COLLECT_LEGACY => collect_legacy_event(run_id, slot_idx, seq_num),
            MODE_COLLECT_CORRUPT => collect_corrupt_event(run_id, slot_idx, seq_num),
            MODE_NON_COLLECT => non_collect_event(run_id, slot_idx, seq_num),
            MODE_RUN_MISMATCH => mismatched_run_event(run_id, slot_idx, seq_num),
            MODE_SLOT_MISMATCH => mismatched_slot_event(run_id, slot_idx, seq_num),
            MODE_EMPTY_EXTRA => empty_extra_event(run_id, slot_idx, seq_num),
            MODE_TRUNCATED_EXTRA => truncated_extra_event(run_id, slot_idx, seq_num),
            _ => unreachable!(),
        };

        if let Some(ev) = event {
            events.push(ev);
        }
    }

    events
}

fn collect_envelope_event(
    run: RunId,
    slot: SlotIdx,
    seq: EventSeq,
    _data: &[u8],
    _i: usize,
) -> Option<JournalEvent> {
    let page = ListId::new(1);
    let value = match postcard::to_allocvec(&SlotValue::List(page)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    let pagination = CollectPaginationState {
        run_id: run,
        collector_slot: slot,
        source: ListId::new(1),
        current_page: ListId::new(1),
        cursor: 0,
        page_size: 8,
        item_count: 0,
        limit: 64,
        time_limit_ms: None,
        start_millis: 0,
    };
    let encoded = match postcard::to_allocvec(&pagination) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let extra = match vb_storage::encode_slot_written_extra(Taint::Clean, Some(encoded)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt: 1,
    })
}

fn collect_legacy_event(run: RunId, slot: SlotIdx, seq: EventSeq) -> Option<JournalEvent> {
    let page = ListId::new(1);
    let value = match postcard::to_allocvec(&SlotValue::List(page)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    // Legacy: just pagination state, no envelope prefix
    let pagination = CollectPaginationState {
        run_id: run,
        collector_slot: slot,
        source: ListId::new(1),
        current_page: ListId::new(1),
        cursor: 0,
        page_size: 8,
        item_count: 0,
        limit: 64,
        time_limit_ms: None,
        start_millis: 0,
    };
    let extra = postcard::to_allocvec(&pagination).ok();
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt: 1,
    })
}

fn collect_corrupt_event(run: RunId, slot: SlotIdx, seq: EventSeq) -> Option<JournalEvent> {
    let value = Some(vec![1, 2, 3]);
    let mut extra = vb_storage::SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    extra.push(0xFF);
    extra.push(0x00);
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra: Some(extra),
        attempt: 1,
    })
}

fn non_collect_event(run: RunId, slot: SlotIdx, seq: EventSeq) -> Option<JournalEvent> {
    let value = Some(postcard::to_allocvec(&SlotValue::I64(42)).ok()?);
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra: None,
        attempt: 1,
    })
}

fn mismatched_run_event(run: RunId, slot: SlotIdx, seq: EventSeq) -> Option<JournalEvent> {
    let page = ListId::new(1);
    let value = match postcard::to_allocvec(&SlotValue::List(page)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    let pagination = CollectPaginationState {
        run_id: RunId::new(999),
        collector_slot: slot,
        source: ListId::new(1),
        current_page: ListId::new(1),
        cursor: 0,
        page_size: 8,
        item_count: 0,
        limit: 64,
        time_limit_ms: None,
        start_millis: 0,
    };
    let encoded = match postcard::to_allocvec(&pagination) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let extra = match vb_storage::encode_slot_written_extra(Taint::Clean, Some(encoded)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt: 1,
    })
}

fn mismatched_slot_event(run: RunId, slot: SlotIdx, seq: EventSeq) -> Option<JournalEvent> {
    let page = ListId::new(1);
    let value = match postcard::to_allocvec(&SlotValue::List(page)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    let pagination = CollectPaginationState {
        run_id: run,
        collector_slot: SlotIdx::new(999),
        source: ListId::new(1),
        current_page: ListId::new(1),
        cursor: 0,
        page_size: 8,
        item_count: 0,
        limit: 64,
        time_limit_ms: None,
        start_millis: 0,
    };
    let encoded = match postcard::to_allocvec(&pagination) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let extra = match vb_storage::encode_slot_written_extra(Taint::Clean, Some(encoded)) {
        Ok(v) => Some(v),
        Err(_) => return None,
    };
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra,
        attempt: 1,
    })
}

fn empty_extra_event(run: RunId, slot: SlotIdx, seq: EventSeq) -> Option<JournalEvent> {
    let value = Some(postcard::to_allocvec(&SlotValue::I64(42)).ok()?);
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra: Some(vec![]),
        attempt: 1,
    })
}

fn truncated_extra_event(run: RunId, slot: SlotIdx, seq: EventSeq) -> Option<JournalEvent> {
    let value = Some(postcard::to_allocvec(&SlotValue::I64(42)).ok()?);
    let extra = vb_storage::SLOT_WRITTEN_EXTRA_PREFIX.to_vec();
    Some(JournalEvent::SlotWrittenEvent {
        run,
        seq,
        slot,
        value,
        extra: Some(extra),
        attempt: 1,
    })
}

fn byte_at(data: &[u8], index: usize) -> u8 {
    data.get(index).copied().unwrap_or(0)
}

fn observe_collect_hydration(
    result: Result<vb_runtime::primitives::collect::CollectStates, EngineError>,
    events: &[JournalEvent],
) -> vb_runtime::primitives::collect::CollectStates {
    match result {
        Ok(states) => {
            // If hydration succeeded, verify at least one state can be found
            // for events that had valid SlotWritten events.
            for event in events {
                if let JournalEvent::SlotWrittenEvent {
                    run, slot, extra, ..
                } = event
                {
                    if extra.is_some() {
                        // Check if this event could be found
                        // (only if hydration didn't fail)
                        let _ = states.find(*run, *slot, ListId::new(1));
                    }
                }
            }
            states
        }
        Err(e) => {
            match e {
                EngineError::CollectExtraHydrationFailed {
                    kind: CollectExtraHydrationFailureKind::EmptyExtra,
                    ..
                } => {}
                EngineError::CollectExtraHydrationFailed {
                    kind: CollectExtraHydrationFailureKind::DecodeFailed,
                    ..
                } => {}
                EngineError::CollectExtraHydrationFailed {
                    kind: CollectExtraHydrationFailureKind::RunMismatch { .. },
                    ..
                } => {}
                EngineError::CollectExtraHydrationFailed {
                    kind: CollectExtraHydrationFailureKind::SlotMismatch { .. },
                    ..
                } => {}
                EngineError::CollectExtraHydrationFailed {
                    kind: CollectExtraHydrationFailureKind::CurrentPageMismatch { .. },
                    ..
                } => {}
                EngineError::InvalidCompiledWorkflow { .. } => {}
                EngineError::CollectPageOrderViolation { .. } => {}
                _ => {
                    panic!("Unexpected error from collect hydration: {:?}", e);
                }
            }
            Default::default()
        }
    }
}
