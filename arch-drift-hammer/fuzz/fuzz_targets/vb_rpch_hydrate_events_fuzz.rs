#![no_main]
#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use vb_core::{RunId, SlotIdx};
use vb_storage::recovery::hydrate::hydrate_events_preconditions;
use vb_storage::{EventSeq, JournalEvent};

fuzz_target!(|data: &[u8]| {
    let events = if data.is_empty() {
        Vec::new()
    } else {
        vec![JournalEvent::RunFinished {
            run: RunId::new(1),
            seq: EventSeq::new(1),
            result: SlotIdx::new(0),
            attempt: 1,
        }]
    };
    assert_eq!(hydrate_events_preconditions(&events), !data.is_empty());
});
