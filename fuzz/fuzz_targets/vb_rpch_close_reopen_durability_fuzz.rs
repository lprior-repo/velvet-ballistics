#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, StepIdx};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

const MAX_BATCH_SIZE: usize = 12;

fuzz_target!(|data: &[u8]| {
    let run = RunId::new(1);
    let mode = data.first().copied().unwrap_or(0);
    let scenario = mode % 4;

    match scenario {
        0 => durability_close_reopen(run, data),
        1 => durability_partial_write(run, data),
        2 => durability_multiple_closes(run, data),
        _ => durability_concurrent_style(run, data),
    }
});

#[path = "vb_rpch_close_reopen_durability_fuzz/scenarios.rs"]
mod scenarios;
use scenarios::*;

fn bounded_count(data: &[u8], min: usize, max: usize) -> usize {
    if data.is_empty() {
        return min;
    }
    let byte = data[0];
    if max <= min {
        return min;
    }
    let span = max - min;
    min + (byte as usize) % span
}

fn build_events(run: RunId, count: usize) -> Option<Vec<JournalEvent>> {
    let mut events = Vec::with_capacity(count);
    for i in 0..count {
        let seq = EventSeq::new(i as u64);
        let step = StepIdx::new(i as u16);
        events.push(JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt: 1,
        });
    }
    Some(events)
}

fn build_events_from(run: RunId, start_seq: u64, count: usize) -> Option<Vec<JournalEvent>> {
    let mut events = Vec::with_capacity(count);
    for i in 0..count {
        let seq = EventSeq::new(start_seq + i as u64);
        let step = StepIdx::new(i as u16);
        events.push(JournalEvent::StepStarted {
            run,
            seq,
            step,
            attempt: 1,
        });
    }
    Some(events)
}
