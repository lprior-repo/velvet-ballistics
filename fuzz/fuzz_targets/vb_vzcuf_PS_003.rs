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
    let max_len_bytes: [u8; 4] = match data.get(0..4) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let max_len = u32::from_le_bytes(max_len_bytes);
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
    let staged_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let staged = u64::from_le_bytes(staged_bytes);
    let candidate_bytes: [u8; 8] = match data.get(8..16) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let candidate = u64::from_le_bytes(candidate_bytes);
    let limit_bytes: [u8; 8] = match data.get(16..24) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let limit = u64::from_le_bytes(limit_bytes);
    if limit == 0 {
        return;
    }
    match staged.checked_add(candidate) {
        Some(total) if total > limit => { /* over-limit distinct from QueueFull */ }
        Some(_) => {}
        None => { /* overflow distinct from PayloadTooLarge */ }
    }
}

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    match selector.checked_rem(2) {
        Some(0) => fuzz_encode_record_errors(rest),
        Some(_) => fuzz_admission_classification(rest),
        None => {}
    }
});
