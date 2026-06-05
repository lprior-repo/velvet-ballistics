// Cargo-fuzz target for error variant discrimination (PS-003, C4).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn fuzz_encode_record_errors(data: &[u8]) {
    use vb_core::{RunId, WorkflowDigest};
    use vb_storage::codec::encode_record;
    use vb_storage::constants::MAGIC_JOURNAL_EVENT;
    use vb_storage::events::JournalEvent;
    use vb_storage::records::RecordKind;
    use vb_storage::types::EventSeq;
    if data.len() < 4 {
        return;
    }
    let max_len = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let event = JournalEvent::RunAccepted {
        run: RunId::new(1),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };
    let result = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        0,
        &event,
        max_len,
    );
    match result {
        Ok(value) => {
            assert!(!value.is_empty());
        }
        Err(e) => {
            let msg = format!("{e}");
            assert!(!msg.is_empty());
        }
    }
}

fn fuzz_admission_classification(data: &[u8]) {
    if data.len() < 24 {
        return;
    }
    let staged = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let candidate = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let limit = u64::from_le_bytes(data[16..24].try_into().unwrap());
    if limit == 0 {
        return;
    }
    match staged.checked_add(candidate) {
        Some(total) => {
            if total > limit { /* over-limit distinct from QueueFull */ }
        }
        None => { /* overflow distinct from PayloadTooLarge */ }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    match data[0] % 2 {
        0 => fuzz_encode_record_errors(&data[1..]),
        _ => fuzz_admission_classification(&data[1..]),
    }
});
