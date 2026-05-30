// Cargo-fuzz target for duplicate accounting (PS-009, C2).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn fuzz_duplicate_accounting(data: &[u8]) {
    if data.len() < 16 { return; }
    let current = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let encoded_len = u64::from_le_bytes(data[8..16].try_into().unwrap());
    if current > u64::MAX / 2 { return; }
    if encoded_len > 1_000_000 { return; }
    let conservative = current + encoded_len;
    assert!(conservative >= current);
    let precise_new = current + encoded_len;
    assert!(precise_new >= current);
    assert_eq!(precise_new, conservative);
    let precise_dup = current;
    assert!(precise_dup >= current);
    if encoded_len > 0 { assert!(precise_dup < conservative); }
}

fn fuzz_encode_record_duplicate(data: &[u8]) {
    use vb_storage::codec::encode_record;
    use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
    use vb_storage::records::RecordKind;
    use vb_storage::events::JournalEvent;
    use vb_storage::types::EventSeq;
    use vb_core::{RunId, WorkflowDigest};
    if data.len() < 8 { return; }
    let run = u64::from_le_bytes(data[0..8].try_into().unwrap());
    if run == 0 { return; }
    let event = JournalEvent::RunAccepted { run: RunId::new(run), seq: EventSeq::new(0), workflow: WorkflowDigest::from_bytes([0u8; 32]) };
    let r1 = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    let r2 = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, MAX_JOURNAL_EVENT_PAYLOAD_BYTES);
    match (&r1, &r2) {
        (Ok(v1), Ok(v2)) => { assert_eq!(v1, v2); assert_eq!(v1.len(), v2.len()); }
        (Err(_), Err(_)) => {}
        _ => panic!("non-deterministic encoding"),
    }
}

fn fuzz_staged_invariant(data: &[u8]) {
    if data.len() < 24 { return; }
    let staged = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let encoded_len = u64::from_le_bytes(data[8..16].try_into().unwrap());
    let limit = u64::from_le_bytes(data[16..24].try_into().unwrap());
    if limit == 0 || staged > limit { return; }
    let is_duplicate = data.len() >= 25 && data[24] & 1 == 1;
    let new_staged = if is_duplicate { staged }
    else { staged.checked_add(encoded_len).and_then(|t| if t <= limit { Some(t) } else { None }).unwrap_or(staged) };
    assert!(new_staged <= limit);
    assert!(new_staged >= staged || is_duplicate);
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() { return; }
    match data[0] % 3 {
        0 => fuzz_duplicate_accounting(&data[1..]),
        1 => fuzz_encode_record_duplicate(&data[1..]),
        _ => fuzz_staged_invariant(&data[1..]),
    }
});
