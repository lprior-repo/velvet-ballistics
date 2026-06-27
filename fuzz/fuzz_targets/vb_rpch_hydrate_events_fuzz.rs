#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx};
use vb_storage::recovery::hydrate::hydrate_events_preconditions;
use vb_storage::{EventSeq, JournalEvent};

const EVENT_CHUNK: usize = 4;
const MAX_EVENTS: usize = 16;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let count = (usize::from(data[0]) % MAX_EVENTS) + 1;
    let mut events: Vec<JournalEvent> = Vec::with_capacity(count);
    let mut idx = 1usize;
    for _ in 0..count {
        if idx >= data.len() {
            events.push(event_from_bytes(&[]));
            continue;
        }
        let end = idx.saturating_add(EVENT_CHUNK);
        let chunk = data.get(idx..end).unwrap_or(&[]);
        events.push(event_from_bytes(chunk));
        idx = end;
    }

    assert_eq!(
        hydrate_events_preconditions(&events),
        !events.is_empty()
    );
});

fn event_from_bytes(bytes: &[u8]) -> JournalEvent {
    let run_byte = bytes.first().copied().unwrap_or(0);
    let seq_byte = bytes.get(1).copied().unwrap_or(0);
    let slot_byte = bytes.get(2).copied().unwrap_or(0);
    let attempt_byte = bytes.get(3).copied().unwrap_or(1);
    JournalEvent::RunFinished {
        run: RunId::new(u64::from(run_byte).saturating_add(1)),
        seq: EventSeq::new(u64::from(seq_byte).saturating_add(1)),
        result: SlotIdx::new(u16::from(slot_byte)),
        attempt: u16::from(attempt_byte).max(1),
    }
}