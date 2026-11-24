#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, StepIdx};
use vb_storage::{EventSeq, FjallJournal, JournalEvent};

const MAX_BATCH_SIZE: u16 = 12;

fuzz_target!(|data: &[u8]| {
    let run = RunId::new(1);
    let mode = data.first().map_or(0, |&byte| byte);
    let scenario = mode % 4;

    let dir = tempfile::tempdir();
    assert!(dir.is_ok(), "durability scenario {scenario}: {dir:?}");
    let Ok(dir) = dir else {
        return;
    };

    match scenario {
        0 => durability_close_reopen(run, data, dir.path()),
        1 => durability_partial_write(run, data, dir.path()),
        2 => durability_multiple_closes(run, data, dir.path()),
        _ => durability_concurrent_style(run, data, dir.path()),
    }
});

#[path = "vb_rpch_close_reopen_durability_fuzz/scenarios.rs"]
mod scenarios;
use scenarios::*;

fn bounded_count(data: &[u8], min: u16, max: u16) -> u16 {
    let Some(&byte) = data.first() else {
        return min;
    };
    u16::from(byte)
        .checked_rem(max.saturating_sub(min))
        .map_or(min, |offset| min.saturating_add(offset))
}

fn build_events(run: RunId, count: u16) -> Vec<JournalEvent> {
    build_events_from(run, 0, count)
}

fn build_events_from(run: RunId, start_seq: u64, count: u16) -> Vec<JournalEvent> {
    (0..count)
        .map(|step| JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(start_seq.saturating_add(u64::from(step))),
            step: StepIdx::new(step),
            attempt: 1,
        })
        .collect()
}
