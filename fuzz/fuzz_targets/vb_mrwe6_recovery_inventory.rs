#![no_main]

//! Fuzz target for obl-vb-in8ib-recovery-fuzz.
//!
//! Exercises arbitrary persisted scheduled/resolution journal bytes through the
//! production decoder and MRWE6 recovery seam. Pending recovery must be derived
//! only from decoded scheduled facts plus marker presence; fallback/parity cases
//! must not be hidden as pending closure.

use libfuzzer_sys::fuzz_target;
use vb_storage::mrwe6_seams::{Mrwe6RecoveryOutcome, mrwe6_recovery_outcome};

fuzz_target!(|data: &[u8]| {
    let Some((&selector, remaining)) = data.split_first() else {
        return;
    };
    let midpoint = remaining.len() / 2;
    let (scheduled_bytes, resolution_bytes) = remaining.split_at(midpoint);

    let scheduled = decode_journal_event(scheduled_bytes);
    let resolution = decode_journal_event(resolution_bytes);
    let marker_present = selector & 1 != 0;
    let legacy_profile = selector & 2 != 0;

    if let Ok(scheduled_event) = scheduled {
        let outcome = mrwe6_recovery_outcome(
            &scheduled_event,
            resolution.as_ref().ok(),
            marker_present,
            legacy_profile,
        );
        if let Ok(Mrwe6RecoveryOutcome::PendingInventory) = outcome {
            assert!(marker_present);
            assert!(resolution.is_err());
        }
    }
});

fn decode_journal_event(data: &[u8]) -> Result<vb_storage::JournalEvent, vb_storage::JournalError> {
    vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    )
    .map(|(_, event)| event)
}
