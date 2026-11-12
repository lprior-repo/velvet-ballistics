// Cargo-fuzz target for overflow safety (PS-002, C7).
#![no_main]
use libfuzzer_sys::fuzz_target;

fn fuzz_checked_add(data: &[u8]) {
    if data.len() < 16 {
        return;
    }
    let a_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let a = u64::from_le_bytes(a_bytes);
    let b_bytes: [u8; 8] = match data.get(8..16) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let b = u64::from_le_bytes(b_bytes);
    match a.checked_add(b) {
        Some(total) => {
            assert_eq!(total, a.wrapping_add(b));
        }
        None => {
            let wide_sum = u128::from(a).saturating_add(u128::from(b));
            assert!(wide_sum > u128::from(u64::MAX));
        }
    }
}

fn fuzz_u32_widen(data: &[u8]) {
    if data.len() < 4 {
        return;
    }
    let n_bytes: [u8; 4] = match data.get(0..4) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let n = u32::from_le_bytes(n_bytes);
    let wide = u64::from(n);
    assert_eq!(u32::try_from(wide), Ok(n));
    assert!(wide <= u64::from(u32::MAX));
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
    let run_bytes: [u8; 8] = match data.get(0..8) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let run = u64::from_le_bytes(run_bytes);
    let seq_bytes: [u8; 8] = match data.get(8..16) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let seq = u64::from_le_bytes(seq_bytes);
    if run == 0 {
        return;
    }
    let Some(seq_mod) = seq.checked_rem(100_000) else {
        return;
    };
    let event = JournalEvent::RunAccepted {
        run: RunId::new(run),
        seq: EventSeq::new(seq_mod),
        workflow: WorkflowDigest::from_bytes([0u8; 32]),
    };
    match encode_record(
        MAGIC_JOURNAL_EVENT,
        RecordKind::RunAccepted,
        seq_mod,
        &event,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    ) {
        Ok(_) | Err(_) => {}
    }
}

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };
    match selector.checked_rem(3) {
        Some(0) => fuzz_checked_add(rest),
        Some(1) => fuzz_u32_widen(rest),
        Some(_) => fuzz_encode_record(rest),
        None => {}
    }
});
