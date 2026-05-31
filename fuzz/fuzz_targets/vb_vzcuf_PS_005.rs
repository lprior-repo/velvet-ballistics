// Cargo-fuzz target for encoded byte accounting (PS-005, C2).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn fuzz_encoded_length_min(data: &[u8]) {
    use vb_storage::codec::encode_record;
    use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN};
    use vb_storage::records::RecordKind;
    use vb_storage::events::JournalEvent;
    use vb_storage::types::EventSeq;
    use vb_core::{RunId, WorkflowDigest};
    if data.len() < 8 { return; }
    let run = u64::from_le_bytes(data[0..8].try_into().unwrap());
    if run == 0 { return; }
    let event = JournalEvent::RunAccepted { run: RunId::new(run), seq: EventSeq::new(0), workflow: WorkflowDigest::from_bytes([0u8; 32]) };
    match encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
        Ok(value) => { assert!(value.len() >= RECORD_HEADER_LEN as usize); assert!(value.len() > RECORD_HEADER_LEN as usize); }
        Err(_) => {}
    }
}

fn fuzz_payload_vs_encoded(data: &[u8]) {
    use vb_storage::codec::encode_record;
    use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN};
    use vb_storage::records::RecordKind;
    use vb_storage::events::JournalEvent;
    use vb_storage::types::EventSeq;
    use vb_core::{RunId, WorkflowDigest};
    if data.len() < 8 { return; }
    let run = u64::from_le_bytes(data[0..8].try_into().unwrap());
    if run == 0 { return; }
    let event = JournalEvent::RunAccepted { run: RunId::new(run), seq: EventSeq::new(0), workflow: WorkflowDigest::from_bytes([0u8; 32]) };
    match encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES) {
        Ok(value) => {
            if let Ok(payload_only) = postcard::to_allocvec(&event) {
                assert!(value.len() > payload_only.len());
                assert_eq!(value.len() - payload_only.len(), RECORD_HEADER_LEN as usize);
            }
        }
        Err(_) => {}
    }
}

fn fuzz_max_encoded_bounds(_data: &[u8]) {
    use vb_storage::constants::{MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN};
    let max_encoded = RECORD_HEADER_LEN as u64 + MAX_JOURNAL_EVENT_PAYLOAD_BYTES as u64;
    assert!(max_encoded < u64::MAX);
    assert_eq!(max_encoded, 1_048_636u64);
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() { return; }
    match data[0] % 3 {
        0 => fuzz_encoded_length_min(&data[1..]),
        1 => fuzz_payload_vs_encoded(&data[1..]),
        _ => fuzz_max_encoded_bounds(&data[1..]),
    }
});
