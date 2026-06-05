// Cargo-fuzz target for overflow safety (PS-002, C7).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn fuzz_checked_add(data: &[u8]) {
    if data.len() < 16 {
        return;
    }
    let a = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let b = u64::from_le_bytes(data[8..16].try_into().unwrap());
    match a.checked_add(b) {
        Some(total) => {
            assert_eq!(total, a.wrapping_add(b));
        }
        None => {
            assert!(a as u128 + b as u128 > u64::MAX as u128);
        }
    }
}

fn fuzz_u32_widen(data: &[u8]) {
    if data.len() < 4 {
        return;
    }
    let n = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let wide: u64 = n as u64;
    assert_eq!(wide as u32, n);
    assert!(wide <= u32::MAX as u64);
}

fn fuzz_encode_record(data: &[u8]) {
    use vb_core::{RunId, WorkflowDigest};
    use vb_storage::codec::encode_record;
    use vb_storage::constants::{MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES};
    use vb_storage::events::JournalEvent;
    use vb_storage::records::RecordKind;
    use vb_storage::types::EventSeq;
    if data.len() < 16 {
        return;
    }
    let run = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let seq = u64::from_le_bytes(data[8..16].try_into().unwrap());
    if run == 0 {
        return;
    }
    let event = JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(seq % 100_000),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };
    let _ = encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        seq % 100_000,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    match data[0] % 3 {
        0 => fuzz_checked_add(&data[1..]),
        1 => fuzz_u32_widen(&data[1..]),
        _ => fuzz_encode_record(&data[1..]),
    }
});
