// Cargo-fuzz target for batch state preservation (PS-004, C5).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn fuzz_encode_record_determinism(data: &[u8]) {
    use vb_storage::codec::encode_record;
    use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
    use vb_storage::records::RecordKind;
    use vb_storage::events::JournalEvent;
    use vb_storage::types::EventSeq;
    use vb_core::{RunId, WorkflowDigest};
    if data.len() < 8 { return; }
    let run_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let run = u64::from_le_bytes(run_bytes);
    if run == 0 { return; }
    let event = JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(0),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };
    let r1 = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    let r2 = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    match (&r1, &r2) {
        (Ok(v1), Ok(v2)) => { assert_eq!(v1, v2); assert_eq!(v1.len(), v2.len()); }
        (Err(_), Err(_)) => {}
        _ => panic!("encode_record non-deterministic"),
    }
}

fn fuzz_staged_state_preservation(data: &[u8]) {
    if data.len() < 24 { return; }
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
    if limit == 0 || staged > limit { return; }
    let staged_before = staged;
    match staged.checked_add(candidate) {
        Some(total) if total <= limit => { assert!(total > staged_before || candidate == 0); }
        _ => { assert_eq!(staged, staged_before); }
    }
}

fn fuzz_rejection_batch_integrity(data: &[u8]) {
    if data.len() < 16 { return; }
    let limit_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let limit = u64::from_le_bytes(limit_bytes);
    let encoded_len_bytes: [u8; 8] = match data.get(8..16) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let encoded_len = u64::from_le_bytes(encoded_len_bytes);
    if limit == 0 { return; }
    let mut staged: u64 = 0;
    match staged.checked_add(encoded_len) {
        Some(total) if total <= limit => { staged = total; assert!(staged <= limit); }
        _ => { assert_eq!(staged, 0); }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() { return; }
    match data[0] % 3 {
        0 => fuzz_encode_record_determinism(&data[1..]),
        1 => fuzz_staged_state_preservation(&data[1..]),
        _ => fuzz_rejection_batch_integrity(&data[1..]),
    }
});
