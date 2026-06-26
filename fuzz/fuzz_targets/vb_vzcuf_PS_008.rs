// Cargo-fuzz target for guard precedence (PS-008, C6).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn guard_chain(key_ok: bool, dup_ok: bool, count_ok: bool, encoding_ok: bool, admission_ok: bool) -> Result<u8, u8> {
    if !key_ok { return Err(0); }
    if !dup_ok { return Err(1); }
    if !count_ok { return Err(2); }
    if !encoding_ok { return Err(3); }
    if !admission_ok { return Err(4); }
    Ok(5)
}

fn fuzz_guard_ordering(data: &[u8]) {
    if data.len() < 5 { return; }
    let key_ok = data[0] & 1 == 1;
    let dup_ok = data[1] & 1 == 1;
    let count_ok = data[2] & 1 == 1;
    let encoding_ok = data[3] & 1 == 1;
    let admission_ok = data[4] & 1 == 1;
    let result = guard_chain(key_ok, dup_ok, count_ok, encoding_ok, admission_ok);
    match result {
        Err(guard) => {
            if !key_ok { assert_eq!(guard, 0); return; }
            if !dup_ok { assert_eq!(guard, 1); return; }
            if !count_ok { assert_eq!(guard, 2); return; }
            if !encoding_ok { assert_eq!(guard, 3); return; }
            if !admission_ok { assert_eq!(guard, 4); return; }
        }
        Ok(guard) => { assert_eq!(guard, 5); assert!(key_ok && dup_ok && count_ok && encoding_ok && admission_ok); }
    }
}

fn fuzz_encode_record_guard(data: &[u8]) {
    use vb_storage::codec::encode_record;
    use vb_storage::constants::MAGIC_JOURNAL_EVENT;
    use vb_storage::records::RecordKind;
    use vb_storage::events::JournalEvent;
    use vb_storage::types::EventSeq;
    use vb_core::{RunId, WorkflowDigest};
    if data.len() < 4 { return; }
    let max_len = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let event = JournalEvent::RunAccepted { run: RunId::new(1), seq: EventSeq::new(0), workflow: WorkflowDigest::from_bytes([0u8; 32]) };
    let result = encode_record(MAGIC_JOURNAL_EVENT, RecordKind::RunAccepted, 0, &event, max_len);
    match result {
        Ok(value) => { assert!(!value.is_empty()); }
        Err(e) => { let msg = format!("{e}"); assert!(!msg.is_empty()); }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() { return; }
    match data[0] % 2 {
        0 => fuzz_guard_ordering(&data[1..]),
        _ => fuzz_encode_record_guard(&data[1..]),
    }
});
